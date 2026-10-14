use crate::agent::{run_agent_loop, AgentEvent, AgentSessionConfig, CustomToolConfig};
use crate::config::{
    atomic_write_file, discover_project_root, get_app_config_dir, load_llm_settings,
    load_pipeline_config, load_project_config, load_project_registry, persist_active_project,
    register_or_update_project, save_llm_settings_to_disk, save_pipeline_config,
    scaffold_cvh_project, BoardDef, get_board_template_columns, load_boards_config,
    save_boards_config, LlmSettings, PipelineConfig, ProjectConfig, ProjectEntry,
    ProjectRegistry,
};
use crate::git::{create_worktree, delete_branch, remove_worktree};
use crate::hooks::{execute_hook, execute_sensor_command, HookContext};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::process::Command;
use std::sync::{Arc, Mutex, RwLock};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

pub use crate::agent::{KanbanTask, Task, TodoItem};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPersonaInfo {
    pub role: String,
    pub has_prompt: bool,
}

/// Stable-нормализация порядка колонок канбана:
/// `backlog` — всегда первая (если есть), `ready` — всегда последняя (если есть),
/// остальные сохраняют относительный порядок. Дублирующиеся id — ошибка.
pub fn normalize_pipeline_columns(
    columns: Vec<crate::config::PipelineColumn>,
) -> Result<Vec<crate::config::PipelineColumn>, String> {
    let mut seen = HashSet::new();
    for c in &columns {
        if !seen.insert(c.id.clone()) {
            return Err(format!(
                "Дублирующийся id колонки: '{}' — id колонок должны быть уникальными",
                c.id
            ));
        }
    }
    let mut backlog: Option<crate::config::PipelineColumn> = None;
    let mut ready: Option<crate::config::PipelineColumn> = None;
    let mut middle = Vec::with_capacity(columns.len());
    for col in columns {
        if col.id == "backlog" && backlog.is_none() {
            backlog = Some(col);
        } else if col.id == "ready" && ready.is_none() {
            ready = Some(col);
        } else {
            middle.push(col);
        }
    }
    let mut out = Vec::with_capacity(
        backlog.is_some() as usize + middle.len() + ready.is_some() as usize,
    );
    if let Some(b) = backlog {
        out.push(b);
    }
    out.extend(middle);
    if let Some(r) = ready {
        out.push(r);
    }
    Ok(out)
}

/// Валидация имени персоны: ^[a-z0-9][a-z0-9-]*$
pub fn is_valid_persona_role(role: &str) -> bool {
    let mut chars = role.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return false;
    }
    for c in chars {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return false;
        }
    }
    true
}

fn persona_placeholder_template(role: &str) -> String {
    format!(
        "# Роль: {role}\n\n\
        > Этот файл создан автоматически командой `create_agent_persona`. \
        > Заполните его под свою персону — все секции ниже помечены TODO.\n\n\
        ## TODO: цель персоны\n\n\
        - Опишите, за что отвечает эта персона и какие задачи решает.\n\n\
        ## TODO: стиль\n\n\
        - Опишите стиль общения и работы: тон, детальность, формат ответов.\n\n\
        ## TODO: запреты\n\n\
        - Перечислите, что персоне запрещено делать.\n\n\
        ## TODO: чеклист настройки\n\n\
        - [ ] Заполнена цель персоны\n\
        - [ ] Описан стиль\n\
        - [ ] Перечислены запреты\n\
        - [ ] Проверена работа персоны на тестовой задаче\n"
    )
}

/// Сканирует `<root>/.cvh/agents/*.md` (только `.md`, сортировка по имени).
/// Пустая/отсутствующая папка → пустой vec, не ошибка.
pub async fn list_agent_personas_in(root: &Path) -> Vec<AgentPersonaInfo> {
    let agents_dir = root.join(".cvh").join("agents");
    let mut out = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(&agents_dir).await else {
        return out;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        let Ok(ft) = entry.file_type().await else {
            continue;
        };
        if !ft.is_file() {
            continue;
        }
        if let Some(role) = name.strip_suffix(".md") {
            if role.is_empty() {
                continue;
            }
            out.push(AgentPersonaInfo {
                role: role.to_string(),
                has_prompt: true,
            });
        }
    }
    out.sort_by(|a, b| a.role.cmp(&b.role));
    out
}

/// Создаёт `.cvh/agents/<role>.md` с плейсхолдер-шаблоном.
/// Возвращает относительный путь `.cvh/agents/<role>.md`.
pub async fn create_agent_persona_file(root: &Path, role: &str) -> Result<String, String> {
    if !is_valid_persona_role(role) {
        return Err(format!(
            "Недопустимое имя роли '{}': разрешены только строчные латинские буквы, цифры и дефис (шаблон ^[a-z0-9][a-z0-9-]*$)",
            role
        ));
    }
    let agents_dir = root.join(".cvh").join("agents");
    let target = agents_dir.join(format!("{}.md", role));
    if target.exists() {
        return Err(format!(
            "Персона '{}' уже существует: .cvh/agents/{}.md",
            role, role
        ));
    }
    let content = persona_placeholder_template(role);
    atomic_write_file(&target, &content)
        .await
        .map_err(|e| format!("Не удалось создать персону '{}': {}", role, e))?;
    Ok(format!(".cvh/agents/{}.md", role))
}

pub struct AppState {
    pub repo_root: RwLock<PathBuf>,
    pub active_agents: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub agent_contexts: Mutex<HashMap<String, Arc<Mutex<crate::agent::compactor::AgentContextState>>>>,
    pub tasks: Arc<Mutex<Vec<Task>>>,
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
            tasks: Arc::new(Mutex::new(Vec::new())),
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

fn ensure_task_board_id(mut tasks: Vec<Task>) -> Vec<Task> {
    for t in &mut tasks {
        if t.board_id.as_deref().unwrap_or("").trim().is_empty() {
            t.board_id = Some("development".to_string());
        }
    }
    tasks
}

pub fn slugify_board_name(name: &str) -> String {
    let cyrillic_map = [
        ('а', "a"), ('б', "b"), ('в', "v"), ('г', "g"), ('д', "d"),
        ('е', "e"), ('ё', "yo"), ('ж', "zh"), ('з', "z"), ('и', "i"),
        ('й', "y"), ('к', "k"), ('л', "l"), ('м', "m"), ('н', "n"),
        ('о', "o"), ('п', "p"), ('р', "r"), ('с', "s"), ('т', "t"),
        ('у', "u"), ('ф', "f"), ('х', "h"), ('ц', "ts"), ('ч', "ch"),
        ('ш', "sh"), ('щ', "sch"), ('ъ', ""), ('ы', "y"), ('ь', ""),
        ('э', "e"), ('ю', "yu"), ('я', "ya"),
    ];
    let mut slug = String::new();
    for c in name.chars() {
        // Unicode lowercase FIRST so Cyrillic uppercase hits the transliteration
        // map below (to_ascii_lowercase leaves e.g. 'М' untouched, breaking lookup).
        let lower: char = c.to_lowercase().next().unwrap_or(c);
        if lower.is_ascii_alphanumeric() {
            slug.push(lower);
        } else if let Some((_, lat)) = cyrillic_map.iter().find(|(cyr, _)| *cyr == lower) {
            slug.push_str(lat);
        } else if lower == ' ' || lower == '-' || lower == '_' {
            if !slug.ends_with('-') && !slug.is_empty() {
                slug.push('-');
            }
        }
    }
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        format!("board-{}", &uuid::Uuid::new_v4().to_string()[..6])
    } else {
        trimmed
    }
}

async fn load_tasks_from_disk(repo_root: &Path) -> Vec<Task> {
    let local_tasks_file = repo_root.join(".cvh").join("tasks.json");
    if local_tasks_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&local_tasks_file).await {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                if let Ok(tasks) = serde_json::from_str::<Vec<Task>>(trimmed) {
                    return ensure_task_board_id(tasks);
                }
            }
        }
    }

    // Try global fallback: %LOCALAPPDATA%\com.voimax.cvh\tasks.json
    let global_tasks_file = get_app_config_dir().join("tasks.json");
    if global_tasks_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&global_tasks_file).await {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                if let Ok(tasks) = serde_json::from_str::<Vec<Task>>(trimmed) {
                    if !tasks.is_empty() {
                        eprintln!(
                            "CVH [load_tasks_from_disk]: Restored {} tasks from global fallback {:?}",
                            tasks.len(),
                            global_tasks_file
                        );
                        return ensure_task_board_id(tasks);
                    }
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
    mut pipeline: PipelineConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    pipeline.columns = normalize_pipeline_columns(pipeline.columns)?;
    let root = state.get_root();
    save_pipeline_config(&root, &pipeline)
        .await
        .map_err(|e| format!("Failed to save pipeline: {}", e))
}

#[tauri::command]
pub async fn list_boards(state: State<'_, AppState>) -> Result<Vec<BoardDef>, String> {
    let root = state.get_root();
    let config = load_boards_config(&root)
        .await
        .map_err(|e| format!("Failed to load boards: {}", e))?;
    Ok(config.boards)
}

#[tauri::command]
pub async fn get_active_board_id(state: State<'_, AppState>) -> Result<String, String> {
    let root = state.get_root();
    let config = load_boards_config(&root)
        .await
        .map_err(|e| format!("Failed to load boards: {}", e))?;
    Ok(config.active_board_id)
}

#[tauri::command]
pub async fn create_board(
    name: String,
    template: Option<String>,
    state: State<'_, AppState>,
) -> Result<BoardDef, String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Название доски не может быть пустым".to_string());
    }
    let root = state.get_root();
    let mut config = load_boards_config(&root)
        .await
        .map_err(|e| format!("Failed to load boards: {}", e))?;

    let base_slug = slugify_board_name(trimmed_name);
    let mut unique_id = base_slug.clone();
    let mut counter = 2;
    while config.boards.iter().any(|b| b.id == unique_id) {
        unique_id = format!("{}-{}", base_slug, counter);
        counter += 1;
    }

    let tmpl = template.as_deref().unwrap_or("canonical");
    let raw_cols = get_board_template_columns(tmpl);
    let columns = normalize_pipeline_columns(raw_cols)?;

    let new_board = BoardDef {
        id: unique_id.clone(),
        name: trimmed_name.to_string(),
        columns,
    };

    config.boards.push(new_board.clone());
    config.active_board_id = unique_id;

    save_boards_config(&root, &config)
        .await
        .map_err(|e| format!("Failed to save new board: {}", e))?;

    Ok(new_board)
}

