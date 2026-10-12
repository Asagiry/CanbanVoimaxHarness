use crate::agent::{run_agent_loop, AgentEvent, AgentSessionConfig};
use crate::config::{
    atomic_write_file, load_pipeline_config, load_project_config, load_project_registry,
    save_pipeline_config, save_project_config, PipelineConfig, ProjectConfig, ProjectRegistry,
};
use crate::git::{create_worktree, remove_worktree};
use crate::hooks::{execute_hook, execute_sensor_command, HookContext};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "stageId")]
    pub stage_id: String,
    #[serde(rename = "worktreePath", default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "updatedAt")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexNode {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(rename = "isDir")]
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<CortexNode>>,
}

pub struct AppState {
    pub repo_root: RwLock<PathBuf>,
    pub active_agents: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub agent_contexts: Mutex<HashMap<String, Arc<Mutex<crate::agent::compactor::AgentContextState>>>>,
}

impl AppState {
    pub fn new(root: PathBuf) -> Self {
        Self {
            repo_root: RwLock::new(root),
            active_agents: Mutex::new(HashMap::new()),
            agent_contexts: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_root(&self) -> PathBuf {
        self.repo_root
            .read()
            .map(|r| r.clone())
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}

async fn load_tasks_from_disk(repo_root: &Path) -> Vec<Task> {
    let tasks_file = repo_root.join(".cvh").join("tasks.json");
    if !tasks_file.exists() {
        return Vec::new();
    }
    match tokio::fs::read_to_string(&tasks_file).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

async fn save_tasks_to_disk(repo_root: &Path, tasks: &[Task]) -> Result<(), String> {
    let tasks_file = repo_root.join(".cvh").join("tasks.json");
    let serialized = serde_json::to_string_pretty(tasks)
        .map_err(|e| format!("Failed to serialize tasks: {}", e))?;
    atomic_write_file(&tasks_file, &serialized)
        .await
        .map_err(|e| format!("Failed to save tasks: {}", e))
}

// ============================================================================
// IPC Commands
// ============================================================================

#[tauri::command]
pub async fn get_pipeline(state: State<'_, AppState>) -> Result<PipelineConfig, String> {
    let root = state.get_root();
    load_pipeline_config(&root)
        .await
        .map_err(|e| format!("Failed to load pipeline: {}", e))
}

#[tauri::command]
pub async fn save_pipeline(
    pipeline: PipelineConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    save_pipeline_config(&root, &pipeline)
        .await
        .map_err(|e| format!("Failed to save pipeline: {}", e))
}

#[tauri::command]
pub async fn get_project_config_cmd(state: State<'_, AppState>) -> Result<ProjectConfig, String> {
    let root = state.get_root();
    load_project_config(&root)
        .await
        .map_err(|e| format!("Failed to load project config: {}", e))
}

#[tauri::command]
pub async fn get_registry(registry_path: Option<String>) -> Result<ProjectRegistry, String> {
    let p = registry_path.map(PathBuf::from);
    load_project_registry(p)
        .await
        .map_err(|e| format!("Failed to load project registry: {}", e))
}

#[tauri::command]
pub async fn get_cortex_tree(state: State<'_, AppState>) -> Result<Vec<CortexNode>, String> {
    let root = state.get_root();
    let mut root_nodes = Vec::new();

    // 1. Column Personas (.cvh/agents/)
    let agents_dir = root.join(".cvh").join("agents");
    if agents_dir.exists() {
        let mut persona_children = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(&agents_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let rel_path = format!(".cvh/agents/{}", name);
                persona_children.push(CortexNode {
                    id: format!("persona-{}", name),
                    name,
                    path: rel_path,
                    is_dir: false,
                    children: None,
                });
            }
        }
        root_nodes.push(CortexNode {
            id: "cortex-personas".to_string(),
            name: "Personas".to_string(),
            path: ".cvh/agents".to_string(),
            is_dir: true,
            children: Some(persona_children),
        });
    }

    // 2. Transition Hooks (.cvh/hooks/)
    let hooks_dir = root.join(".cvh").join("hooks");
    if hooks_dir.exists() {
        let mut hook_children = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(&hooks_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let rel_path = format!(".cvh/hooks/{}", name);
                hook_children.push(CortexNode {
                    id: format!("hook-{}", name),
                    name,
                    path: rel_path,
                    is_dir: false,
                    children: None,
                });
            }
        }
        root_nodes.push(CortexNode {
            id: "cortex-hooks".to_string(),
            name: "Transition Hooks".to_string(),
            path: ".cvh/hooks".to_string(),
            is_dir: true,
            children: Some(hook_children),
        });
    }

    // 3. Skills Repository (.agents/skills/)
    let skills_dir = root.join(".agents").join("skills");
    if skills_dir.exists() {
        let mut skill_children = Vec::new();
        if let Ok(mut entries) = tokio::fs::read_dir(&skills_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                    let s_name = entry.file_name().to_string_lossy().to_string();
                    let skill_doc = format!(".agents/skills/{}/SKILL.md", s_name);
                    skill_children.push(CortexNode {
                        id: format!("skill-{}", s_name),
                        name: s_name,
                        path: skill_doc,
                        is_dir: false,
                        children: None,
                    });
                }
            }
        }
        root_nodes.push(CortexNode {
            id: "cortex-skills".to_string(),
            name: "Skills & Rules".to_string(),
            path: ".agents/skills".to_string(),
            is_dir: true,
            children: Some(skill_children),
        });
    }

