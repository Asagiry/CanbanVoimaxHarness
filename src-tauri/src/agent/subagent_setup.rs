use tauri::Emitter;
use crate::agent::compactor;
use crate::agent::prompts::{is_tool_active_for_agent, strip_yaml_frontmatter};
use crate::agent::spill::get_all_tools_for_role;
use crate::agent::subagent_runtime::{canonical_tool_name, generate_subagent_suffix, OwnedToolContext};
use crate::config::load_agent_setting;
use crate::db::ActivityItem;
use crate::llm::{ChatMessage, ToolDefinition};

pub struct SubagentPreparedContext {
    pub parent_id: String,
    pub subagent_suffix: String,
    pub subagent_id: String,
    pub subagent_dialog_id: String,
    pub tool_call_id: Option<String>,
    pub effective_sub_tools: Vec<String>,
    pub active_sub_tool_defs: Vec<ToolDefinition>,
    pub subagent_setting: crate::config::AgentSetting,
    pub sub_messages: Vec<ChatMessage>,
    pub resolved_model: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub reasoning_effort: Option<String>,
    pub system_tokens: u64,
}

pub async fn prepare_subagent_execution(
    ctx: &OwnedToolContext,
    title: &str,
    role: &str,
    prompt: &str,
    requested_tools: Option<&[String]>,
    explicit_subagent_id: Option<&str>,
) -> SubagentPreparedContext {
    let parent_id = ctx.task_id.as_deref().unwrap_or("unknown").to_string();
    let subagent_suffix = match explicit_subagent_id {
        Some(eid) => eid
            .rsplit(".sub-")
            .next()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| generate_subagent_suffix(ctx.call_id.as_deref())),
        None => generate_subagent_suffix(ctx.call_id.as_deref()),
    };
    let is_bg_subagent = explicit_subagent_id.is_some();
    let raw_sub_id = explicit_subagent_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}.sub-{}", parent_id, subagent_suffix));
    let (subagent_id, subagent_dialog_id) = if raw_sub_id.contains(":sub:") {
        let after = raw_sub_id.rsplit(":sub:").next().unwrap_or(&raw_sub_id).to_string();
        (after, raw_sub_id)
    } else {
        let dialog_id = format!("{}:sub:{}", parent_id, raw_sub_id);
        (raw_sub_id, dialog_id)
    };
    let tool_call_id = ctx
        .call_id
        .clone()
        .or_else(|| subagent_id.rfind(".sub-").map(|pos| subagent_id[(pos + 5)..].to_string()));

    // 1. Tool filtering
    let subagent_setting = load_agent_setting(&ctx.repo_root, role).await;
    let all_tools = get_all_tools_for_role(&ctx.repo_root, Some(role)).await;
    let base_tools: Vec<String> = match requested_tools {
        Some(t) if !t.is_empty() => t.to_vec(),
        _ => all_tools.iter().map(|t| t.function.name.clone()).collect(),
    };

    let mut effective_sub_tools: Vec<String> = base_tools
        .into_iter()
        .filter(|t| {
            !matches!(
                canonical_tool_name(t),
                "subagent"
                    | "subagent_bg"
                    | "collect_subagent"
                    | "subagent_list"
                    | "advance_task"
                    | "reject_task"
                    | "create_task"
                    | "complete_task"
            ) && is_tool_active_for_agent(
                t,
                &ctx.tool_policy,
                role,
                ctx.stage_id.as_deref(),
                subagent_setting.allowed_tools.as_deref(),
            )
        })
        .collect();
    effective_sub_tools.sort();
    effective_sub_tools.dedup();

    // 2. Emit subagent:spawned
    if let Some(ref app) = ctx.app_handle {
        let _ = app.emit(
            "subagent:spawned",
            serde_json::json!({
                "parentTaskId": &parent_id,
                "subagentId": &subagent_id,
                "toolCallId": &tool_call_id,
                "title": title,
                "persona": role,
                "role": role,
                "prompt": prompt,
                "tools": &effective_sub_tools,
                "createdAt": chrono::Utc::now().timestamp_millis(),
            }),
        );
    }

    // 3. Persist top instruction prompt into SQLite dialog
    let prompt_item = ActivityItem {
        id: format!("sub-p-{}", subagent_suffix),
        item_type: "prompt".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        content: Some(prompt.to_string()),
        tool: Some(serde_json::json!({
            "title": title,
            "persona": role,
            "role": role,
            "is_bg": is_bg_subagent
        })),
        duration_ms: None,
        images: None,
    };
    if let Err(e) = crate::db::append_dialog_items(
        &ctx.repo_root,
        &subagent_dialog_id,
        std::slice::from_ref(&prompt_item),
    ) {
        eprintln!("CVH [WARN]: Failed to append subagent prompt dialog item: {}", e);
    }

    // 4. Resolve schemas for allowed subagent tools
    let all_tools = get_all_tools_for_role(&ctx.repo_root, Some(role)).await;
    let mut active_sub_tool_defs: Vec<ToolDefinition> = all_tools
        .into_iter()
        .filter(|t| effective_sub_tools.contains(&t.function.name))
        .collect();
    active_sub_tool_defs.sort_by(|a, b| a.function.name.cmp(&b.function.name));

    // 5. System persona & initial messages
    let persona_content = {
        let persona_path = crate::config::agent_persona_path(&ctx.repo_root, role);
        if persona_path.is_file() {
            tokio::fs::read_to_string(&persona_path).await.ok()
        } else {
            None
        }
    };

    let system_prompt = format!(
        "# Role Persona: {}\n{}\n\n# Scoped Subagent Mandate\nYou are an in-process subagent executing an atomic subtask.\nAllowed Tools: {:?}\nInvariants:\n- Do not attempt to move kanban stages or spawn nested subagents.\n- Working Directory Invariant: All file tools and execute_command ALREADY run inside the working directory. NEVER prepend 'cd /d <path> &&' or 'cd <path> &&' to execute_command.\n- Search Invariant: Prefer native 'grep', 'glob', and 'read_file' tools over shell 'findstr' or 'grep' via execute_command.\n- Single-Action Collection: When invoking 'collect_subagent', you MUST invoke it as the ONLY tool call in that turn. Do NOT batch file reads or other tools alongside collect_subagent; wait to consume and analyze the received reports before planning next steps.\n- Execute the task thoroughly and verify results.\n- When done, output your comprehensive final report.",
        role,
        persona_content.as_deref().map(strip_yaml_frontmatter).unwrap_or("You are a specialized autonomous subagent."),
        effective_sub_tools
    );

    let initial_user_prompt = format!(
        "# Context\nWorking directory: {} (already active for all file tools and execute_command — do NOT prepend 'cd /d')\nTask Title: {}\n\n# Objective: {}\n\n{}",
        ctx.worktree_dir.to_string_lossy(),
        title,
        title,
        prompt
    );

    let system_tokens = compactor::estimate_text_tokens(&system_prompt) as u64;
    let sub_messages = vec![
        ChatMessage::system(system_prompt).with_cache_control(crate::llm::CacheControl::ephemeral()),
        ChatMessage::user(initial_user_prompt).with_cache_control(crate::llm::CacheControl::ephemeral()),
    ];

    let base_url = ctx.base_url.as_deref().unwrap_or("https://opencode.ai/inference/go/openai/v1").to_string();
    let resolved_model = match ctx
        .model
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
    {
        Some(m) => m.to_string(),
        None => {
            let global_settings = crate::config::load_llm_settings(&ctx.repo_root)
                .await
                .unwrap_or_default();
            let m = global_settings.model.trim();
            if !m.is_empty() {
                m.to_string()
            } else {
                global_settings
                    .providers
                    .iter()
                    .flat_map(|p| p.models.iter())
                    .next()
                    .map(|m| m.id.clone())
                    .unwrap_or_default()
            }
        }
    };
    let api_key = ctx.api_key.clone();
    let reasoning_effort = crate::config::normalize_reasoning_effort(
        subagent_setting.reasoning_effort.as_deref(),
    )
    .or_else(|| ctx.reasoning_effort.clone());

    SubagentPreparedContext {
        parent_id,
        subagent_suffix,
        subagent_id,
        subagent_dialog_id,
        tool_call_id,
        effective_sub_tools,
        active_sub_tool_defs,
        subagent_setting,
        sub_messages,
        resolved_model,
        base_url,
        api_key,
        reasoning_effort,
        system_tokens,
    }
}