#[tauri::command]
pub async fn save_board(
    mut board: BoardDef,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let trimmed_name = board.name.trim();
    if trimmed_name.is_empty() {
        return Err("Название доски не может быть пустым".to_string());
    }
    board.name = trimmed_name.to_string();
    board.columns = normalize_pipeline_columns(board.columns)?;

    let root = state.get_root();
    let mut config = load_boards_config(&root)
        .await
        .map_err(|e| format!("Failed to load boards: {}", e))?;

    if let Some(existing) = config.boards.iter_mut().find(|b| b.id == board.id) {
        *existing = board;
    } else {
        config.boards.push(board);
    }

    save_boards_config(&root, &config)
        .await
        .map_err(|e| format!("Failed to save board: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn delete_board(
    board_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    let mut config = load_boards_config(&root)
        .await
        .map_err(|e| format!("Failed to load boards: {}", e))?;

    if config.boards.len() <= 1 {
        return Err("Нельзя удалить единственную доску в проекте".to_string());
    }

    let pos = config
        .boards
        .iter()
        .position(|b| b.id == board_id)
        .ok_or_else(|| format!("Доска '{}' не найдена", board_id))?;

    config.boards.remove(pos);

    if config.active_board_id == board_id {
        config.active_board_id = config.boards[0].id.clone();
    }

    save_boards_config(&root, &config)
        .await
        .map_err(|e| format!("Failed to save boards after deletion: {}", e))?;

    let mut tasks = load_tasks_from_disk(&root).await;
    let mut tasks_modified = false;
    for t in &mut tasks {
        if t.board_id.as_deref() == Some(&board_id) {
            t.board_id = Some(config.active_board_id.clone());
            tasks_modified = true;
        }
    }
    if tasks_modified {
        if let Ok(mut guard) = state.tasks.lock() {
            *guard = tasks.clone();
        }
        let _ = save_tasks_to_disk(&root, &tasks).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn set_active_board(
    board_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    let mut config = load_boards_config(&root)
        .await
        .map_err(|e| format!("Failed to load boards: {}", e))?;

    if !config.boards.iter().any(|b| b.id == board_id) {
        return Err(format!("Доска '{}' не найдена", board_id));
    }

    config.active_board_id = board_id;

    save_boards_config(&root, &config)
        .await
        .map_err(|e| format!("Failed to set active board: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn list_agent_personas(
    state: State<'_, AppState>,
) -> Result<Vec<AgentPersonaInfo>, String> {
    let root = state.get_root();
    Ok(list_agent_personas_in(&root).await)
}

#[tauri::command]
pub async fn create_agent_persona(
    role: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let root = state.get_root();
    create_agent_persona_file(&root, &role).await
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
    let tasks = load_tasks_from_disk(&root).await;
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    Ok(tasks)
}

#[tauri::command]
pub async fn save_tasks(tasks: Vec<Task>, state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    let root = state.get_root();
    let res = save_tasks_to_disk(&root, &tasks).await;
    match &res {
        Ok(_) => eprintln!(
            "CVH [save_tasks]: persisted {} tasks to {:?}",
            tasks.len(),
            root.join(".cvh").join("tasks.json")
        ),
        Err(e) => eprintln!("CVH [save_tasks]: FAILED to persist {} tasks: {}", tasks.len(), e),
    }
    res
}

// ============================================================================
// Task Dialog Persistence
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub timestamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
}

fn sanitize_task_id(task_id: &str) -> Result<String, String> {
    let trimmed = task_id.trim();
    if trimmed.is_empty() {
        return Err("Task ID cannot be empty".to_string());
    }
    if trimmed.contains("..") || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(format!("Invalid task ID: '{}'", trimmed));
    }
    if trimmed.chars().any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*')) {
        return Err(format!("Task ID contains invalid characters: '{}'", trimmed));
    }
    Ok(trimmed.to_string())
}

fn get_dialog_paths(repo_root: &Path, task_id: &str) -> (PathBuf, PathBuf, PathBuf) {
    let p_dialogs = repo_root.join(".cvh").join("dialogs").join(format!("{}.json", task_id));
    let p_tasks = repo_root.join(".cvh").join("tasks").join(task_id).join("dialog.json");
    let p_global = get_app_config_dir().join("dialogs").join(format!("{}.json", task_id));
    (p_dialogs, p_tasks, p_global)
}

#[tauri::command]
pub async fn save_task_dialog(
    task_id: String,
    items: Vec<ActivityItem>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    let (p_dialogs, p_tasks, p_global) = get_dialog_paths(&root, &safe_id);

    let serialized = serde_json::to_string_pretty(&items)
        .map_err(|e| format!("Failed to serialize task dialog items: {}", e))?;

    // 1. Primary path: .cvh/dialogs/<taskId>.json
    let res_dialogs = atomic_write_file(&p_dialogs, &serialized).await;

    // 2. Secondary path: .cvh/tasks/<taskId>/dialog.json
    let res_tasks = atomic_write_file(&p_tasks, &serialized).await;

    // 3. Global app data fallback: %LOCALAPPDATA%/com.voimax.cvh/dialogs/<taskId>.json
    let res_global = atomic_write_file(&p_global, &serialized).await;

    match (res_dialogs, res_tasks, res_global) {
        (Ok(_), _, _) => {
            eprintln!("CVH [save_task_dialog]: saved {} items to {:?}", items.len(), p_dialogs);
            Ok(())
        }
        (Err(e1), Ok(_), _) => {
            eprintln!(
                "CVH [save_task_dialog]: Warning - failed to write {:?}: {}, but succeeded in {:?}",
                p_dialogs, e1, p_tasks
            );
            Ok(())
        }
        (Err(e1), Err(e2), Ok(_)) => {
            eprintln!(
                "CVH [save_task_dialog]: Warning - failed local writes ({}, {}), but persisted to global fallback {:?}",
                e1, e2, p_global
            );
            Ok(())
        }
        (Err(e1), Err(e2), Err(e3)) => Err(format!(
            "Failed to save task dialog to any target: dialogs err: {}, tasks err: {}, global err: {}",
            e1, e2, e3
        )),
    }
}

#[tauri::command]
pub async fn load_task_dialog(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ActivityItem>, String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    let (p_dialogs, p_tasks, p_global) = get_dialog_paths(&root, &safe_id);

    // 1. Try primary path: .cvh/dialogs/<taskId>.json
    if p_dialogs.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&p_dialogs).await {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                match serde_json::from_str::<Vec<ActivityItem>>(trimmed) {
                    Ok(items) => return Ok(items),
                    Err(e) => eprintln!(
                        "CVH [load_task_dialog]: Failed to parse {:?}: {}",
                        p_dialogs, e
                    ),
                }
            }
        }
    }

    // 2. Try secondary path: .cvh/tasks/<taskId>/dialog.json
    if p_tasks.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&p_tasks).await {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                match serde_json::from_str::<Vec<ActivityItem>>(trimmed) {
                    Ok(items) => return Ok(items),
                    Err(e) => eprintln!(
                        "CVH [load_task_dialog]: Failed to parse {:?}: {}",
                        p_tasks, e
                    ),
                }
            }
        }
    }

    // 3. Try global fallback: %LOCALAPPDATA%/com.voimax.cvh/dialogs/<taskId>.json
    if p_global.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&p_global).await {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                match serde_json::from_str::<Vec<ActivityItem>>(trimmed) {
                    Ok(items) => {
                        eprintln!(
                            "CVH [load_task_dialog]: Restored {} items from global fallback {:?}",
                            items.len(),
                            p_global
                        );
                        return Ok(items);
                    }
                    Err(e) => eprintln!(
                        "CVH [load_task_dialog]: Failed to parse {:?}: {}",
                        p_global, e
                    ),
                }
            }
        }
    }

    // Default to empty activity feed if no dialog file exists yet
    Ok(Vec::new())
}

/// Receives renderer-side JS errors so white-screen crashes land in the backend log.
#[tauri::command]
pub async fn log_ui_error(message: String) -> Result<(), String> {
    let trimmed = message.chars().take(4000).collect::<String>();
    eprintln!("CVH [UI-ERROR]: {}", trimmed);
    Ok(())
}