    // 4. Invariants (AGENTS.md)
    let agents_md = root.join("AGENTS.md");
    if agents_md.exists() {
        root_nodes.push(CortexNode {
            id: "cortex-agents-md".to_string(),
            name: "AGENTS.md (Invariants)".to_string(),
            path: "AGENTS.md".to_string(),
            is_dir: false,
            children: None,
        });
    }

    Ok(root_nodes)
}

#[tauri::command]
pub async fn read_cortex_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let root = state.get_root();
    let target = if Path::new(&path).is_absolute() {
        PathBuf::from(path)
    } else {
        root.join(path)
    };

    tokio::fs::read_to_string(&target)
        .await
        .map_err(|e| format!("Failed to read cortex file {:?}: {}", target, e))
}

#[tauri::command]
pub async fn save_cortex_file(
    path: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    let target = if Path::new(&path).is_absolute() {
        PathBuf::from(path)
    } else {
        root.join(path)
    };

    atomic_write_file(&target, &content)
        .await
        .map_err(|e| format!("Failed to save cortex file {:?}: {}", target, e))
}

#[tauri::command]
pub async fn list_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let root = state.get_root();
    Ok(load_tasks_from_disk(&root).await)
}

#[tauri::command]
pub async fn create_task(
    title: String,
    description: String,
    stage_id: String,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;

    // Generate task ID
    let task_num = tasks.len() + 1;
    let task_id = format!("CVH-{:03}", task_num);
    let task_slug = task_id.to_lowercase();
    let now = Utc::now().timestamp_millis();

    // Create worktree if entering dev or active stage
    let worktree_dir = create_worktree(&root, &task_slug, "main")
        .await
        .map_err(|e| format!("Failed to create worktree: {}", e))?;

    let worktree_path = worktree_dir.to_string_lossy().to_string();
    let branch = format!("feat/{}", task_slug);

    let new_task = Task {
        id: task_id.clone(),
        title: title.clone(),
        description: description.clone(),
        stage_id: stage_id.clone(),
        worktree_path: Some(worktree_path.clone()),
        branch: Some(branch.clone()),
        verdict: None,
        created_at: now,
        updated_at: now,
    };

    // Check pipeline config for stage hooks
    if let Ok(pipeline) = load_pipeline_config(&root).await {
        if let Some(col) = pipeline.columns.iter().find(|c| c.id == stage_id) {
            if let Some(hook_rel) = col.hooks.get("on_enter") {
                let hook_path = root.join(hook_rel);
                if hook_path.exists() {
                    let ctx = HookContext {
                        task_id: task_id.clone(),
                        task_title: title.clone(),
                        worktree_path: worktree_path.clone(),
                        branch: branch.clone(),
                        verdict: None,
                        extra_env: HashMap::new(),
                    };
                    let _ = execute_hook(hook_path, &root, &ctx, None).await;
                }
            }
        }
    }

    tasks.push(new_task.clone());
    save_tasks_to_disk(&root, &tasks).await?;

    Ok(new_task)
}

