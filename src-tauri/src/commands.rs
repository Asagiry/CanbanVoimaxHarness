use crate::agent::{run_agent_loop, AgentEvent, AgentSessionConfig};
pub use crate::config::{discover_project_root, persist_active_project};
use crate::config::{
    atomic_write_file, get_app_config_dir, load_llm_settings, load_pipeline_config,
    load_project_config, load_project_registry, register_or_update_project,
    save_llm_settings_to_disk, save_pipeline_config, scaffold_cvh_project, LlmSettings,
    PipelineConfig, ProjectConfig, ProjectEntry, ProjectRegistry,
};
use crate::git::{create_worktree, remove_worktree};
use crate::hooks::{execute_hook, execute_sensor_command, HookContext};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "stageId", alias = "columnId")]
    pub stage_id: String,
    #[serde(rename = "worktreePath", alias = "worktree_path", default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(rename = "customFlow", alias = "custom_flow", default)]
    pub custom_flow: Option<Vec<String>>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(rename = "createdAt", alias = "created_at", default)]
    pub created_at: i64,
    #[serde(rename = "updatedAt", alias = "updated_at", default)]
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
        let actual_root = if root.join(".cvh").exists() || root.join(".git").exists() {
            root
        } else {
            discover_project_root()
        };
        Self {
            repo_root: RwLock::new(actual_root),
            active_agents: Mutex::new(HashMap::new()),
            agent_contexts: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_root(&self) -> PathBuf {
        self.repo_root
            .read()
            .map(|r| r.clone())
            .unwrap_or_else(|_| discover_project_root())
    }

    pub fn set_root(&self, new_root: PathBuf) -> Result<(), String> {
        let mut guard = self
            .repo_root
            .write()
            .map_err(|e| format!("Failed to acquire write lock on repo_root: {}", e))?;
        *guard = new_root;
        Ok(())
    }
}

async fn load_tasks_from_disk(repo_root: &Path) -> Vec<Task> {
    let local_tasks_file = repo_root.join(".cvh").join("tasks.json");
    if local_tasks_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&local_tasks_file).await {
            if let Ok(tasks) = serde_json::from_str::<Vec<Task>>(&content) {
                if !tasks.is_empty() {
                    return tasks;
                }
            }
        }
    }

    // Try global fallback: %LOCALAPPDATA%\com.voimax.cvh\tasks.json
    let global_tasks_file = get_app_config_dir().join("tasks.json");
    if global_tasks_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&global_tasks_file).await {
            if let Ok(tasks) = serde_json::from_str::<Vec<Task>>(&content) {
                if !tasks.is_empty() {
                    eprintln!(
                        "CVH [load_tasks_from_disk]: Restored {} tasks from global fallback {:?}",
                        tasks.len(),
                        global_tasks_file
                    );
                    return tasks;
                }
            }
        }
    }

    Vec::new()
}

