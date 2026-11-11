use crate::config::{
    get_board_template_stages, load_boards_config,
    load_pipeline_config, normalize_pipeline_stages,
    save_boards_config, save_pipeline_config, BoardDef, PipelineConfig,
};
use crate::commands::state::AppState;
use crate::commands::tasks::{heal_tasks, save_tasks_to_disk, try_load_tasks_from_disk};
use std::sync::LazyLock;
use tauri::{AppHandle, Emitter, State};

static BOARDS_MUTEX: LazyLock<tokio::sync::Mutex<()>> = LazyLock::new(|| tokio::sync::Mutex::new(()));

pub use crate::commands::personas::*;

impl PartialEq for crate::config::models::BoardsConfig {
    fn eq(&self, other: &Self) -> bool {
        self.active_board_id == other.active_board_id && self.boards == other.boards
    }
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
        } else if (lower == ' ' || lower == '-' || lower == '_')
            && !slug.ends_with('-')
            && !slug.is_empty()
        {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        format!("board-{}", &uuid::Uuid::new_v4().to_string()[..6])
    } else {
        trimmed
    }
}

// ============================================================================
// IPC Commands: Boards & Pipelines
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
    app: AppHandle,
) -> Result<(), String> {
    let _boards_guard = BOARDS_MUTEX.lock().await;
    pipeline.stages = normalize_pipeline_stages(pipeline.stages)?;
    let root = state.get_root();
    save_pipeline_config(&root, &pipeline)
        .await
        .map_err(|e| format!("Failed to save pipeline: {}", e))?;

    if let Err(e) = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipeline })) {
        eprintln!("CVH [WARN]: Failed to emit pipeline:updated: {}", e);
    }
    let boards_cfg = load_boards_config(&root)
        .await
        .map_err(|e| format!("Failed to reload boards after pipeline save: {}", e))?;
    if let Err(e) = app.emit(
        "boards:updated",
        serde_json::json!({ "boards": &boards_cfg.boards, "activeBoardId": &boards_cfg.active_board_id }),
    ) {
        eprintln!("CVH [WARN]: Failed to emit boards:updated: {}", e);
    }
    let mut tasks = try_load_tasks_from_disk(&root).await?;
    let modified = heal_tasks(&mut tasks, &boards_cfg);
    if modified {
        {
            let mut guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
            *guard = tasks.clone();
        }
        save_tasks_to_disk(&root, &tasks).await?;
    }
    if let Err(e) = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks })) {
        eprintln!("CVH [WARN]: Failed to emit tasks:updated: {}", e);
    }
    Ok(())
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
    app: AppHandle,
) -> Result<BoardDef, String> {
    let _boards_guard = BOARDS_MUTEX.lock().await;
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
    let raw_stages = get_board_template_stages(tmpl);
    let stages = normalize_pipeline_stages(raw_stages)?;

    let new_board = BoardDef {
        id: unique_id.clone(),
        name: trimmed_name.to_string(),
        stages,
    };

    config.boards.push(new_board.clone());
    config.active_board_id = unique_id;

    save_boards_config(&root, &config)
        .await
        .map_err(|e| format!("Failed to save new board: {}", e))?;

    if let Err(e) = app.emit(
        "boards:updated",
        serde_json::json!({ "boards": &config.boards, "activeBoardId": &config.active_board_id }),
    ) {
        eprintln!("CVH [WARN]: Failed to emit boards:updated: {}", e);
    }
    if let Ok(pipe) = load_pipeline_config(&root).await {
        if let Err(e) = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe })) {
            eprintln!("CVH [WARN]: Failed to emit pipeline:updated: {}", e);
        }
    }

    let mut tasks = try_load_tasks_from_disk(&root).await?;
    let modified = heal_tasks(&mut tasks, &config);
    if modified {
        {
            let mut guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
            *guard = tasks.clone();
        }
        save_tasks_to_disk(&root, &tasks).await?;
    }
    if let Err(e) = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks })) {
        eprintln!("CVH [WARN]: Failed to emit tasks:updated: {}", e);
    }

    Ok(new_board)
}

#[tauri::command]
pub async fn save_board(
    mut board: BoardDef,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let _boards_guard = BOARDS_MUTEX.lock().await;
    let trimmed_name = board.name.trim();
    if trimmed_name.is_empty() {
        return Err("Название доски не может быть пустым".to_string());
    }
    board.name = trimmed_name.to_string();
    board.stages = normalize_pipeline_stages(board.stages)?;

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

    if let Err(e) = app.emit(
        "boards:updated",
        serde_json::json!({ "boards": &config.boards, "activeBoardId": &config.active_board_id }),
    ) {
        eprintln!("CVH [WARN]: Failed to emit boards:updated: {}", e);
    }
    if let Ok(pipe) = load_pipeline_config(&root).await {
        if let Err(e) = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe })) {
            eprintln!("CVH [WARN]: Failed to emit pipeline:updated: {}", e);
        }
    }

    // Verify and heal all tasks:
    // Any task on a deleted/renamed board must be mapped to config.active_board_id,
    // and any task in a removed column mapped to the board's first column (backlog).
    let mut tasks = try_load_tasks_from_disk(&root).await?;
    let modified = heal_tasks(&mut tasks, &config);
    if modified {
        {
            let mut guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
            *guard = tasks.clone();
        }
        save_tasks_to_disk(&root, &tasks).await?;
    }
    // Emit both boards:updated and tasks:updated so the UI immediately updates!
    if let Err(e) = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks })) {
        eprintln!("CVH [WARN]: Failed to emit tasks:updated: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_board(
    board_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let _boards_guard = BOARDS_MUTEX.lock().await;
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

    let target_board = &config.boards[pos];
    if target_board.id == "cvh-board"
        || target_board.id == "development"
        || target_board.name.to_lowercase() == "cvh-board"
    {
        return Err("Нельзя удалить защищенную изначальную доску 'cvh-board'".to_string());
    }

    config.boards.remove(pos);

    if config.active_board_id == board_id {
        config.active_board_id = config.boards[0].id.clone();
    }

    save_boards_config(&root, &config)
        .await
        .map_err(|e| format!("Failed to save boards after deletion: {}", e))?;

    if let Err(e) = app.emit(
        "boards:updated",
        serde_json::json!({ "boards": &config.boards, "activeBoardId": &config.active_board_id }),
    ) {
        eprintln!("CVH [WARN]: Failed to emit boards:updated: {}", e);
    }
    if let Ok(pipe) = load_pipeline_config(&root).await {
        if let Err(e) = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe })) {
            eprintln!("CVH [WARN]: Failed to emit pipeline:updated: {}", e);
        }
    }

    // Verify and heal all tasks:
    // Any task on the deleted board must be mapped to config.active_board_id,
    // and any task in a removed column mapped to the target board's first column (backlog).
    let mut tasks = try_load_tasks_from_disk(&root).await?;
    let _modified = heal_tasks(&mut tasks, &config);
    {
        let mut guard = state.tasks.lock().unwrap_or_else(|p| p.into_inner());
        *guard = tasks.clone();
    }
    save_tasks_to_disk(&root, &tasks).await?;
    if let Err(e) = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks })) {
        eprintln!("CVH [WARN]: Failed to emit tasks:updated: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub async fn set_active_board(
    board_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let _boards_guard = BOARDS_MUTEX.lock().await;
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

    let _ = app.emit(
        "boards:updated",
        serde_json::json!({ "boards": &config.boards, "activeBoardId": &config.active_board_id }),
    );
    if let Ok(pipe) = load_pipeline_config(&root).await {
        let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
    }

    Ok(())
}