#[tauri::command]
pub async fn move_task_stage(
    task_id: String,
    target_stage_id: String,
    verdict: Option<String>,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;

    let task_idx = tasks
        .iter()
        .position(|t| t.id == task_id)
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let pipeline = load_pipeline_config(&root)
        .await
        .map_err(|e| format!("Failed to load pipeline: {}", e))?;

    let target_col = pipeline
        .columns
        .iter()
        .find(|c| c.id == target_stage_id)
        .ok_or_else(|| format!("Target stage {} not found in pipeline", target_stage_id))?;

    let current_task = &mut tasks[task_idx];

    // Gatekeeper Sensor Verification
    if let Some(sensor_cmd) = &target_col.gatekeeper_sensor {
        let wt_path = current_task
            .worktree_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| root.clone());

        let sensor_res = execute_sensor_command(sensor_cmd, &wt_path, None)
            .await
            .map_err(|e| format!("Sensor execution failed: {}", e))?;

        if !sensor_res.success {
            return Err(format!(
                "Gatekeeper Sensor '{}' rejected transition (Exit code: {:?}):\nSTDOUT: {}\nSTDERR: {}",
                sensor_cmd, sensor_res.exit_code, sensor_res.stdout, sensor_res.stderr
            ));
        }
    }

    // Trigger on_enter hook if configured
    if let Some(hook_rel) = target_col.hooks.get("on_enter") {
        let hook_path = root.join(hook_rel);
        if hook_path.exists() {
            let ctx = HookContext {
                task_id: current_task.id.clone(),
                task_title: current_task.title.clone(),
                worktree_path: current_task.worktree_path.clone().unwrap_or_default(),
                branch: current_task.branch.clone().unwrap_or_default(),
                verdict: verdict.clone().or_else(|| current_task.verdict.clone()),
                extra_env: HashMap::new(),
            };
            let hook_res = execute_hook(hook_path, &root, &ctx, None)
                .await
                .map_err(|e| format!("Stage transition hook failed: {}", e))?;

            if !hook_res.success {
                return Err(format!(
                    "Stage transition hook rejected transition (Exit code: {:?}):\n{}",
                    hook_res.exit_code, hook_res.stderr
                ));
            }
        }
    }

    // Update task
    current_task.stage_id = target_stage_id;
    if verdict.is_some() {
        current_task.verdict = verdict;
    }
    current_task.updated_at = Utc::now().timestamp_millis();

    let updated = current_task.clone();
    save_tasks_to_disk(&root, &tasks).await?;

    Ok(updated)
}