async fn save_tasks_to_disk(repo_root: &Path, tasks: &[Task]) -> Result<(), String> {
    let serialized = serde_json::to_string_pretty(tasks)
        .map_err(|e| format!("Failed to serialize tasks: {}", e))?;

    // 1. Save to repo_root/.cvh/tasks.json
    let local_tasks_file = repo_root.join(".cvh").join("tasks.json");
    let local_res = atomic_write_file(&local_tasks_file, &serialized).await;

    // 2. ALSO save a copy to %LOCALAPPDATA%\com.voimax.cvh\tasks.json as a global fallback!
    let global_tasks_file = get_app_config_dir().join("tasks.json");
    let global_res = atomic_write_file(&global_tasks_file, &serialized).await;

    match (local_res, global_res) {
        (Ok(_), _) => Ok(()),
        (Err(e), Ok(_)) => {
            eprintln!(
                "CVH: Warning - Failed to save tasks to local repo {:?}: {}, but persisted to global fallback {:?}",
                local_tasks_file, e, global_tasks_file
            );
            Ok(())
        }
        (Err(e1), Err(e2)) => Err(format!(
            "Failed to save tasks to both local ({}) and global ({})",
            e1, e2
        )),
    }
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
pub async fn save_tasks(tasks: Vec<Task>, state: State<'_, AppState>) -> Result<(), String> {
    let root = state.get_root();
    save_tasks_to_disk(&root, &tasks).await
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
        custom_flow: None,
        status: None,
        model: None,
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
        .unwrap_or_else(|_| crate::config::default_pipeline_config());

    let col = pipeline
        .columns
        .iter()
        .find(|c| c.id == task.stage_id)
        .cloned()
        .unwrap_or_else(|| {
            eprintln!(
                "CVH [run_agent]: Column '{}' not found in pipeline, falling back to default column",
                task.stage_id
            );
            pipeline.columns.first().cloned().unwrap_or_else(|| crate::config::PipelineColumn {
                id: task.stage_id.clone(),
                title: task.stage_id.clone(),
                color: None,
                icon: None,
                persona: Some("developer".to_string()),
                context_window: Some(400000),
                model_override: None,
                reasoning_effort: None,
                temperature: Some(0.2),
                gatekeeper_sensor: None,
                allowed_tools: None,
                hooks: HashMap::new(),
                routes: None,
            })
        });

    let persona_role = col.persona.clone().unwrap_or_else(|| "developer".to_string());

    let proj_cfg = load_project_config(&root)
        .await
        .unwrap_or_else(|_| ProjectConfig {
            name: "CanbanVoimaxHarness".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            active_pipeline: None,
            default_model: Some("gemini-3.8-flash-high".to_string()),
            api_base_url: Some("http://127.0.0.1:8045/v1".to_string()),
            tool_permissions: None,
            providers: HashMap::new(),
        });

    let llm_settings = load_llm_settings(&root)
        .await
        .unwrap_or_default();

    // Model resolution precedence:
    // 1. Task-specific model override (task.model)
    // 2. Global LLM settings model (llm_settings.model)
    // 3. Pipeline column model override (col.model_override)
    // 4. Project default model (proj_cfg.default_model)
    let model = if let Some(ref tm) = task.model {
        if !tm.trim().is_empty() {
            tm.trim().to_string()
        } else if !llm_settings.model.trim().is_empty() {
            llm_settings.model.trim().to_string()
        } else if let Some(ref m) = col.model_override {
            if !m.trim().is_empty() {
                m.trim().to_string()
            } else {
                proj_cfg.default_model.clone().unwrap_or_else(|| "gemini-3.8-flash-high".to_string())
            }
        } else {
            proj_cfg.default_model.clone().unwrap_or_else(|| "gemini-3.8-flash-high".to_string())
        }
    } else if !llm_settings.model.trim().is_empty() {
        llm_settings.model.trim().to_string()
    } else if let Some(ref m) = col.model_override {
        if !m.trim().is_empty() {
            m.trim().to_string()
        } else {
            proj_cfg.default_model.clone().unwrap_or_else(|| "gemini-3.8-flash-high".to_string())
        }
    } else {
        proj_cfg
            .default_model
            .clone()
            .unwrap_or_else(|| "gemini-3.8-flash-high".to_string())
    };

    let mut base_url = if !llm_settings.base_url.trim().is_empty() {
        llm_settings.base_url.trim().to_string()
    } else {
        proj_cfg
            .api_base_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:8045/v1".to_string())
    };

    if base_url.trim().is_empty() {
        base_url = "http://127.0.0.1:8045/v1".to_string();
    }

    let api_key = if !llm_settings.api_key.trim().is_empty() {
        Some(llm_settings.api_key.trim().to_string())
    } else {
        None
    };

    eprintln!(
        "CVH [run_agent]: task_id={}, model={}, base_url={}, api_key_present={}",
        task.id,
        model,
        base_url,
        api_key.is_some()
    );

    let temperature = llm_settings.temperature.or(col.temperature);

    // If task.worktree_path is None or doesn't exist on disk, create or scaffold cleanly
    let worktree_dir = match &task.worktree_path {
        Some(p) if !p.trim().is_empty() => {
            let pb = Path::new(p);
            let exists = if pb.is_absolute() {
                pb.exists()
            } else {
                root.join(pb).exists()
            };
            if exists {
                if pb.is_absolute() {
                    pb.to_path_buf()
                } else {
                    root.join(pb)
                }
            } else {
                let task_slug = task.id.to_lowercase();
                match create_worktree(&root, &task_slug, "main").await {
                    Ok(wt_abs) => {
                        let wt_str = wt_abs.to_string_lossy().to_string();
                        let branch_name = format!("feat/{}", task_slug);
                        let mut all_tasks = load_tasks_from_disk(&root).await;
                        if let Some(t) = all_tasks.iter_mut().find(|t| t.id == task_id) {
                            t.worktree_path = Some(wt_str.clone());
                            if t.branch.is_none() {
                                t.branch = Some(branch_name);
                            }
                            t.updated_at = Utc::now().timestamp_millis();
                            let _ = save_tasks_to_disk(&root, &all_tasks).await;
                        }
                        wt_abs
                    }
                    Err(e) => {
                        eprintln!(
                            "Worktree creation error for {}: {}, falling back to repo root",
                            task_id, e
                        );
                        root.clone()
                    }
                }
            }
        }
        _ => {
            let task_slug = task.id.to_lowercase();
            match create_worktree(&root, &task_slug, "main").await {
                Ok(wt_abs) => {
                    let wt_str = wt_abs.to_string_lossy().to_string();
                    let branch_name = format!("feat/{}", task_slug);
                    let mut all_tasks = load_tasks_from_disk(&root).await;
                    if let Some(t) = all_tasks.iter_mut().find(|t| t.id == task_id) {
                        t.worktree_path = Some(wt_str.clone());
                        if t.branch.is_none() {
                            t.branch = Some(branch_name);
                        }
                        t.updated_at = Utc::now().timestamp_millis();
                        let _ = save_tasks_to_disk(&root, &all_tasks).await;
                    }
                    wt_abs
                }
                Err(e) => {
                    eprintln!(
                        "Worktree creation error for {}: {}, falling back to repo root",
                        task_id, e
                    );
                    root.clone()
                }
            }
        }
    };

    if !worktree_dir.exists() {
        let _ = tokio::fs::create_dir_all(&worktree_dir).await;
    }

    let abort_flag = Arc::new(AtomicBool::new(false));

    // Store in active agents registry
    {
        let mut map = state
            .active_agents
            .lock()
            .map_err(|_| "Failed to lock active agents".to_string())?;
        map.insert(task_id.clone(), Arc::clone(&abort_flag));
    }

    // Persist status "running" to tasks.json
    {
        let mut all_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = all_tasks.iter_mut().find(|t| t.id == task_id) {
            t.status = Some("running".to_string());
            t.updated_at = Utc::now().timestamp_millis();
            let _ = save_tasks_to_disk(&root, &all_tasks).await;
        }
    }

    // Emit live status: running
    let _ = app_handle.emit(
        "agent:status",
        serde_json::json!({ "taskId": task.id, "status": "running" }),
    );

    let agent_cfg = AgentSessionConfig {
        repo_root: root.clone(),
        worktree_path: worktree_dir,
        role: persona_role,
        stage_id: task.stage_id.clone(),
        task_id: task.id.clone(),
        task_title: task.title.clone(),
        task_description: task.description.clone(),
        base_url,
        api_key,
        model,
        temperature,
        reasoning_effort: col.reasoning_effort.clone(),
        max_turns: 25,
    };

    let tid = task.id.clone();
    let app = app_handle.clone();
    let root_task = root.clone();

    tokio::spawn(async move {
        let app_inner = app.clone();
        let tid_inner = tid.clone();

        let event_sink = move |event: AgentEvent| {
            match event {
                AgentEvent::Token(token) => {
                    let payload = serde_json::json!({
                        "taskId": tid_inner,
                        "output": &token,
                        "content": &token,
                        "token": &token
                    });
                    let _ = app_inner.emit("agent:token", &payload);
                    let _ = app_inner.emit("agent:output", &payload);
                }
                AgentEvent::Thought(thought) => {
                    let _ = app_inner.emit(
                        "agent:thought",
                        serde_json::json!({ "taskId": tid_inner, "thought": &thought, "delta": &thought }),
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
                        serde_json::json!({
                            "taskId": tid_inner,
                            "status": "finished",
                            "response": final_response
                        }),
                    );
                }
                AgentEvent::Aborted => {
                    let _ = app_inner.emit(
                        "agent:aborted",
                        serde_json::json!({ "taskId": tid_inner }),
                    );
                    let _ = app_inner.emit(
                        "agent:status",
                        serde_json::json!({ "taskId": tid_inner, "status": "aborted" }),
                    );
                }
                AgentEvent::Error(err) => {
                    let _ = app_inner.emit(
                        "agent:error",
                        serde_json::json!({ "taskId": tid_inner, "error": &err }),
                    );
                    let _ = app_inner.emit(
                        "agent:status",
                        serde_json::json!({ "taskId": tid_inner, "status": "error", "error": &err }),
                    );
                    let _ = app_inner.emit(
                        "agent:output",
                        serde_json::json!({
                            "taskId": tid_inner,
                            "output": format!("\n[LLM Error: {}]\n", &err),
                            "content": format!("\n[LLM Error: {}]\n", &err),
                        }),
                    );
                }
            }
        };

        let result = run_agent_loop(agent_cfg, abort_flag, event_sink).await;

        match result {
            Ok(final_response) => {
                let _ = app.emit(
                    "agent:status",
                    serde_json::json!({ "taskId": tid, "status": "finished" }),
                );

                // Auto-promotion along Custom Flow or Pipeline Flow
                let mut tasks = load_tasks_from_disk(&root_task).await;
                if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                    let old_stage = tasks[task_idx].stage_id.clone();

                    // Check for review verdict if current stage is review
                    if tasks[task_idx].stage_id == "review" {
                        let upper = final_response.to_uppercase();
                        if upper.contains("APPROVED_WITH_CAVEATS") {
                            tasks[task_idx].verdict = Some("APPROVED_WITH_CAVEATS".to_string());
                        } else if upper.contains("APPROVED") {
                            tasks[task_idx].verdict = Some("APPROVED".to_string());
                        } else if upper.contains("CHANGES_REQUESTED") {
                            tasks[task_idx].verdict = Some("CHANGES_REQUESTED".to_string());
                        }
                    }

                    let mut promoted = false;

                    // If task has custom_flow
                    if let Some(ref flow) = tasks[task_idx].custom_flow {
                        if !flow.is_empty() {
                            if let Some(curr_idx) = flow.iter().position(|s| s == &tasks[task_idx].stage_id) {
                                if tasks[task_idx].stage_id == "review"
                                    && tasks[task_idx].verdict.as_deref() == Some("CHANGES_REQUESTED")
                                {
                                    // Route back to dev if changes requested
                                    if curr_idx > 0 {
                                        tasks[task_idx].stage_id = flow[curr_idx - 1].clone();
                                    } else {
                                        tasks[task_idx].stage_id = "dev".to_string();
                                    }
                                    tasks[task_idx].status = Some("running".to_string());
                                    promoted = true;
                                } else if curr_idx + 1 < flow.len() {
                                    let next_stage = flow[curr_idx + 1].clone();
                                    tasks[task_idx].stage_id = next_stage.clone();
                                    if next_stage == "ready" {
                                        tasks[task_idx].status = Some("done".to_string());
                                    } else {
                                        tasks[task_idx].status = Some("running".to_string());
                                    }
                                    promoted = true;
                                } else {
                                    // At end of custom_flow: set status to done (ready)
                                    tasks[task_idx].stage_id = "ready".to_string();
                                    tasks[task_idx].status = Some("done".to_string());
                                    promoted = true;
                                }
                            } else {
                                // Current stage not found in custom_flow (e.g. backlog): start from first stage
                                let next_stage = flow[0].clone();
                                tasks[task_idx].stage_id = next_stage.clone();
                                if next_stage == "ready" {
                                    tasks[task_idx].status = Some("done".to_string());
                                } else {
                                    tasks[task_idx].status = Some("running".to_string());
                                }
                                promoted = true;
                            }
                        }
                    } else {
                        // Standard pipeline fallback
                        if tasks[task_idx].stage_id == "review"
                            && tasks[task_idx].verdict.as_deref() == Some("CHANGES_REQUESTED")
                        {
                            tasks[task_idx].stage_id = "dev".to_string();
                            tasks[task_idx].status = Some("running".to_string());
                            promoted = true;
                        } else if let Ok(pipeline) = load_pipeline_config(&root_task).await {
                            if let Some(curr_idx) = pipeline.columns.iter().position(|c| c.id == tasks[task_idx].stage_id) {
                                if curr_idx + 1 < pipeline.columns.len() {
                                    let next_stage = pipeline.columns[curr_idx + 1].id.clone();
                                    tasks[task_idx].stage_id = next_stage.clone();
                                    if next_stage == "ready" {
                                        tasks[task_idx].status = Some("done".to_string());
                                    } else {
                                        tasks[task_idx].status = Some("running".to_string());
                                    }
                                    promoted = true;
                                } else {
                                    tasks[task_idx].status = Some("done".to_string());
                                    promoted = true;
                                }
                            }
                        }
                    }

                    if promoted {
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                        let updated_task = tasks[task_idx].clone();
                        let new_stage = updated_task.stage_id.clone();

                        let _ = save_tasks_to_disk(&root_task, &tasks).await;

                        // Trigger on_enter hook for the new stage if configured
                        if let Ok(pipeline) = load_pipeline_config(&root_task).await {
                            if let Some(col) = pipeline.columns.iter().find(|c| c.id == new_stage) {
                                if let Some(hook_rel) = col.hooks.get("on_enter") {
                                    let hook_path = root_task.join(hook_rel);
                                    if hook_path.exists() {
                                        let ctx = HookContext {
                                            task_id: updated_task.id.clone(),
                                            task_title: updated_task.title.clone(),
                                            worktree_path: updated_task.worktree_path.clone().unwrap_or_default(),
                                            branch: updated_task.branch.clone().unwrap_or_default(),
                                            verdict: updated_task.verdict.clone(),
                                            extra_env: HashMap::new(),
                                        };
                                        let _ = execute_hook(hook_path, &root_task, &ctx, None).await;
                                    }
                                }
                            }
                        }

                        let _ = app.emit(
                            "agent:finished",
                            serde_json::json!({
                                "taskId": tid,
                                "status": "finished",
                                "response": final_response,
                                "stageId": new_stage,
                                "task": &updated_task,
                            }),
                        );

                        let _ = app.emit(
                            "task:moved",
                            serde_json::json!({
                                "taskId": tid,
                                "fromStage": old_stage,
                                "toStage": new_stage,
                                "stageId": new_stage,
                                "status": updated_task.status,
                                "task": updated_task,
                            }),
                        );
                    } else {
                        if tasks[task_idx].stage_id == "ready" {
                            tasks[task_idx].status = Some("done".to_string());
                        }
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                        let updated_task = tasks[task_idx].clone();
                        let _ = save_tasks_to_disk(&root_task, &tasks).await;
                        let _ = app.emit(
                            "task:moved",
                            serde_json::json!({
                                "taskId": tid,
                                "fromStage": old_stage.clone(),
                                "toStage": old_stage,
                                "stageId": tasks[task_idx].stage_id,
                                "status": updated_task.status,
                                "task": updated_task,
                            }),
                        );
                    }
                }
            }
            Err(err) => {
                let err_str = err.to_string();
                eprintln!("CVH [run_agent]: Agent execution failed for task {}: {}", tid, err_str);
                let _ = app.emit(
                    "agent:status",
                    serde_json::json!({ "taskId": tid, "status": "error", "error": &err_str }),
                );
                let _ = app.emit(
                    "agent:error",
                    serde_json::json!({ "taskId": tid, "error": &err_str }),
                );
                let _ = app.emit(
                    "agent:output",
                    serde_json::json!({
                        "taskId": tid,
                        "output": format!("\n[LLM Error: {}]\n", &err_str),
                        "content": format!("\n[LLM Error: {}]\n", &err_str),
                    }),
                );
                // Also persist error status in tasks.json
                let mut tasks = load_tasks_from_disk(&root_task).await;
                if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                    tasks[task_idx].status = Some("error".to_string());
                    tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                    let _ = save_tasks_to_disk(&root_task, &tasks).await;
                }
            }
        }

        // Clean up from active_agents registry upon agent completion
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(mut map) = state_ref.active_agents.lock() {
                map.remove(&tid);
            }
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

    // If an agent is currently running for this task, signal abort before re-triggering
    {
        let mut map = state
            .active_agents
            .lock()
            .map_err(|_| "Failed to lock active agents".to_string())?;
        if let Some(flag) = map.remove(&task_id) {
            flag.store(true, Ordering::SeqCst);
        }
    }

    // Append steering instruction into task description on disk so subsequent priming contains it
    {
        let mut current_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = current_tasks.iter_mut().find(|t| t.id == task_id) {
            t.description.push_str(&format!("\n\n[USER STEER]: {}", message));
            t.updated_at = Utc::now().timestamp_millis();
            let _ = save_tasks_to_disk(&root, &current_tasks).await;
        }
    }

    // Append user turn to active compaction context if present
    if let Ok(contexts) = state.agent_contexts.lock() {
        if let Some(cs) = contexts.get(&task_id) {
            if let Ok(mut guard) = cs.lock() {
                guard.messages.push(crate::llm::ChatMessage::user(format!(
                    "[USER STEER]: {}",
                    message
                )));
            }
        }
    }

    // Re-invoke agent loop with steering message
    run_agent(task_id, app_handle, state).await
}

// ============================================================================
// LLM Connection & Settings IPC Commands
// ============================================================================

#[tauri::command]
pub async fn get_llm_settings(state: State<'_, AppState>) -> Result<LlmSettings, String> {
    let root = state.get_root();
    load_llm_settings(&root)
        .await
        .map_err(|e| format!("Failed to load LLM settings: {}", e))
}

#[tauri::command]
pub async fn save_llm_settings(
    settings: LlmSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    save_llm_settings_to_disk(&root, &settings)
        .await
        .map_err(|e| format!("Failed to save LLM settings: {}", e))
}

#[tauri::command]
pub async fn fetch_available_models(
    base_url: String,
    api_key: Option<String>,
) -> Result<Vec<String>, String> {
    let trimmed_base = base_url.trim().trim_end_matches('/');
    if trimmed_base.is_empty() {
        return Err("base_url cannot be empty".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Prepare candidate endpoints in priority order
    let mut candidates = Vec::new();
    if trimmed_base.ends_with("/models") || trimmed_base.ends_with("/api/tags") {
        candidates.push(trimmed_base.to_string());
    } else if trimmed_base.ends_with("/v1") {
        candidates.push(format!("{}/models", trimmed_base));
        let root = trimmed_base.trim_end_matches("/v1");
        candidates.push(format!("{}/models", root));
        candidates.push(format!("{}/api/tags", root));
    } else {
        candidates.push(format!("{}/models", trimmed_base));
        candidates.push(format!("{}/v1/models", trimmed_base));
        candidates.push(format!("{}/api/tags", trimmed_base));
    }

    let mut last_error = String::new();

    for url in candidates {
        let mut req = client.get(&url);
        if let Some(ref key) = api_key {
            let key_trimmed = key.trim();
            if !key_trimmed.is_empty() {
                req = req.header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", key_trimmed),
                );
            }
        }

        match req.send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            let mut models = Vec::new();

                            // OpenAI format: {"data": [{"id": "model-id"}, ...]}
                            if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                                for item in data {
                                    if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                                        models.push(id.to_string());
                                    }
                                }
                            }
                            // Ollama format: {"models": [{"name": "model-id"}, ...]}
                            else if let Some(models_arr) = json.get("models").and_then(|m| m.as_array()) {
                                for item in models_arr {
                                    if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                                        models.push(name.to_string());
                                    } else if let Some(model) = item.get("model").and_then(|m| m.as_str()) {
                                        models.push(model.to_string());
                                    }
                                }
                            }
                            // Array format fallback: [{"id": "..."}, ...] or ["model-id", ...]
                            else if let Some(arr) = json.as_array() {
                                for item in arr {
                                    if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                                        models.push(id.to_string());
                                    } else if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                                        models.push(name.to_string());
                                    } else if let Some(s) = item.as_str() {
                                        models.push(s.to_string());
                                    }
                                }
                            }

                            if !models.is_empty() {
                                models.sort();
                                models.dedup();
                                return Ok(models);
                            } else {
                                last_error = format!("Endpoint {} returned no recognized models in JSON payload", url);
                            }
                        }
                        Err(e) => {
                            last_error = format!("Failed to parse JSON from {}: {}", url, e);
                        }
                    }
                } else {
                    last_error = format!("Endpoint {} returned HTTP status {}", url, resp.status());
                }
            }
            Err(e) => {
                last_error = format!("Failed to connect to {}: {}", url, e);
            }
        }
    }

    Err(format!("Could not fetch models from {}: {}", base_url, last_error))
}

