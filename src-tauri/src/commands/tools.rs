use crate::agent::CustomToolConfig;
use crate::commands::state::AppState;
use crate::config::atomic_write_file;
use std::path::Path;
use tauri::State;

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