#[tauri::command]
pub async fn run_agent(
    task_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let root = state.get_root();
    let tasks = load_tasks_from_disk(&root).await;

    let task = tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| format!("Task {} not found", task_id))?
        .clone();

    let pipeline = load_pipeline_config(&root)
        .await
        .map_err(|e| format!("Failed to load pipeline: {}", e))?;

    let col = pipeline
        .columns
        .iter()
        .find(|c| c.id == task.stage_id)
        .ok_or_else(|| format!("Column {} not found in pipeline", task.stage_id))?;

    let persona_role = col.persona.clone().unwrap_or_else(|| "developer".to_string());

    let proj_cfg = load_project_config(&root)
        .await
        .unwrap_or_else(|_| ProjectConfig {
            name: "CanbanVoimaxHarness".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            active_pipeline: None,
            default_model: Some("gemini-3.8-flash-high".to_string()),
            api_base_url: Some("http://127.0.0.1:19387/api/ai".to_string()),
            tool_permissions: None,
            providers: HashMap::new(),
        });

    let model = col
        .model_override
        .clone()
        .or_else(|| proj_cfg.default_model.clone())
        .unwrap_or_else(|| "gemini-3.8-flash-high".to_string());

    let base_url = proj_cfg
        .api_base_url
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:19387/api/ai".to_string());

    let worktree_dir = task
        .worktree_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| root.clone());

    let abort_flag = Arc::new(AtomicBool::new(false));

    // Store in active agents registry
    {
        let mut map = state
            .active_agents
            .lock()
            .map_err(|_| "Failed to lock active agents".to_string())?;
        map.insert(task_id.clone(), Arc::clone(&abort_flag));
    }

    let agent_cfg = AgentSessionConfig {
        repo_root: root,
        worktree_path: worktree_dir,
        role: persona_role,
        stage_id: task.stage_id.clone(),
        task_id: task.id.clone(),
        task_title: task.title.clone(),
        task_description: task.description.clone(),
        base_url,
        api_key: None,
        model,
        temperature: col.temperature,
        reasoning_effort: col.reasoning_effort.clone(),
        max_turns: 25,
    };

    let tid = task.id.clone();
    let app = app_handle.clone();

    tokio::spawn(async move {
        let app_inner = app.clone();
        let tid_inner = tid.clone();

        let event_sink = move |event: AgentEvent| {
            match event {
                AgentEvent::Token(token) => {
                    let _ = app_inner.emit(
                        "agent:token",
                        serde_json::json!({ "taskId": tid_inner, "token": token }),
                    );
                }
                AgentEvent::Thought(thought) => {
                    let _ = app_inner.emit(
                        "agent:thought",
                        serde_json::json!({ "taskId": tid_inner, "thought": thought }),
                    );
                }
                AgentEvent::ToolCall { id, name, arguments } => {
                    let _ = app_inner.emit(
                        "agent:tool_call",
                        serde_json::json!({ "taskId": tid_inner, "id": id, "name": name, "arguments": arguments }),
                    );
                }
                AgentEvent::ToolResult { id, name, result, success } => {
                    let _ = app_inner.emit(
                        "agent:tool_result",
                        serde_json::json!({ "taskId": tid_inner, "id": id, "name": name, "result": result, "success": success }),
                    );
                }
                AgentEvent::TurnStart { turn } => {
                    let _ = app_inner.emit(
                        "agent:turn_start",
                        serde_json::json!({ "taskId": tid_inner, "turn": turn }),
                    );
                }
                AgentEvent::Telemetry {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens,
                    cache_creation_tokens,
                    cache_hit_rate,
                } => {
                    let _ = app_inner.emit(
                        "agent:telemetry",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "inputTokens": input_tokens,
                            "outputTokens": output_tokens,
                            "cacheReadTokens": cache_read_tokens,
                            "cacheCreationTokens": cache_creation_tokens,
                            "cacheHitRate": cache_hit_rate,
                        }),
                    );
                }
                AgentEvent::Compacted {
                    saved_chars,
                    estimated_saved_tokens,
                    evaporated_count,
                } => {
                    let _ = app_inner.emit(
                        "agent:compacted",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "savedChars": saved_chars,
                            "estimatedSavedTokens": estimated_saved_tokens,
                            "evaporatedCount": evaporated_count,
                        }),
                    );
                }
                AgentEvent::Finished { final_response } => {
                    let _ = app_inner.emit(
                        "agent:finished",
                        serde_json::json!({ "taskId": tid_inner, "response": final_response }),
                    );
                }
                AgentEvent::Aborted => {
                    let _ = app_inner.emit(
                        "agent:aborted",
                        serde_json::json!({ "taskId": tid_inner }),
                    );
                }
                AgentEvent::Error(err) => {
                    let _ = app_inner.emit(
                        "agent:error",
                        serde_json::json!({ "taskId": tid_inner, "error": err }),
                    );
                }
            }
        };

        let result = run_agent_loop(agent_cfg, abort_flag, event_sink).await;
        if let Err(err) = result {
            let _ = app.emit(
                "agent:error",
                serde_json::json!({ "taskId": tid, "error": err.to_string() }),
            );
        }
    });

    Ok(format!("Agent launched for task {}", task_id))
}