// ============================================================================
// Workspace & Project Management IPC Commands
// ============================================================================

#[tauri::command]
pub async fn select_project_folder(
    path: Option<String>,
    app_handle: AppHandle,
) -> Result<Option<String>, String> {
    if let Some(p) = path {
        if !p.trim().is_empty() {
            let path_buf = PathBuf::from(p.trim());
            if path_buf.exists() && path_buf.is_dir() {
                let _ = scaffold_cvh_project(&path_buf).await;
                persist_active_project(&path_buf).await;
                return Ok(Some(path_buf.to_string_lossy().to_string()));
            }
        }
    }

    // tauri-plugin-dialog replaces the direct rfd dependency to avoid its
    // COM/OLE STA initialization conflicting with WebView2. The blocking call
    // runs on the blocking-pool thread so the event loop is never stalled.
    let selected = tokio::task::spawn_blocking(move || {
        app_handle
            .dialog()
            .file()
            .set_title("Select Project Folder")
            .blocking_pick_folder()
    })
    .await
    .map_err(|e| format!("Folder dialog task failed: {}", e))?
    .and_then(|folder| folder.simplified().into_path().ok());

    if let Some(ref folder) = selected {
        let _ = scaffold_cvh_project(folder).await;
        persist_active_project(folder).await;
    }

    Ok(selected.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn set_active_project(
    path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<ProjectConfig, String> {
    let new_path = PathBuf::from(path.trim());
    if !new_path.exists() {
        return Err(format!("Specified project directory does not exist: {:?}", new_path));
    }
    if !new_path.is_dir() {
        return Err(format!("Specified path is not a directory: {:?}", new_path));
    }

    // Persist to %LOCALAPPDATA%\com.voimax.cvh\active_project.txt
    persist_active_project(&new_path).await;

    // Scaffold empty .cvh structure (agents/, worktrees/, skills/, settings/) if missing
    scaffold_cvh_project(&new_path)
        .await
        .map_err(|e| format!("Failed to scaffold .cvh structure: {}", e))?;

    // Update AppState repo_root using write lock
    state.set_root(new_path.clone())?;

    // Register or update project in global registry
    let _ = register_or_update_project(None, &new_path).await;

    // Reload or default project configuration
    let proj_cfg = match load_project_config(&new_path).await {
        Ok(cfg) => cfg,
        Err(_) => ProjectConfig {
            name: new_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("CanbanProject")
                .to_string(),
            version: "0.1.0".to_string(),
            description: None,
            active_pipeline: None,
            default_model: Some("gemini-3.8-flash-high".to_string()),
            api_base_url: Some("http://127.0.0.1:8045/v1".to_string()),
            tool_permissions: None,
            providers: HashMap::new(),
        },
    };

    // Emit event to notify frontend of project switch
    let _ = app_handle.emit(
        "project:changed",
        serde_json::json!({
            "rootPath": new_path.to_string_lossy().to_string(),
            "name": proj_cfg.name
        }),
    );

    Ok(proj_cfg)
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectEntry>, String> {
    let root = state.get_root();
    let registry = load_project_registry(None)
        .await
        .unwrap_or_default();

    let mut projects = registry.projects;

    // Ensure active project is represented in the list
    let root_str = root.to_string_lossy().to_string();
    let contains_current = projects
        .iter()
        .any(|p| p.path == root_str || p.root_path.as_deref() == Some(&root_str));

    if !contains_current && root.exists() {
        let entry = ProjectEntry::from_path(&root);
        projects.insert(0, entry);
    }

    Ok(projects)
}

// Backward-compatible alias for choose_project_folder
#[tauri::command]
pub async fn choose_project_folder(app_handle: AppHandle) -> Result<Option<String>, String> {
    select_project_folder(None, app_handle).await
}

// Backward-compatible alias for set_project_root
#[tauri::command]
pub async fn set_project_root(
    path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    persist_active_project(&PathBuf::from(path.trim())).await;
    let _cfg = set_active_project(path.clone(), state, app_handle).await?;
    Ok(path)
}