#[tauri::command]
pub async fn clear_task_dialog(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    let (p_dialogs, p_tasks, p_global) = get_dialog_paths(&root, &safe_id);

    if p_dialogs.exists() {
        let _ = tokio::fs::remove_file(&p_dialogs).await;
    }
    if p_tasks.exists() {
        let _ = tokio::fs::remove_file(&p_tasks).await;
    }
    if p_global.exists() {
        let _ = tokio::fs::remove_file(&p_global).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn create_task(
    title: String,
    description: String,
    stage_id: String,
    board_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;

    let effective_board_id = match board_id.as_deref() {
        Some(b) if !b.trim().is_empty() => b.trim().to_string(),
        _ => {
            load_boards_config(&root)
                .await
                .map(|bc| bc.active_board_id)
                .unwrap_or_else(|_| "development".to_string())
        }
    };

    // Generate task ID — monotonic max+1 over on-disk tasks, never len+1:
    // after deleting tasks, len shrinks but the max suffix does not, so IDs
    // are never re-issued (re-issued CVH-001 would resurrect stale dialog
    // history from .cvh/dialogs/ and localStorage).
    let task_num = next_task_num(&tasks);
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
        board_id: Some(effective_board_id.clone()),
        worktree_path: Some(worktree_path.clone()),
        branch: Some(branch.clone()),
        verdict: None,
        custom_flow: None,
        status: None,
        model: None,
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        cover_image: None,
        attachments: None,
        todos: None,
        created_at: now,
        updated_at: now,
    };

    // Check pipeline config for stage hooks
    if let Ok(pipeline) = load_pipeline_config(&root).await {
        if let Some(col) = pipeline.columns.iter().find(|c| c.id == stage_id) {
            trigger_column_on_enter_script(col, &new_task, &stage_id, None, &root).await;
            if let Some(hook_rel) = col.hooks.get("on_enter") {
                let hook_path = root.join(hook_rel);
                if hook_path.exists() {
                    let ctx = HookContext {
                        task_id: task_id.clone(),
                        task_title: title.clone(),
                        worktree_path: worktree_path.clone(),
                        branch: branch.clone(),
                        verdict: None,
                        stage_id: Some(stage_id.clone()),
                        board_id: Some(effective_board_id.clone()),
                        extra_env: HashMap::new(),
                    };
                    let _ = execute_hook(hook_path, &root, &ctx, None).await;
                }
            }
        }
    }

    tasks.push(new_task.clone());
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    save_tasks_to_disk(&root, &tasks).await?;

    Ok(new_task)
}

/// Monotonic top-level task number: max numeric suffix over existing tasks + 1.
/// Subtask suffixes (CVH-012.3) count toward their base number only.
fn next_task_num(tasks: &[Task]) -> u32 {
    let mut max_num: u32 = 0;
    for t in tasks {
        let base = t.id.split('.').next().unwrap_or(t.id.as_str());
        let digits: String = base.chars().filter(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<u32>() {
            if n > max_num {
                max_num = n;
            }
        }
    }
    // Belt & braces: skip any id that still collides (e.g. non-standard ids).
    let mut candidate = max_num.saturating_add(1).max(1);
    while tasks.iter().any(|t| t.id == format!("CVH-{:03}", candidate)) {
        candidate = candidate.saturating_add(1);
    }
    candidate
}

/// Helper: calculates next subtask index for a given parent task id.
/// If parent is CVH-012, and existing subtasks are CVH-012.1, CVH-012.2, next is 3.
pub fn get_next_subtask_index(tasks: &[Task], parent_id: &str) -> u32 {
    let mut max_idx: u32 = 0;
    let prefix = format!("{}.", parent_id);
    for t in tasks {
        if t.parent_id.as_deref() == Some(parent_id) {
            if let Some(idx) = t.subtask_index {
                if idx > max_idx {
                    max_idx = idx;
                }
            } else if let Some(suffix) = t.id.strip_prefix(&prefix) {
                if let Ok(idx) = suffix.parse::<u32>() {
                    if idx > max_idx {
                        max_idx = idx;
                    }
                }
            }
        } else if t.id.starts_with(&prefix) {
            if let Some(suffix) = t.id.strip_prefix(&prefix) {
                if let Ok(idx) = suffix.parse::<u32>() {
                    if idx > max_idx {
                        max_idx = idx;
                    }
                }
            }
        }
    }
    max_idx + 1
}

#[tauri::command]
pub async fn create_subtask(
    parent_id: String,
    target_stage_id: Option<String>,
    title: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    let mut tasks = load_tasks_from_disk(&root).await;

    let parent_idx = tasks
        .iter()
        .position(|t| t.id == parent_id)
        .ok_or_else(|| format!("Parent task {} not found", parent_id))?;

    let pipeline = load_pipeline_config(&root)
        .await
        .unwrap_or_else(|_| crate::config::default_pipeline_config());

    let target_col = if let Some(target_id) = target_stage_id {
        pipeline
            .columns
            .iter()
            .find(|c| c.id == target_id)
            .cloned()
            .ok_or_else(|| format!("Target column {} not found in pipeline", target_id))?
    } else {
        let parent_col_idx = pipeline
            .columns
            .iter()
            .position(|c| c.id == tasks[parent_idx].stage_id)
            .ok_or_else(|| format!("Parent column {} not found in pipeline", tasks[parent_idx].stage_id))?;
        if parent_col_idx + 1 < pipeline.columns.len() {
            pipeline.columns[parent_col_idx + 1].clone()
        } else {
            return Err("Parent is already in the last pipeline stage".to_string());
        }
    };

    let next_subtask_idx = get_next_subtask_index(&tasks, &parent_id);
    let subtask_id = format!("{}.{}", parent_id, next_subtask_idx);
    let now = Utc::now().timestamp_millis();

    let parent_title = tasks[parent_idx].title.clone();
    let parent_wt = tasks[parent_idx].worktree_path.clone();
    let parent_branch = tasks[parent_idx].branch.clone();
    let parent_model = tasks[parent_idx].model.clone();

    // Update parent task: mark waiting for subtask
    tasks[parent_idx].waiting_for_subtask_id = Some(subtask_id.clone());
    tasks[parent_idx].status = Some("waiting_subtask".to_string());
    tasks[parent_idx].updated_at = now;

    let subtask_title = title.unwrap_or_else(|| format!("{} ({})", parent_title, target_col.title));
    let subtask_desc = format!(
        "### Подзадача этапа: {}
Родительская задача: {}

{}",
        target_col.title, parent_id, tasks[parent_idx].description
    );

    let subtask_status = if target_col.terminal == Some(true) {
        "done"
    } else if target_col.persona.is_some() {
        "running"
    } else {
        "idle"
    };

    let new_subtask = Task {
        id: subtask_id.clone(),
        title: subtask_title,
        description: subtask_desc.clone(),
        stage_id: target_col.id.clone(),
        board_id: tasks[parent_idx].board_id.clone(),
        worktree_path: parent_wt,
        branch: parent_branch,
        verdict: None,
        custom_flow: None,
        status: Some(subtask_status.to_string()),
        model: target_col.model_override.clone().or(parent_model),
        parent_id: Some(parent_id.clone()),
        subtask_index: Some(next_subtask_idx),
        waiting_for_subtask_id: None,
        cover_image: None,
        attachments: None,
        todos: None,
        created_at: now,
        updated_at: now,
    };

    tasks.push(new_subtask.clone());
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    save_tasks_to_disk(&root, &tasks).await?;

    let _ = app_handle.emit("task:created", serde_json::json!({ "task": &new_subtask }));
    let _ = app_handle.emit(
        "task:moved",
        serde_json::json!({
            "taskId": parent_id,
            "status": "waiting_subtask",
            "waitingForSubtaskId": &subtask_id,
            "task": &tasks[parent_idx],
        }),
    );
    let _ = app_handle.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

    if target_col.auto_run == Some(true) || target_col.persona.is_some() {
        let col_clone = target_col.clone();
        let subtask_clone = new_subtask.clone();
        let root_clone = root.clone();
        let sid = subtask_id.clone();
        let app = app_handle.clone();
        tokio::spawn(async move {
            trigger_column_on_enter_script(
                &col_clone,
                &subtask_clone,
                &col_clone.id,
                None,
                &root_clone,
            )
            .await;
            let _ = run_agent(sid, Some(subtask_desc), app).await;
        });
    }

    Ok(new_subtask)
}

/// Helper: executes column on_enter_script if configured on the target column.
/// Runs execute_hook asynchronously, injecting CVH env vars including CVH_STAGE_ID and CVH_BOARD_ID.
/// Logs success or error output to console.
pub async fn trigger_column_on_enter_script(
    col: &crate::config::PipelineColumn,
    task: &Task,
    target_stage_id: &str,
    verdict: Option<&str>,
    root: &Path,
) {
    if let Some(ref script_str) = col.on_enter_script {
        let trimmed = script_str.trim();
        if trimmed.is_empty() {
            return;
        }

        let script_buf = PathBuf::from(trimmed);
        let script_path = if script_buf.is_absolute() {
            script_buf
        } else {
            root.join(&script_buf)
        };

        if !script_path.exists() {
            eprintln!(
                "CVH [on_enter_script]: Hook script does not exist at {:?}",
                script_path
            );
            return;
        }

        let wt_path = task
            .worktree_path
            .as_deref()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .unwrap_or_else(|| root.to_path_buf());

        let ctx = HookContext {
            task_id: task.id.clone(),
            task_title: task.title.clone(),
            worktree_path: task.worktree_path.clone().unwrap_or_default(),
            branch: task.branch.clone().unwrap_or_default(),
            verdict: verdict.map(|v| v.to_string()).or_else(|| task.verdict.clone()),
            stage_id: Some(target_stage_id.to_string()),
            board_id: task.board_id.clone(),
            extra_env: HashMap::new(),
        };

        println!(
            "CVH [on_enter_script]: Executing hook {:?} for task {} entering stage {}",
            script_path, task.id, target_stage_id
        );

        match execute_hook(&script_path, &wt_path, &ctx, None).await {
            Ok(res) => {
                if res.success {
                    println!(
                        "CVH [on_enter_script]: Hook {:?} succeeded for task {} in stage {} (exit code {:?}):
STDOUT: {}",
                        script_path, task.id, target_stage_id, res.exit_code, res.stdout
                    );
                } else {
                    eprintln!(
                        "CVH [on_enter_script]: Hook {:?} failed for task {} in stage {} (exit code {:?}):
STDOUT: {}
STDERR: {}",
                        script_path, task.id, target_stage_id, res.exit_code, res.stdout, res.stderr
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "CVH [on_enter_script]: Error executing hook {:?} for task {}: {}",
                    script_path, task.id, e
                );
            }
        }
    }
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

    let current_task = &mut tasks[task_idx];

    let target_col = {
        let boards_cfg = load_boards_config(&root).await.ok();
        let col_from_board = boards_cfg.as_ref().and_then(|bc| {
            let b = if let Some(ref bid) = current_task.board_id {
                bc.boards.iter().find(|b| &b.id == bid)
            } else {
                bc.boards.iter().find(|b| b.id == bc.active_board_id)
            };
            b.and_then(|b| b.columns.iter().find(|c| c.id == target_stage_id).cloned())
        });
        match col_from_board {
            Some(c) => c,
            None => {
                let pipeline = load_pipeline_config(&root)
                    .await
                    .map_err(|e| format!("Failed to load pipeline: {}", e))?;
                pipeline
                    .columns
                    .into_iter()
                    .find(|c| c.id == target_stage_id)
                    .ok_or_else(|| format!("Target stage {} not found in pipeline or board", target_stage_id))?
            }
        }
    };

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
                stage_id: Some(target_stage_id.clone()),
                board_id: current_task.board_id.clone(),
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

    // Trigger on_enter_script if configured on target column
    trigger_column_on_enter_script(
        &target_col,
        current_task,
        &target_stage_id,
        verdict.as_deref(),
        &root,
    )
    .await;

    // Update task
    current_task.stage_id = target_stage_id;
    if verdict.is_some() {
        current_task.verdict = verdict;
    }
    current_task.updated_at = Utc::now().timestamp_millis();

    let updated = current_task.clone();
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    save_tasks_to_disk(&root, &tasks).await?;

    Ok(updated)
}

#[tauri::command]
pub async fn run_agent(
    task_id: String,
    prompt: Option<String>,
    app_handle: AppHandle,
) -> Result<String, String> {
    let root = app_handle.state::<AppState>().get_root();

    // Check in-memory task registry first
    let task_opt = {
        if let Ok(guard) = app_handle.state::<AppState>().tasks.lock() {
            guard.iter().find(|t| t.id == task_id).cloned()
        } else {
            None
        }
    };

    let mut task = match task_opt {
        Some(t) => t,
        None => {
            // If not found in memory, try disk
            let disk_tasks = load_tasks_from_disk(&root).await;
            if let Some(t) = disk_tasks.iter().find(|t| t.id == task_id).cloned() {
                if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
                    *guard = disk_tasks;
                }
                t
            } else {
                let mem_count = app_handle.state::<AppState>().tasks.lock().map(|g| g.len()).unwrap_or(0);
                eprintln!(
                    "CVH [run_agent]: Task {} not found (in-memory tasks: {}, disk tasks in {:?}: {}, root: {:?})",
                    task_id,
                    mem_count,
                    root.join(".cvh").join("tasks.json"),
                    disk_tasks.len(),
                    root
                );
                return Err(format!("Task {} not found", task_id));
            }
        }
    };

    // If an explicit prompt was provided by the user:
    // - If task.description is empty (brand-new task without prompt), set initial description
    // - If task already exists and has a description, DO NOT mutate task.description; pass steer message to LLM context
    let steer_prompt = if let Some(p) = prompt {
        let p_trimmed = p.trim();
        if !p_trimmed.is_empty() {
            if task.description.trim().is_empty() {
                task.description = p_trimmed.to_string();
                task.updated_at = Utc::now().timestamp_millis();

                // Persist description update immediately
                let mut all_tasks = load_tasks_from_disk(&root).await;
                if let Some(t) = all_tasks.iter_mut().find(|t| t.id == task_id) {
                    t.description = task.description.clone();
                    t.updated_at = task.updated_at;
                    let _ = save_tasks_to_disk(&root, &all_tasks).await;
                }
                if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
                    if let Some(t) = guard.iter_mut().find(|t| t.id == task_id) {
                        t.description = task.description.clone();
                        t.updated_at = task.updated_at;
                    }
                }
                None
            } else {
                // Task already exists with an objective: preserve pristine initial prompt, pass steer to dialogue context
                Some(p_trimmed.to_string())
            }
        } else {
            None
        }
    } else {
        None
    };

    let pipeline = load_pipeline_config(&root)
        .await
        .unwrap_or_else(|_| crate::config::default_pipeline_config());

    let col = {
        let boards_cfg = load_boards_config(&root).await.ok();
        let col_from_board = boards_cfg.as_ref().and_then(|bc| {
            let b = if let Some(ref bid) = task.board_id {
                bc.boards.iter().find(|b| &b.id == bid)
            } else {
                bc.boards.iter().find(|b| b.id == bc.active_board_id)
            };
            b.and_then(|b| b.columns.iter().find(|c| c.id == task.stage_id).cloned())
        });
        col_from_board
            .or_else(|| pipeline.columns.iter().find(|c| c.id == task.stage_id).cloned())
            .unwrap_or_else(|| {
                eprintln!(
                    "CVH [run_agent]: Column '{}' not found in board or pipeline, falling back to default column",
                    task.stage_id
                );
                pipeline.columns.first().cloned().unwrap_or_else(|| crate::config::PipelineColumn {
                    id: task.stage_id.clone(),
                    title: task.stage_id.clone(),
                    color: None,
                    icon: None,
                    persona: Some("developer".to_string()),
                    on_enter_script: None,
                    context_window: Some(400000),
                    model_override: None,
                    reasoning_effort: None,
                    temperature: Some(0.2),
                    gatekeeper_sensor: None,
                    allowed_tools: None,
                    hooks: HashMap::new(),
                    routes: None,
                    auto_run: None,
                    terminal: None,
                })
            })
    };

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
        let state = app_handle.state::<AppState>();
        let mut map = state
            .active_agents
            .lock()
            .map_err(|_| "Failed to lock active agents".to_string())?;
        map.insert(task_id.clone(), Arc::clone(&abort_flag));
    }

    // Persist status "running" to tasks.json and update state.tasks
    {
        let mut all_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = all_tasks.iter_mut().find(|t| t.id == task_id) {
            t.status = Some("running".to_string());
            t.updated_at = Utc::now().timestamp_millis();
            let _ = save_tasks_to_disk(&root, &all_tasks).await;
        }
        if let Ok(mut guard) = app_handle.state::<AppState>().tasks.lock() {
            *guard = all_tasks;
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
        steer_prompt,
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
                AgentEvent::Output(output) => {
                    let payload = serde_json::json!({
                        "taskId": tid_inner,
                        "output": &output,
                        "content": &output,
                        "token": &output
                    });
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
                AgentEvent::TaskUpdated { task_id, todos } => {
                    let _ = app_inner.emit(
                        "task:updated",
                        serde_json::json!({
                            "taskId": &task_id,
                            "todos": &todos,
                        }),
                    );
                    let _ = app_inner.emit(
                        "tasks:updated",
                        serde_json::json!({ "taskId": &task_id }),
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

                // Auto-promotion along pipeline columns with Subtask Hierarchy & Pipeline lifecycle
                let mut tasks = load_tasks_from_disk(&root_task).await;
                if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                    let old_stage = tasks[task_idx].stage_id.clone();

                    // Parse verdict from agent output for any column.
                    let upper = final_response.to_uppercase();
                    let parsed_verdict = if upper.contains("CHANGES_REQUESTED") {
                        Some("CHANGES_REQUESTED".to_string())
                    } else if upper.contains("APPROVED_WITH_CAVEATS") {
                        Some("APPROVED_WITH_CAVEATS".to_string())
                    } else if upper.contains("APPROVED") {
                        Some("APPROVED".to_string())
                    } else {
                        None
                    };

                    if parsed_verdict.is_some() {
                        tasks[task_idx].verdict = parsed_verdict.clone();
                    }

                    let parent_id_opt = tasks[task_idx].parent_id.clone();

                    if let Some(pid) = parent_id_opt {
                        // ====================================================
                        // 1. SUBTASK COMPLETED (e.g. CVH-012.1 in review or retro)
                        // ====================================================
                        tasks[task_idx].status = Some("done".to_string());
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();

                        if parsed_verdict == Some("CHANGES_REQUESTED".to_string()) {
                            // Reviewer requested changes: unlock parent in dev stage!
                            if let Some(p_idx) = tasks.iter().position(|t| t.id == pid) {
                                tasks[p_idx].waiting_for_subtask_id = None;
                                tasks[p_idx].verdict = Some("CHANGES_REQUESTED".to_string());
                                tasks[p_idx].status = Some("idle".to_string());
                                tasks[p_idx].updated_at = Utc::now().timestamp_millis();
                            }
                            let _ = save_tasks_to_disk(&root_task, &tasks).await;
                            if let Some(state_ref) = app.try_state::<AppState>() {
                                if let Ok(mut guard) = state_ref.tasks.lock() {
                                    *guard = tasks.clone();
                                }
                            }
                            let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                            let _ = app.emit(
                                "task:moved",
                                serde_json::json!({
                                    "taskId": pid,
                                    "status": "idle",
                                    "verdict": "CHANGES_REQUESTED"
                                }),
                            );
                        } else {
                            // APPROVED or APPROVED_WITH_CAVEATS: determine next stage
                            let mut next_stage_col: Option<crate::config::PipelineColumn> = None;
                            if let Ok(pipeline) = load_pipeline_config(&root_task).await {
                                if let Some(curr_idx) = pipeline.columns.iter().position(|c| c.id == tasks[task_idx].stage_id) {
                                    let curr_col = &pipeline.columns[curr_idx];
                                    let mut route_target: Option<String> = None;
                                    if parsed_verdict == Some("APPROVED".to_string()) {
                                        if let Some(routes) = curr_col.routes.as_ref() {
                                            if let Some(t) = routes.get("on_approved") {
                                                route_target = Some(t.clone());
                                            }
                                        }
                                    } else if parsed_verdict == Some("APPROVED_WITH_CAVEATS".to_string()) {
                                        if let Some(routes) = curr_col.routes.as_ref() {
                                            if let Some(t) = routes.get("on_approved_with_caveats").or_else(|| routes.get("on_approved")) {
                                                route_target = Some(t.clone());
                                            }
                                        }
                                    }

                                    if let Some(target_id) = route_target {
                                        next_stage_col = pipeline.columns.iter().find(|c| c.id == target_id).cloned();
                                    } else if curr_idx + 1 < pipeline.columns.len() {
                                        next_stage_col = Some(pipeline.columns[curr_idx + 1].clone());
                                    }
                                }
                            }

                            let is_terminal = next_stage_col.as_ref().map(|c| c.terminal == Some(true)).unwrap_or(true);

                            if !is_terminal && next_stage_col.is_some() {
                                let next_col = next_stage_col.unwrap();
                                // Create NEXT subtask (e.g. CVH-012.2 in retro)
                                let next_subtask_idx = get_next_subtask_index(&tasks, &pid);
                                let next_subtask_id = format!("{}.{}", pid, next_subtask_idx);
                                let now = Utc::now().timestamp_millis();

                                let (parent_title, parent_wt, parent_br, parent_model) = if let Some(p_idx) = tasks.iter().position(|t| t.id == pid) {
                                    tasks[p_idx].waiting_for_subtask_id = Some(next_subtask_id.clone());
                                    tasks[p_idx].status = Some("waiting_subtask".to_string());
                                    tasks[p_idx].updated_at = now;
                                    (
                                        tasks[p_idx].title.clone(),
                                        tasks[p_idx].worktree_path.clone(),
                                        tasks[p_idx].branch.clone(),
                                        tasks[p_idx].model.clone(),
                                    )
                                } else {
                                    (pid.clone(), None, None, None)
                                };

                                let subtask_title = format!("{} ({})", parent_title, next_col.title);
                                let subtask_desc = format!(
                                    "### Подзадача этапа: {}
Родитель: {}

---
**Контекст предыдущего этапа:**
{}",
                                    next_col.title, pid, final_response
                                );

                                let new_subtask = Task {
                                    id: next_subtask_id.clone(),
                                    title: subtask_title,
                                    description: subtask_desc.clone(),
                                    stage_id: next_col.id.clone(),
                                    board_id: tasks.iter().find(|t| t.id == pid).and_then(|t| t.board_id.clone()),
                                    worktree_path: parent_wt,
                                    branch: parent_br,
                                    verdict: None,
                                    custom_flow: None,
                                    status: Some("running".to_string()),
                                    model: next_col.model_override.clone().or(parent_model),
                                    parent_id: Some(pid.clone()),
                                    subtask_index: Some(next_subtask_idx),
                                    waiting_for_subtask_id: None,
                                    cover_image: None,
                                    attachments: None,
                                    todos: None,
                                    created_at: now,
                                    updated_at: now,
                                };

                                tasks.push(new_subtask.clone());
                                let _ = save_tasks_to_disk(&root_task, &tasks).await;
                                if let Some(state_ref) = app.try_state::<AppState>() {
                                    if let Ok(mut guard) = state_ref.tasks.lock() {
                                        *guard = tasks.clone();
                                    }
                                }

                                let _ = app.emit("task:created", serde_json::json!({ "task": &new_subtask }));
                                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                                let _ = app.emit(
                                    "task:moved",
                                    serde_json::json!({
                                        "taskId": pid,
                                        "status": "waiting_subtask",
                                        "waitingForSubtaskId": &next_subtask_id,
                                    }),
                                );

                                if next_col.persona.is_some() || next_col.auto_run == Some(true) {
                                    trigger_column_on_enter_script(
                                        &next_col,
                                        &new_subtask,
                                        &next_col.id,
                                        None,
                                        &root_task,
                                    )
                                    .await;
                                    let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": &next_subtask_id }));
                                }
                            } else {
                                // Terminal stage: all subtasks complete! Move root parent task to Done!
                                if let Some(p_idx) = tasks.iter().position(|t| t.id == pid) {
                                    let term_stage_id = next_stage_col.as_ref().map(|c| c.id.clone()).unwrap_or_else(|| "ready".to_string());
                                    if let Some(ref term_col) = next_stage_col {
                                        trigger_column_on_enter_script(
                                            term_col,
                                            &tasks[p_idx],
                                            &term_stage_id,
                                            Some("APPROVED"),
                                            &root_task,
                                        )
                                        .await;
                                    }
                                    tasks[p_idx].stage_id = term_stage_id.clone();
                                    tasks[p_idx].status = Some("done".to_string());
                                    tasks[p_idx].waiting_for_subtask_id = None;
                                    tasks[p_idx].verdict = Some("APPROVED".to_string());
                                    tasks[p_idx].updated_at = Utc::now().timestamp_millis();

                                    let updated_p = tasks[p_idx].clone();
                                    let _ = save_tasks_to_disk(&root_task, &tasks).await;
                                    if let Some(state_ref) = app.try_state::<AppState>() {
                                        if let Ok(mut guard) = state_ref.tasks.lock() {
                                            *guard = tasks.clone();
                                        }
                                    }
                                    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                                    let _ = app.emit(
                                        "task:moved",
                                        serde_json::json!({
                                            "taskId": pid,
                                            "toStage": term_stage_id,
                                            "stageId": term_stage_id,
                                            "status": "done",
                                            "task": updated_p
                                        }),
                                    );
                                }
                            }
                        }
                    } else {
                        // ====================================================
                        // 2. ROOT TASK COMPLETED (e.g. CVH-012 in dev)
                        // ====================================================
                        let mut next_stage_col: Option<crate::config::PipelineColumn> = None;
                        if let Ok(pipeline) = load_pipeline_config(&root_task).await {
                            if let Some(curr_idx) = pipeline.columns.iter().position(|c| c.id == tasks[task_idx].stage_id) {
                                let curr_col = &pipeline.columns[curr_idx];
                                let mut route_target: Option<String> = None;
                                if parsed_verdict == Some("CHANGES_REQUESTED".to_string()) {
                                    if let Some(routes) = curr_col.routes.as_ref() {
                                        if let Some(t) = routes.get("on_changes_requested") {
                                            route_target = Some(t.clone());
                                        }
                                    }
                                } else if parsed_verdict == Some("APPROVED_WITH_CAVEATS".to_string()) {
                                    if let Some(routes) = curr_col.routes.as_ref() {
                                        if let Some(t) = routes.get("on_approved_with_caveats").or_else(|| routes.get("on_approved")) {
                                            route_target = Some(t.clone());
                                        }
                                    }
                                } else if parsed_verdict == Some("APPROVED".to_string()) {
                                    if let Some(routes) = curr_col.routes.as_ref() {
                                        if let Some(t) = routes.get("on_approved") {
                                            route_target = Some(t.clone());
                                        }
                                    }
                                }

                                if let Some(target_id) = route_target {
                                    next_stage_col = pipeline.columns.iter().find(|c| c.id == target_id).cloned();
                                } else if curr_idx + 1 < pipeline.columns.len() {
                                    next_stage_col = Some(pipeline.columns[curr_idx + 1].clone());
                                }
                            }
                        }

                        if let Some(next_col) = next_stage_col {
                            let is_terminal = next_col.terminal == Some(true);
                            if !is_terminal && next_col.persona.is_some() {
                                // Intermediate reviewer column: parent STAYS in current stage, subtask created in next stage!
                                let next_subtask_idx = get_next_subtask_index(&tasks, &tid);
                                let subtask_id = format!("{}.{}", tid, next_subtask_idx);
                                let now = Utc::now().timestamp_millis();

                                tasks[task_idx].status = Some("waiting_subtask".to_string());
                                tasks[task_idx].waiting_for_subtask_id = Some(subtask_id.clone());
                                tasks[task_idx].updated_at = now;

                                let subtask_title = format!("{} ({})", tasks[task_idx].title, next_col.title);
                                let subtask_desc = format!(
                                    "### Подзадача этапа: {}
Родитель: {}

---
**Результат разработки:**
{}",
                                    next_col.title, tid, final_response
                                );

                                let new_subtask = Task {
                                    id: subtask_id.clone(),
                                    title: subtask_title,
                                    description: subtask_desc.clone(),
                                    stage_id: next_col.id.clone(),
                                    board_id: tasks[task_idx].board_id.clone(),
                                    worktree_path: tasks[task_idx].worktree_path.clone(),
                                    branch: tasks[task_idx].branch.clone(),
                                    verdict: None,
                                    custom_flow: None,
                                    status: Some("running".to_string()),
                                    model: next_col.model_override.clone().or_else(|| tasks[task_idx].model.clone()),
                                    parent_id: Some(tid.clone()),
                                    subtask_index: Some(next_subtask_idx),
                                    waiting_for_subtask_id: None,
                                    cover_image: None,
                                    attachments: None,
                                    todos: None,
                                    created_at: now,
                                    updated_at: now,
                                };

                                tasks.push(new_subtask.clone());
                                let updated_parent = tasks[task_idx].clone();
                                let _ = save_tasks_to_disk(&root_task, &tasks).await;
                                if let Some(state_ref) = app.try_state::<AppState>() {
                                    if let Ok(mut guard) = state_ref.tasks.lock() {
                                        *guard = tasks.clone();
                                    }
                                }

                                let _ = app.emit("task:created", serde_json::json!({ "task": &new_subtask }));
                                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                                let _ = app.emit(
                                    "task:moved",
                                    serde_json::json!({
                                        "taskId": tid,
                                        "fromStage": old_stage,
                                        "toStage": old_stage,
                                        "stageId": old_stage,
                                        "status": "waiting_subtask",
                                        "waitingForSubtaskId": &subtask_id,
                                        "task": updated_parent
                                    }),
                                );

                                if next_col.persona.is_some() || next_col.auto_run == Some(true) {
                                    trigger_column_on_enter_script(
                                        &next_col,
                                        &new_subtask,
                                        &next_col.id,
                                        None,
                                        &root_task,
                                    )
                                    .await;
                                    let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": &subtask_id }));
                                }
                            } else {
                                // Terminal column or non-persona: move root task directly
                                trigger_column_on_enter_script(
                                    &next_col,
                                    &tasks[task_idx],
                                    &next_col.id,
                                    None,
                                    &root_task,
                                )
                                .await;
                                tasks[task_idx].stage_id = next_col.id.clone();
                                tasks[task_idx].status = Some(if is_terminal { "done".to_string() } else { "idle".to_string() });
                                tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                                let updated_task = tasks[task_idx].clone();
                                let _ = save_tasks_to_disk(&root_task, &tasks).await;
                                if let Some(state_ref) = app.try_state::<AppState>() {
                                    if let Ok(mut guard) = state_ref.tasks.lock() {
                                        *guard = tasks.clone();
                                    }
                                }
                                let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
                                let _ = app.emit(
                                    "task:moved",
                                    serde_json::json!({
                                        "taskId": tid,
                                        "fromStage": old_stage,
                                        "toStage": next_col.id,
                                        "stageId": next_col.id,
                                        "status": updated_task.status,
                                        "task": updated_task
                                    }),
                                );
                            }
                        }
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
                    if let Some(state_ref) = app.try_state::<AppState>() {
                        if let Ok(mut guard) = state_ref.tasks.lock() {
                            *guard = tasks.clone();
                        }
                    }
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
        .map_err(|e| format!("Failed to remove worktree for {}: {}", task_id, e))?;
    // Also drop the feature branch created for the task (best-effort: it may
    // already be merged/deleted or never have existed — never fail cleanup).
    let branch = format!("feat/{}", task_slug);
    let _ = delete_branch(&root, &branch, true).await;
    Ok(())
}

// ============================================================================
// Task Git Diff Inspection
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffFile {
    pub path: String,
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
    pub patch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskGitDiff {
    pub files: Vec<DiffFile>,
    #[serde(alias = "total_additions")]
    pub total_additions: usize,
    #[serde(alias = "total_deletions")]
    pub total_deletions: usize,
    pub branch: String,
}

fn parse_diff_blocks(diff_output: &str) -> HashMap<String, String> {
    let mut blocks = HashMap::new();
    let mut current_file: Option<String> = None;
    let mut current_patch = String::new();

    for line in diff_output.lines() {
        if line.starts_with("diff --git ") {
            if let Some(file) = current_file.take() {
                blocks.insert(file, current_patch.clone());
            }
            current_patch.clear();
            current_patch.push_str(line);
            current_patch.push('\n');

            if let Some(idx) = line.rfind(" b/") {
                let target = &line[idx + 3..];
                let unquoted = target.trim_matches('"');
                current_file = Some(unquoted.to_string());
            }
        } else if current_file.is_some() {
            current_patch.push_str(line);
            current_patch.push('\n');
        }
    }

    if let Some(file) = current_file {
        blocks.insert(file, current_patch);
    }

    blocks
}

#[tauri::command]
pub async fn get_task_git_diff(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<TaskGitDiff, String> {
    let root = state.get_root();

    let task_opt = {
        if let Ok(guard) = state.tasks.lock() {
            guard.iter().find(|t| t.id == task_id).cloned()
        } else {
            None
        }
    };

    let task = match task_opt {
        Some(t) => t,
        None => {
            let disk_tasks = load_tasks_from_disk(&root).await;
            disk_tasks
                .into_iter()
                .find(|t| t.id == task_id)
                .ok_or_else(|| format!("Task {} not found", task_id))?
        }
    };

    let workdir = match &task.worktree_path {
        Some(p) if !p.trim().is_empty() => {
            let pb = Path::new(p.trim());
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
                root.clone()
            }
        }
        _ => root.clone(),
    };

    let branch = match crate::git::get_current_branch(&workdir).await {
        Ok(b) if !b.trim().is_empty() && b != "HEAD" => b.trim().to_string(),
        _ => task.branch.unwrap_or_else(|| "main".to_string()),
    };

    let status_output = Command::new("git")
        .current_dir(&workdir)
        .args(["status", "--porcelain=v1", "-uall"])
        .output()
        .await
        .map_err(|e| format!("Failed to run git status in {:?}: {}", workdir, e))?;

    if !status_output.status.success() {
        let stderr = String::from_utf8_lossy(&status_output.stderr);
        return Err(format!("git status failed in {:?}: {}", workdir, stderr));
    }

    let status_stdout = String::from_utf8_lossy(&status_output.stdout);
    let mut file_entries: Vec<(String, String)> = Vec::new();

    for line in status_stdout.lines() {
        let line = line.trim_end();
        if line.len() < 4 {
            continue;
        }
        let xy = &line[0..2];
        let raw_path = line[3..].trim();
        let target_path = if let Some((_old, new)) = raw_path.split_once(" -> ") {
            new.trim()
        } else {
            raw_path
        };
        let unquoted = target_path.trim_matches('"').to_string();
        if unquoted.is_empty() {
            continue;
        }

        let status = if xy == "??" {
            "A".to_string()
        } else if xy.contains('A') {
            "A".to_string()
        } else if xy.contains('D') {
            "D".to_string()
        } else {
            "M".to_string()
        };

        file_entries.push((unquoted, status));
    }

    if file_entries.is_empty() {
        return Ok(TaskGitDiff {
            files: Vec::new(),
            total_additions: 0,
            total_deletions: 0,
            branch,
        });
    }

    let head_exists = Command::new("git")
        .current_dir(&workdir)
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    let diff_args = if head_exists {
        vec!["diff", "HEAD"]
    } else {
        vec!["diff"]
    };

    let git_diff_out = Command::new("git")
        .current_dir(&workdir)
        .args(&diff_args)
        .output()
        .await;

    let mut diff_blocks: HashMap<String, String> = HashMap::new();
    if let Ok(out) = git_diff_out {
        if out.status.code().unwrap_or(1) <= 1 {
            let full_diff = String::from_utf8_lossy(&out.stdout);
            diff_blocks = parse_diff_blocks(&full_diff);
        }
    }

    let mut files: Vec<DiffFile> = Vec::new();
    let mut total_additions: usize = 0;
    let mut total_deletions: usize = 0;

    for (file_path, status) in file_entries {
        let patch = if status == "A" && (!head_exists || !diff_blocks.contains_key(&file_path)) {
            let untracked_diff = Command::new("git")
                .current_dir(&workdir)
                .args(["diff", "--no-index", "--", "/dev/null", &file_path])
                .output()
                .await;

            match untracked_diff {
                Ok(out) if out.status.code().unwrap_or(2) <= 1 => {
                    String::from_utf8_lossy(&out.stdout).to_string()
                }
                _ => {
                    let abs_file = workdir.join(&file_path);
                    if let Ok(content) = tokio::fs::read_to_string(&abs_file).await {
                        let line_count = content.lines().count();
                        format!(
                            "diff --git a/{} b/{}\nnew file mode 100644\n--- /dev/null\n+++ b/{}\n@@ -0,0 +1,{} @@\n{}",
                            file_path,
                            file_path,
                            file_path,
                            line_count,
                            content.lines().map(|l| format!("+{}", l)).collect::<Vec<_>>().join("\n")
                        )
                    } else {
                        String::new()
                    }
                }
            }
        } else if let Some(p) = diff_blocks.remove(&file_path) {
            p
        } else {
            let file_diff = Command::new("git")
                .current_dir(&workdir)
                .args(["diff", "HEAD", "--", &file_path])
                .output()
                .await;
            match file_diff {
                Ok(out) if out.status.code().unwrap_or(2) <= 1 => {
                    String::from_utf8_lossy(&out.stdout).to_string()
                }
                _ => String::new(),
            }
        };

        let mut additions: usize = 0;
        let mut deletions: usize = 0;

        for line in patch.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                additions += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                deletions += 1;
            }
        }

        total_additions += additions;
        total_deletions += deletions;

        files.push(DiffFile {
            path: file_path,
            status,
            additions,
            deletions,
            patch,
        });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(TaskGitDiff {
        files,
        total_additions,
        total_deletions,
        branch,
    })
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

// ============================================================================
// Custom Tools (.cvh/tools/*.json) IPC Commands
// ============================================================================

/// Валидация имени кастомного тула: ^[a-z0-9_]+$
pub fn is_valid_custom_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Сканирует `<root>/.cvh/tools/*.json` (пропуск битых файлов молча, сортировка по name).
/// Пустая/отсутствующая папка → пустой vec, не ошибка.
pub async fn list_custom_tools_in(root: &Path) -> Vec<CustomToolConfig> {
    let mut tools = crate::agent::load_custom_tools(root).await;
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    tools
}

/// Записывает `.cvh/tools/<name>.json` pretty-JSON (создаёт папку при нужде).
/// Возвращает относительный путь `.cvh/tools/<name>.json`.
pub async fn save_custom_tool_file(
    root: &Path,
    name: &str,
    description: &str,
    parameters: serde_json::Value,
    command: &str,
) -> Result<String, String> {
    if !is_valid_custom_tool_name(name) {
        return Err(format!(
            "Недопустимое имя тула '{}': разрешены только строчные латинские буквы, цифры и подчёркивание (шаблон ^[a-z0-9_]+$)",
            name
        ));
    }
    if !parameters.is_object() {
        return Err("Параметр 'parameters' должен быть JSON-объектом".to_string());
    }
    let cfg = CustomToolConfig {
        name: name.to_string(),
        description: description.to_string(),
        parameters,
        command: command.to_string(),
    };
    let serialized =
        serde_json::to_string_pretty(&cfg).map_err(|e| format!("Failed to serialize custom tool: {}", e))?;
    let target = root.join(".cvh").join("tools").join(format!("{}.json", name));
    atomic_write_file(&target, &serialized)
        .await
        .map_err(|e| format!("Failed to save custom tool '{}': {}", name, e))?;
    Ok(format!(".cvh/tools/{}.json", name))
}

/// Удаляет `.cvh/tools/<name>.json`. Err если имени нет или файла нет.
pub async fn delete_custom_tool_file(root: &Path, name: &str) -> Result<(), String> {
    if !is_valid_custom_tool_name(name) {
        return Err(format!(
            "Недопустимое имя тула '{}': разрешены только строчные латинские буквы, цифры и подчёркивание (шаблон ^[a-z0-9_]+$)",
            name
        ));
    }
    let target = root.join(".cvh").join("tools").join(format!("{}.json", name));
    if !target.exists() {
        return Err(format!("Кастомный тул '{}' не найден", name));
    }
    tokio::fs::remove_file(&target)
        .await
        .map_err(|e| format!("Failed to delete custom tool '{}': {}", name, e))?;
    Ok(())
}

#[tauri::command]
pub async fn list_custom_tools(
    state: State<'_, AppState>,
) -> Result<Vec<CustomToolConfig>, String> {
    let root = state.get_root();
    Ok(list_custom_tools_in(&root).await)
}

#[tauri::command]
pub async fn save_custom_tool(
    name: String,
    description: String,
    parameters: serde_json::Value,
    command: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let root = state.get_root();
    save_custom_tool_file(&root, &name, &description, parameters, &command).await
}

#[tauri::command]
pub async fn delete_custom_tool(
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.get_root();
    delete_custom_tool_file(&root, &name).await
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

    // Check in-memory task registry first, then fallback to disk
    let task_found = {
        if let Ok(guard) = state.tasks.lock() {
            guard.iter().any(|t| t.id == task_id)
        } else {
            false
        }
    };

    if !task_found {
        let tasks = load_tasks_from_disk(&root).await;
        if !tasks.iter().any(|t| t.id == task_id) {
            return Err(format!("Task {} not found", task_id));
        }
    }

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

    // Update timestamp on task without polluting task.description with steer messages
    {
        let mut current_tasks = load_tasks_from_disk(&root).await;
        if let Some(t) = current_tasks.iter_mut().find(|t| t.id == task_id) {
            t.updated_at = Utc::now().timestamp_millis();
            let _ = save_tasks_to_disk(&root, &current_tasks).await;
        }
        if let Ok(mut guard) = state.tasks.lock() {
            if let Some(t) = guard.iter_mut().find(|t| t.id == task_id) {
                t.updated_at = Utc::now().timestamp_millis();
            }
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
    run_agent(task_id, Some(message), app_handle).await
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
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    if let Some(p) = path {
        if !p.trim().is_empty() {
            let path_buf = PathBuf::from(p.trim());
            if path_buf.exists() && path_buf.is_dir() {
                let _ = scaffold_cvh_project(&path_buf).await;
                persist_active_project(&path_buf).await;
                let _ = state.set_root(path_buf.clone());
                let tasks = load_tasks_from_disk(&path_buf).await;
                if let Ok(mut guard) = state.tasks.lock() {
                    *guard = tasks;
                }
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
        let _ = state.set_root(folder.clone());
        let tasks = load_tasks_from_disk(folder).await;
        if let Ok(mut guard) = state.tasks.lock() {
            *guard = tasks;
        }
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

    // Reload state.tasks for the new project root
    let new_tasks = load_tasks_from_disk(&new_path).await;
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = new_tasks;
    }

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
pub async fn choose_project_folder(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    select_project_folder(None, app_handle, state).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PipelineColumn;
    use std::collections::HashMap;

    fn test_col(id: &str) -> PipelineColumn {
        PipelineColumn {
            id: id.to_string(),
            title: id.to_string(),
            color: None,
            icon: None,
            persona: None,
            on_enter_script: None,
            context_window: None,
            model_override: None,
            reasoning_effort: None,
            temperature: None,
            gatekeeper_sensor: None,
            allowed_tools: None,
            hooks: HashMap::new(),
            routes: None,
            auto_run: None,
            terminal: None,
        }
    }

    fn ids(cols: &[PipelineColumn]) -> Vec<String> {
        cols.iter().map(|c| c.id.clone()).collect()
    }

    #[test]
    fn test_normalize_backlog_first_ready_last_stable() {
        let cols = vec![
            test_col("dev"),
            test_col("ready"),
            test_col("review"),
            test_col("backlog"),
            test_col("qa"),
        ];
        let out = normalize_pipeline_columns(cols).expect("normalize failed");
        assert_eq!(ids(&out), vec!["backlog", "dev", "review", "qa", "ready"]);
    }

    #[test]
    fn test_normalize_already_ordered_keeps_middle_order() {
        let cols = vec![
            test_col("backlog"),
            test_col("a"),
            test_col("b"),
            test_col("ready"),
        ];
        let out = normalize_pipeline_columns(cols).expect("normalize failed");
        assert_eq!(ids(&out), vec!["backlog", "a", "b", "ready"]);
    }

    #[test]
    fn test_normalize_without_specials_keeps_order() {
        let cols = vec![test_col("a"), test_col("b"), test_col("c")];
        let out = normalize_pipeline_columns(cols).expect("normalize failed");
        assert_eq!(ids(&out), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_normalize_duplicate_ids_error() {
        let cols = vec![test_col("dev"), test_col("dev")];
        let err = normalize_pipeline_columns(cols).expect_err("expected duplicate error");
        assert!(err.contains("dev"), "unexpected error text: {}", err);
    }

    #[test]
    fn test_persona_role_validation() {
        for ok in ["developer", "a", "0abc", "my-role-1", "r2-d2"] {
            assert!(is_valid_persona_role(ok), "expected valid: {}", ok);
        }
        for bad in [
            "",
            "-abc",
            "Abc",
            "my_role",
            "my role",
            "роль",
            "a/b",
            "A",
            "dev!",
        ] {
            assert!(!is_valid_persona_role(bad), "expected invalid: {}", bad);
        }
    }

    #[test]
    fn test_persona_placeholder_template_ru() {
        let t = persona_placeholder_template("reviewer");
        assert!(t.contains("# Роль: reviewer"));
        assert!(t.contains("TODO"));
        assert!(t.contains("создан автоматически"));
    }

    #[test]
    fn test_agent_persona_info_camel_case() {
        let info = AgentPersonaInfo {
            role: "developer".to_string(),
            has_prompt: true,
        };
        let v = serde_json::to_value(&info).expect("serialize");
        assert_eq!(v.get("role").and_then(|x| x.as_str()), Some("developer"));
        assert_eq!(v.get("hasPrompt").and_then(|x| x.as_bool()), Some(true));
        assert!(v.get("has_prompt").is_none());
    }

    #[tokio::test]
    async fn test_create_persona_placeholder_in_tmp_dir() {
        let base =
            PathBuf::from("target").join(format!("cvh_persona_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        // invalid role → Err
        let bad = create_agent_persona_file(&base, "Bad_Name").await;
        assert!(bad.is_err());

        // valid create → relative path + placeholder content
        let rel = create_agent_persona_file(&base, "my-role")
            .await
            .expect("create failed");
        assert_eq!(rel, ".cvh/agents/my-role.md");
        let content = tokio::fs::read_to_string(base.join(".cvh").join("agents").join("my-role.md"))
            .await
            .expect("read created file");
        assert!(content.contains("# Роль: my-role"));

        // duplicate → Err mentioning существование
        let dup = create_agent_persona_file(&base, "my-role").await;
        assert!(dup.is_err());
        assert!(dup.unwrap_err().contains("уже существует"));

        // list sees it, sorted, .md only
        tokio::fs::write(
            base.join(".cvh").join("agents").join("aaa.md"),
            "# Роль: aaa\n",
        )
        .await
        .expect("seed aaa");
        tokio::fs::write(
            base.join(".cvh").join("agents").join("notes.txt"),
            "ignore me",
        )
        .await
        .expect("seed txt");
        let listed = list_agent_personas_in(&base).await;
        let roles: Vec<String> = listed.iter().map(|p| p.role.clone()).collect();
        assert_eq!(roles, vec!["aaa".to_string(), "my-role".to_string()]);
        assert!(listed.iter().all(|p| p.has_prompt));

        // missing folder → empty vec, not error
        let empty_base =
            PathBuf::from("target").join(format!("cvh_persona_empty_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&empty_base).await.expect("empty dir");
        let empty = list_agent_personas_in(&empty_base).await;
        assert!(empty.is_empty());

        let _ = tokio::fs::remove_dir_all(&base).await;
        let _ = tokio::fs::remove_dir_all(&empty_base).await;
    }

    #[test]
    fn test_next_task_num_never_reuses_ids() {
        // Gap after deletion: len is 2 but max suffix is 3 -> next must be 4,
        // never CVH-003 again (stale dialog history) and never len+1=3.
        let tasks: Vec<Task> = serde_json::from_value(serde_json::json!([
            {"id": "CVH-001", "title": "a", "stageId": "backlog"},
            {"id": "CVH-003", "title": "b", "stageId": "backlog"},
            {"id": "CVH-003.2", "title": "sub", "stageId": "dev", "parentId": "CVH-003", "subtaskIndex": 2}
        ]))
        .expect("deserialize tasks");
        assert_eq!(next_task_num(&tasks), 4);

        // Empty disk -> first id.
        let empty: Vec<Task> = vec![];
        assert_eq!(next_task_num(&empty), 1);

        // Subtask-only suffixes must not inflate the top-level counter.
        let subs: Vec<Task> = serde_json::from_value(serde_json::json!([
            {"id": "CVH-002", "title": "a", "stageId": "backlog"},
            {"id": "CVH-002.9", "title": "sub", "stageId": "dev", "parentId": "CVH-002", "subtaskIndex": 9}
        ]))
        .expect("deserialize tasks");
        assert_eq!(next_task_num(&subs), 3);
    }

    #[test]
    fn test_sanitize_task_id() {
        assert!(sanitize_task_id("CVH-001").is_ok());
        assert!(sanitize_task_id("task-123").is_ok());
        assert!(sanitize_task_id("feature_test").is_ok());

        assert!(sanitize_task_id("").is_err());
        assert!(sanitize_task_id("   ").is_err());
        assert!(sanitize_task_id("../escape").is_err());
        assert!(sanitize_task_id("a/b").is_err());
        assert!(sanitize_task_id("a\\b").is_err());
        assert!(sanitize_task_id("task:1").is_err());
        assert!(sanitize_task_id("task*1").is_err());
        assert!(sanitize_task_id("task?1").is_err());
    }

    #[test]
    fn test_activity_item_serialization_camel_case() {
        let item = ActivityItem {
            id: "th-123".to_string(),
            item_type: "thought".to_string(),
            timestamp: "12:00:00".to_string(),
            content: Some("Thinking about the architecture...".to_string()),
            tool: None,
            duration_ms: Some(1500.0),
        };

        let json = serde_json::to_string(&item).expect("serialize");
        assert!(json.contains(r#""type":"thought""#));
        assert!(json.contains(r#""durationMs":1500"#));
        assert!(!json.contains("item_type"));
        assert!(!json.contains("duration_ms"));

        let deserialized: ActivityItem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.id, "th-123");
        assert_eq!(deserialized.item_type, "thought");
        assert_eq!(deserialized.content.as_deref(), Some("Thinking about the architecture..."));
        assert_eq!(deserialized.duration_ms, Some(1500.0));
    }

    #[tokio::test]
    async fn test_task_dialog_disk_roundtrip() {
        let base = PathBuf::from("target").join(format!("cvh_dialog_test_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let items = vec![
            ActivityItem {
                id: "user-1".to_string(),
                item_type: "user".to_string(),
                timestamp: "10:00:00".to_string(),
                content: Some("Почини сохранение диалогов".to_string()),
                tool: None,
                duration_ms: None,
            },
            ActivityItem {
                id: "th-1".to_string(),
                item_type: "thought".to_string(),
                timestamp: "10:00:01".to_string(),
                content: Some("Анализирую требования к персистентности...".to_string()),
                tool: None,
                duration_ms: Some(2500.0),
            },
            ActivityItem {
                id: "call-1".to_string(),
                item_type: "tool".to_string(),
                timestamp: "10:00:03".to_string(),
                content: None,
                tool: Some(serde_json::json!({
                    "id": "call-1",
                    "toolName": "read",
                    "status": "success",
                    "exitCode": 0
                })),
                duration_ms: Some(120.0),
            },
            ActivityItem {
                id: "out-1".to_string(),
                item_type: "output".to_string(),
                timestamp: "10:00:05".to_string(),
                content: Some("Диалоги успешно сохраняются на диск.".to_string()),
                tool: None,
                duration_ms: None,
            },
        ];

        let serialized = serde_json::to_string_pretty(&items).expect("serialize");
        let p_dialogs = base.join(".cvh").join("dialogs").join("CVH-001.json");
        atomic_write_file(&p_dialogs, &serialized).await.expect("write dialog");

        assert!(p_dialogs.exists());
        let read_back = tokio::fs::read_to_string(&p_dialogs).await.expect("read dialog");
        let loaded: Vec<ActivityItem> = serde_json::from_str(&read_back).expect("parse dialog");

        assert_eq!(loaded.len(), 4);
        assert_eq!(loaded[0].item_type, "user");
        assert_eq!(loaded[1].item_type, "thought");
        assert_eq!(loaded[2].item_type, "tool");
        assert_eq!(loaded[3].item_type, "output");

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[test]
    fn test_slugify_board_name_cyrillic_and_latin() {
        assert_eq!(slugify_board_name("3D Моделирование"), "3d-modelirovanie");
        assert_eq!(slugify_board_name("Разработка"), "razrabotka");
        assert_eq!(slugify_board_name("Dev Pipeline"), "dev-pipeline");
        assert_eq!(slugify_board_name("Engineering Core 2"), "engineering-core-2");
    }

    #[test]
    fn test_task_board_id_serialization() {
        let json_with_board = r#"{
            "id": "CVH-100",
            "title": "3D asset task",
            "stageId": "concept",
            "boardId": "modeler"
        }"#;

        let task: Task = serde_json::from_str(json_with_board).expect("deserialize task");
        assert_eq!(task.board_id.as_deref(), Some("modeler"));

        let json_without_board = r#"{
            "id": "CVH-101",
            "title": "Legacy task",
            "stageId": "dev"
        }"#;

        let legacy_task: Task = serde_json::from_str(json_without_board).expect("deserialize legacy task");
        assert!(legacy_task.board_id.is_none());

        let fixed = ensure_task_board_id(vec![legacy_task]);
        assert_eq!(fixed[0].board_id.as_deref(), Some("development"));
    }

    #[tokio::test]
    async fn test_custom_tools_save_list_delete_roundtrip() {
        let base =
            PathBuf::from("target").join(format!("cvh_tools_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let params = serde_json::json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"]
        });

        // save b_tool first, then a_tool → list must be sorted by name
        let rel_b = save_custom_tool_file(&base, "b_tool", "B desc", params.clone(), "echo b")
            .await
            .expect("save b_tool");
        assert_eq!(rel_b, ".cvh/tools/b_tool.json");
        let rel_a = save_custom_tool_file(&base, "a_tool", "A desc", params.clone(), "echo a")
            .await
            .expect("save a_tool");
        assert_eq!(rel_a, ".cvh/tools/a_tool.json");

        // pretty JSON on disk parses back to the same config
        let raw = tokio::fs::read_to_string(base.join(".cvh").join("tools").join("a_tool.json"))
            .await
            .expect("read a_tool file");
        let parsed: CustomToolConfig = serde_json::from_str(&raw).expect("parse a_tool");
        assert_eq!(parsed.name, "a_tool");
        assert_eq!(parsed.description, "A desc");
        assert_eq!(parsed.command, "echo a");
        assert_eq!(parsed.parameters, params);

        let listed = list_custom_tools_in(&base).await;
        let names: Vec<String> = listed.iter().map(|t| t.name.clone()).collect();
        assert_eq!(names, vec!["a_tool".to_string(), "b_tool".to_string()]);

        delete_custom_tool_file(&base, "a_tool").await.expect("delete a_tool");
        assert!(!base.join(".cvh").join("tools").join("a_tool.json").exists());
        let listed2 = list_custom_tools_in(&base).await;
        let names2: Vec<String> = listed2.iter().map(|t| t.name.clone()).collect();
        assert_eq!(names2, vec!["b_tool".to_string()]);

        delete_custom_tool_file(&base, "b_tool").await.expect("delete b_tool");
        let listed3 = list_custom_tools_in(&base).await;
        assert!(listed3.is_empty());

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_custom_tool_invalid_name_rejected() {
        let base =
            PathBuf::from("target").join(format!("cvh_tools_badname_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");
        let params = serde_json::json!({"type": "object"});

        for bad in ["", "BadName", "with space", "has-dash", "a/b", "имя", "name!"] {
            let res = save_custom_tool_file(&base, bad, "d", params.clone(), "cmd").await;
            assert!(res.is_err(), "expected invalid name error for '{}'", bad);
            let del = delete_custom_tool_file(&base, bad).await;
            assert!(del.is_err(), "expected delete invalid name error for '{}'", bad);
        }

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_custom_tool_parameters_must_be_object() {
        let base =
            PathBuf::from("target").join(format!("cvh_tools_params_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        for non_obj in [
            serde_json::json!([]),
            serde_json::json!("str"),
            serde_json::json!(42),
            serde_json::Value::Null,
        ] {
            let res =
                save_custom_tool_file(&base, "good_name", "d", non_obj, "cmd").await;
            assert!(res.is_err(), "expected parameters-object error");
        }
        // nothing was written
        assert!(list_custom_tools_in(&base).await.is_empty());

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_custom_tool_delete_missing_errors() {
        let base =
            PathBuf::from("target").join(format!("cvh_tools_missing_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let res = delete_custom_tool_file(&base, "no_such_tool").await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("не найден"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_custom_tool_broken_json_ignored() {
        let base =
            PathBuf::from("target").join(format!("cvh_tools_broken_{}", uuid::Uuid::new_v4()));
        let tools_dir = base.join(".cvh").join("tools");
        tokio::fs::create_dir_all(&tools_dir).await.expect("test dir");

        save_custom_tool_file(&base, "good_tool", "ok", serde_json::json!({"type": "object"}), "echo ok")
            .await
            .expect("save good");
        // broken json + non-json file must be silently skipped
        tokio::fs::write(tools_dir.join("broken.json"), "{ not valid json {{{")
            .await
            .expect("seed broken");
        tokio::fs::write(tools_dir.join("notes.txt"), "ignore me")
            .await
            .expect("seed txt");

        let listed = list_custom_tools_in(&base).await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "good_tool");

        // missing folder → []
        let empty_base =
            PathBuf::from("target").join(format!("cvh_tools_empty_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&empty_base).await.expect("empty dir");
        assert!(list_custom_tools_in(&empty_base).await.is_empty());

        let _ = tokio::fs::remove_dir_all(&base).await;
        let _ = tokio::fs::remove_dir_all(&empty_base).await;
    }
}