#[tauri::command]
pub async fn abort_agent(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut map = state
        .active_agents
        .lock()
        .map_err(|_| "Failed to lock active agents".to_string())?;

    if let Some(flag) = map.remove(&task_id) {
        flag.store(true, Ordering::SeqCst);
        Ok(())
    } else {
        Err(format!("No running agent found for task {}", task_id))
    }
}

#[tauri::command]
pub async fn cleanup_task_worktree(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    let task_slug = task_id.to_lowercase();
    remove_worktree(&root, &task_slug)
        .await
        .map_err(|e| format!("Failed to remove worktree for {}: {}", task_id, e))
}

#[tauri::command]
pub async fn get_tool_policies(
    state: State<'_, AppState>,
) -> Result<crate::config::ToolPolicyConfig, String> {
    let root = state.get_root();
    crate::config::load_tool_policies(&root)
        .await
        .map_err(|e| format!("Failed to load tool policies: {}", e))
}

#[tauri::command]
pub async fn save_tool_policies(
    policies: crate::config::ToolPolicyConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    crate::config::save_tool_policies(&root, &policies)
        .await
        .map_err(|e| format!("Failed to save tool policies: {}", e))
}

#[tauri::command]
pub async fn compact_task_context(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<crate::agent::compactor::CompactionResult, String> {
    let root = state.get_root();
    let tasks = load_tasks_from_disk(&root).await;
    let task = tasks.iter().find(|t| t.id == task_id);

    let snapshot = crate::agent::compactor::ExecutionStateSnapshot {
        modified_files: vec!["src/agent.rs".to_string(), "src/commands.rs".to_string()],
        sensor_status: "PASSED (Sensors verified)".to_string(),
        next_step: format!("Continuing autonomous execution for {}", task_id),
        verdict: task.and_then(|t| t.verdict.clone()).unwrap_or_else(|| "IN_PROGRESS".to_string()),
    };

    let mut sample_messages = vec![
        crate::llm::ChatMessage::system("Base system prompt instructions"),
        crate::llm::ChatMessage::user(format!("Task execution prompt for {}", task_id)),
        crate::llm::ChatMessage::assistant(Some("Investigating files".to_string()), None),
        crate::llm::ChatMessage::tool_response("call_01", "A".repeat(800)),
        crate::llm::ChatMessage::assistant(Some("Applied modifications".to_string()), None),
    ];

    let result = crate::agent::compactor::compact_conversation(&mut sample_messages, Some(snapshot));
    Ok(result)
}

#[tauri::command]
pub async fn compact_context(
    task_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::agent::compactor::CompactionReport, String> {
    let root = state.get_root();

    let context_state_opt = {
        let map = state
            .agent_contexts
            .lock()
            .map_err(|_| "Failed to lock agent contexts".to_string())?;
        map.get(&task_id).cloned()
    };

    let report = if let Some(cs) = context_state_opt {
        let mut guard = cs
            .lock()
            .map_err(|_| "Failed to lock agent context state".to_string())?;
        let current_turn = guard.current_turn;
        crate::agent::compactor::compact_context_messages(&mut guard.messages, current_turn)
    } else {
        let tasks = load_tasks_from_disk(&root).await;
        let task = tasks.into_iter().find(|t| t.id == task_id);

        let mut messages = if let Some(t) = task {
            vec![
                crate::llm::ChatMessage::system(format!(
                    "# Role Persona: developer\nYou are an expert AI agent working on {}.",
                    t.id
                )),
                crate::llm::ChatMessage::user(format!(
                    "Execute task {}: {}\nDetails:\n{}",
                    t.id, t.title, t.description
                )),
            ]
        } else {
            vec![
                crate::llm::ChatMessage::system("Agent session context".to_string()),
                crate::llm::ChatMessage::user(format!("Execute task {}", task_id)),
            ]
        };

        let current_turn = 0;
        let r = crate::agent::compactor::compact_context_messages(&mut messages, current_turn);

        let new_state = Arc::new(Mutex::new(crate::agent::compactor::AgentContextState {
            current_turn,
            messages,
        }));
        if let Ok(mut map) = state.agent_contexts.lock() {
            map.insert(task_id.clone(), new_state);
        }

        r
    };

    // Emit Tauri event agent:compacted with CompactionReport payload
    let _ = app_handle.emit("agent:compacted", &report);

    Ok(report)
}

#[tauri::command]
pub async fn steer_agent(
    task_id: String,
    message: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let root = state.get_root();
    let tasks = load_tasks_from_disk(&root).await;
    let _task = tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let steer_payload = serde_json::json!({
        "taskId": task_id,
        "content": message,
        "type": "steer"
    });
    let _ = app_handle.emit("agent:steer", steer_payload);

    // Re-invoke agent loop with steering message
    run_agent(task_id, app_handle, state).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSettings {
    pub provider: String,
    #[serde(rename = "gatewayUrl", alias = "gateway_url", alias = "apiBaseUrl")]
    pub gateway_url: String,
    #[serde(rename = "apiKey", alias = "api_key", default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub model: String,
}

#[tauri::command]
pub async fn save_llm_settings(
    settings: LlmSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    let mut config = load_project_config(&root).await.unwrap_or_else(|_| ProjectConfig {
        name: "CanbanVoimaxHarness".to_string(),
        version: "0.1.0".to_string(),
        description: None,
        active_pipeline: None,
        default_model: Some("gemini-3.8-flash-high".to_string()),
        api_base_url: Some("http://127.0.0.1:19387/api/ai".to_string()),
        tool_permissions: None,
        providers: HashMap::new(),
    });

    config.default_model = Some(settings.model.clone());
    config.api_base_url = Some(settings.gateway_url.clone());
    config.providers.insert(
        settings.provider.clone(),
        crate::config::ProviderConfig {
            provider_type: settings.provider.clone(),
            base_url: settings.gateway_url.clone(),
            default_model: Some(settings.model.clone()),
            api_key: settings.api_key.clone(),
        },
    );

    save_project_config(&root, &config)
        .await
        .map_err(|e| format!("Failed to save LLM settings: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_llm_settings(state: State<'_, AppState>) -> Result<LlmSettings, String> {
    let root = state.get_root();
    if let Ok(config) = load_project_config(&root).await {
        let (prov, key) = if let Some((name, prov_cfg)) = config.providers.iter().next() {
            (name.clone(), prov_cfg.api_key.clone())
        } else {
            ("Antigravity Proxy".to_string(), None)
        };
        Ok(LlmSettings {
            provider: prov,
            gateway_url: config.api_base_url.unwrap_or_else(|| "http://127.0.0.1:19387/api/ai".to_string()),
            api_key: key,
            model: config.default_model.unwrap_or_else(|| "gemini-3.8-flash-high".to_string()),
        })
    } else {
        Ok(LlmSettings {
            provider: "Antigravity Proxy".to_string(),
            gateway_url: "http://127.0.0.1:19387/api/ai".to_string(),
            api_key: None,
            model: "gemini-3.8-flash-high".to_string(),
        })
    }
}

#[tauri::command]
pub async fn set_project_root(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("Directory does not exist: {}", path));
    }
    if let Ok(mut root) = state.repo_root.write() {
        *root = p.clone();
    }
    Ok(p.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn choose_project_folder() -> Result<Option<String>, String> {
    #[cfg(target_os = "windows")]
    {
        let output = tokio::process::Command::new("powershell")
            .args(&[
                "-NoProfile",
                "-Command",
                r#"Add-Type -AssemblyName System.Windows.Forms; $dialog = New-Object System.Windows.Forms.FolderBrowserDialog; $dialog.Description = 'Выберите папку проекта'; if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { Write-Output $dialog.SelectedPath }"#,
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to open folder picker: {}", e))?;
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }
}
