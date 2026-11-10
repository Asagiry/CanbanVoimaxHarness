//! Persona file operations and IPC command handlers.

use crate::commands::personas::meta::*;
use crate::commands::state::AppState;
use crate::config::atomic_write_file;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPersonaInfo {
    #[serde(alias = "persona")]
    pub role: String,
    pub has_prompt: bool,
    #[serde(default)]
    pub is_builtin: bool,
    #[serde(default)]
    pub persona_file: String,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub hooks: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub context_window: Option<u64>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider_id: Option<String>,
}

/// Сканирует subдиректории `<root>/.cvh/agents/<role>/` (СТРОГО директории, flat .md файлы игнорируются).
/// Проверяет наличие `persona.md` или `agent.md`.
/// Считывает правила из `<root>/.cvh/agents/<role>/rules/*.md` и хуки из `<root>/.cvh/agents/<role>/hooks/*`.
/// Всегда включает встроенную персону `cvh-dev` (is_builtin = true).
pub async fn list_agent_personas_in(root: &Path) -> Vec<AgentPersonaInfo> {
    // If .cvh/agents/cvh-dev/persona.md does not exist on disk, seed the cvh-dev scaffold
    if !root.join(".cvh").join("agents").join("cvh-dev").join("persona.md").is_file() {
        let _ = crate::config::ensure_cvh_dev_scaffold(root).await;
    }

    let agents_dir = root.join(".cvh").join("agents");
    let mut roles = HashSet::new();
    if let Ok(mut entries) = tokio::fs::read_dir(&agents_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            if let Some(role_name) = entry.file_name().to_str() {
                if !role_name.is_empty()
                    && is_valid_persona_role(role_name)
                    && (p.join("persona.md").is_file() || p.join("agent.md").is_file())
                {
                    roles.insert(role_name.to_string());
                }
            }
        }
    }
    // Always include the builtin cvh-dev agent
    roles.insert("cvh-dev".to_string());

    // Первый проход: собираем уже назначенные иконки
    let mut used_icons = HashSet::new();
    for r in &roles {
        let r_dir = agents_dir.join(r);
        if let Some(ic) = read_persona_icon_from_dir(&r_dir, r).await {
            used_icons.insert(ic);
        }
    }
    used_icons.insert("Cpu".to_string());

    let mut out: Vec<AgentPersonaInfo> = Vec::with_capacity(roles.len());
    for role in roles {
        let is_builtin = role == "cvh-dev";
        let role_dir = agents_dir.join(&role);

        let persona_file = if role_dir.join("persona.md").is_file() {
            "persona.md".to_string()
        } else if role_dir.join("agent.md").is_file() {
            "agent.md".to_string()
        } else {
            "persona.md".to_string()
        };

        let has_prompt = role_dir.join("persona.md").is_file()
            || role_dir.join("agent.md").is_file()
            || role == "cvh-dev"
            || role == "developer";

        let mut rules = Vec::new();
        let rules_dir = role_dir.join("rules");
        if let Ok(mut r_entries) = tokio::fs::read_dir(&rules_dir).await {
            while let Ok(Some(r_entry)) = r_entries.next_entry().await {
                let rp = r_entry.path();
                if rp.is_file() && rp.extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Some(name) = r_entry.file_name().to_str() {
                        rules.push(name.to_string());
                    }
                }
            }
        }
        rules.sort();

        let mut hooks = Vec::new();
        let hooks_dir = role_dir.join("hooks");
        if let Ok(mut h_entries) = tokio::fs::read_dir(&hooks_dir).await {
            while let Ok(Some(h_entry)) = h_entries.next_entry().await {
                let hp = h_entry.path();
                if hp.is_file() {
                    if let Some(name) = h_entry.file_name().to_str() {
                        hooks.push(name.to_string());
                    }
                }
            }
        }
        hooks.sort();

        // Считываем иконку или детерминированно назначаем и сохраняем
        let icon = match read_persona_icon_from_dir(&role_dir, &role).await {
            Some(ic) => ic,
            None => {
                let assigned = deterministic_icon_for_role(&role, &used_icons);
                used_icons.insert(assigned.clone());
                if role_dir.exists() {
                    let _ = save_persona_icon(&role_dir, &assigned).await;
                }
                assigned
            }
        };

        // Считываем цвет или детерминированно назначаем и сохраняем (исключая серый и зеленый)
        let color = match read_persona_color_from_dir(&role_dir, &role).await {
            Some(c) => {
                let norm = c.trim().to_lowercase();
                if FORBIDDEN_AGENT_COLORS.contains(&norm.as_str()) {
                    let assigned = deterministic_color_for_role(&role);
                    if role_dir.exists() {
                        let _ = save_persona_color(&role_dir, &assigned).await;
                    }
                    assigned
                } else {
                    c
                }
            }
            None => {
                let assigned = deterministic_color_for_role(&role);
                if role_dir.exists() {
                    let _ = save_persona_color(&role_dir, &assigned).await;
                }
                assigned
            }
        };

        let setting = crate::config::load_agent_setting(root, &role).await;
        let context_window = setting.context_window;
        let reasoning_effort =
            crate::config::normalize_reasoning_effort(setting.reasoning_effort.as_deref());

        out.push(AgentPersonaInfo {
            role,
            has_prompt,
            is_builtin,
            persona_file,
            rules,
            hooks,
            icon: Some(icon),
            color: Some(color),
            context_window,
            reasoning_effort,
            model: setting.model,
            provider_id: setting.provider_id,
        });
    }
    out.sort_by(|a, b| a.role.cmp(&b.role));
    out
}

/// Удаляет персону: каталог `.cvh/agents/<role>`.
/// Удаление встроенной персоны `cvh-dev` строго запрещено.
/// При удалении сбрасывает ссылки на персону у колонок в boards.json.
pub async fn delete_agent_persona_file(root: &Path, role: &str) -> Result<(), String> {
    if role == "cvh-dev" {
        return Err("Запрещено удалять встроенную персону 'cvh-dev'".to_string());
    }
    let role_dir = crate::config::agent_dir(root, role);
    let flat = root.join(".cvh").join("agents").join(format!("{}.md", role));
    let mut deleted = false;
    if role_dir.exists() {
        tokio::fs::remove_dir_all(&role_dir)
            .await
            .map_err(|e| format!("Не удалось удалить каталог персоны '{}': {}", role, e))?;
        deleted = true;
    }
    if flat.exists() {
        let _ = tokio::fs::remove_file(&flat).await;
        deleted = true;
    }
    if !deleted {
        return Err(format!("Персона '{}' не найдена", role));
    }

    // Сброс ссылок на persona в boards.json с возвратом дефолтной иконки стадии
    if let Ok(mut boards_cfg) = crate::config::load_boards_config(root).await {
        let mut changed = false;
        for board in &mut boards_cfg.boards {
            for col in &mut board.stages {
                if col.persona.as_deref() == Some(role) {
                    col.persona = None;
                    col.icon = Some(default_stage_icon(&col.id));
                    changed = true;
                }
            }
        }
        if changed {
            let _ = crate::config::save_boards_config(root, &boards_cfg).await;
        }
    }

    Ok(())
}

/// Переименовывает персону из `old_role` в `new_role`:
/// 1. Валидирует `new_role` через `is_valid_persona_role`.
/// 2. Запрещает переименование `cvh-dev`.
/// 3. Переименовывает директорию `.cvh/agents/<old_role>` в `.cvh/agents/<new_role>` (и flat-файл, если был).
/// 4. Обновляет ссылки на persona в boards.json с old_role на new_role.
pub async fn rename_agent_persona_file(root: &Path, old_role: &str, new_role: &str) -> Result<(), String> {
    if old_role == "cvh-dev" {
        return Err("Запрещено переименовывать встроенную персону 'cvh-dev'".to_string());
    }
    if !is_valid_persona_role(new_role) {
        return Err(format!(
            "Недопустимое имя роли '{}': разрешены только строчные латинские буквы, цифры и дефис (шаблон ^[a-z0-9][a-z0-9-]*$)",
            new_role
        ));
    }
    if old_role == new_role {
        return Ok(());
    }

    let old_dir = crate::config::agent_dir(root, old_role);
    let old_flat = root.join(".cvh").join("agents").join(format!("{}.md", old_role));
    if !old_dir.exists() && !old_flat.exists() {
        return Err(format!("Персона '{}' не найдена", old_role));
    }

    let new_dir = crate::config::agent_dir(root, new_role);
    let new_flat = root.join(".cvh").join("agents").join(format!("{}.md", new_role));
    if new_dir.exists() || new_flat.exists() {
        return Err(format!("Персона '{}' уже существует", new_role));
    }

    // Переименование каталога или файла
    if old_dir.exists() {
        tokio::fs::rename(&old_dir, &new_dir)
            .await
            .map_err(|e| format!("Не удалось переименовать каталог персоны '{}' в '{}': {}", old_role, new_role, e))?;
    }
    if old_flat.exists() {
        tokio::fs::rename(&old_flat, &new_flat)
            .await
            .map_err(|e| format!("Не удалось переименовать файл персоны '{}.md' в '{}.md': {}", old_role, new_role, e))?;
    }

    // Обновление ссылок на persona в boards.json
    if let Ok(mut boards_cfg) = crate::config::load_boards_config(root).await {
        let mut changed = false;
        for board in &mut boards_cfg.boards {
            for col in &mut board.stages {
                if col.persona.as_deref() == Some(old_role) {
                    col.persona = Some(new_role.to_string());
                    changed = true;
                }
            }
        }
        if changed {
            let _ = crate::config::save_boards_config(root, &boards_cfg).await;
        }
    }

    Ok(())
}

/// Создаёт директорию `.cvh/agents/<role>/` с `persona.md`,
/// пустыми per-role aux-директориями `rules/`, `hooks/` и `setting` (`{}`).
/// Возвращает относительный путь `.cvh/agents/<role>/persona.md`.
pub async fn create_agent_persona_file(root: &Path, role: &str) -> Result<String, String> {
    if !is_valid_persona_role(role) {
        return Err(format!(
            "Недопустимое имя роли '{}': разрешены только строчные латинские буквы, цифры и дефис (шаблон ^[a-z0-9][a-z0-9-]*$)",
            role
        ));
    }
    let role_dir = crate::config::agent_dir(root, role);
    let persona_file = role_dir.join("persona.md");
    let agent_file = role_dir.join("agent.md");
    if persona_file.exists() || agent_file.exists() {
        return Err(format!(
            "Персона '{}' уже существует: .cvh/agents/{}/persona.md",
            role, role
        ));
    }

    // Собираем уже занятые иконки у существующих персон
    let mut used_icons = HashSet::new();
    let agents_dir = root.join(".cvh").join("agents");
    if let Ok(mut entries) = tokio::fs::read_dir(&agents_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let p = entry.path();
            if p.is_dir() {
                if let Some(r_name) = entry.file_name().to_str() {
                    if let Some(ic) = read_persona_icon_from_dir(&p, r_name).await {
                        used_icons.insert(ic);
                    }
                }
            }
        }
    }
    used_icons.insert("Cpu".to_string());

    // Выбираем случайную иконку из свободных (или из всех, если свободных не осталось)
    let available: Vec<&'static str> = AGENT_ICONS
        .iter()
        .copied()
        .filter(|&icon| !used_icons.contains(icon))
        .collect();
    let pool = if !available.is_empty() {
        &available[..]
    } else {
        AGENT_ICONS
    };
    let chosen_icon = pool[(uuid::Uuid::new_v4().as_u128() as usize) % pool.len()];
    let chosen_color = deterministic_color_for_role(role);

    tokio::fs::create_dir_all(role_dir.join("rules"))
        .await
        .map_err(|e| format!("Не удалось создать rules для персоны '{}': {}", role, e))?;
    tokio::fs::create_dir_all(role_dir.join("hooks"))
        .await
        .map_err(|e| format!("Не удалось создать hooks для персоны '{}': {}", role, e))?;
    let content = persona_placeholder_template(role, chosen_icon, &chosen_color);
    atomic_write_file(&persona_file, &content)
        .await
        .map_err(|e| format!("Не удалось создать персону '{}': {}", role, e))?;
    let setting_path = role_dir.join("setting");
    if !setting_path.exists() {
        let setting_obj = serde_json::json!({
            "model": "gemini-3.8-flash-high",
            "context_window": 200000,
            "temperature": 0.2,
            "max_tokens": 8192,
            "icon": chosen_icon,
            "color": chosen_color,
        });
        let pretty = serde_json::to_string_pretty(&setting_obj)
            .unwrap_or_else(|_| crate::config::DEFAULT_AGENT_SETTING_JSON.to_string());
        atomic_write_file(&setting_path, &pretty)
            .await
            .map_err(|e| format!("Не удалось создать setting для персоны '{}': {}", role, e))?;
    }
    Ok(format!(".cvh/agents/{}/persona.md", role))
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
    app: AppHandle,
) -> Result<String, String> {
    let root = state.get_root();
    let res = create_agent_persona_file(&root, &role).await?;
    let list = list_agent_personas_in(&root).await;
    let _ = app.emit("agent:personas_updated", &list);
    Ok(res)
}

#[tauri::command]
pub async fn delete_agent_persona(
    role: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let root = state.get_root();
    delete_agent_persona_file(&root, &role).await?;
    let list = list_agent_personas_in(&root).await;
    let _ = app.emit("agent:personas_updated", &list);
    if let Ok(bc) = load_boards_config(&root).await {
        let _ = app.emit(
            "boards:updated",
            serde_json::json!({ "boards": &bc.boards, "activeBoardId": &bc.active_board_id }),
        );
    }
    if let Ok(pipe) = load_pipeline_config(&root).await {
        let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
    }
    Ok(())
}

#[tauri::command]
pub async fn rename_agent_persona(
    old_role: String,
    new_role: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let root = state.get_root();
    rename_agent_persona_file(&root, &old_role, &new_role).await?;
    let list = list_agent_personas_in(&root).await;
    let _ = app.emit("agent:personas_updated", &list);
    if let Ok(bc) = load_boards_config(&root).await {
        let _ = app.emit(
            "boards:updated",
            serde_json::json!({ "boards": &bc.boards, "activeBoardId": &bc.active_board_id }),
        );
    }
    if let Ok(pipe) = load_pipeline_config(&root).await {
        let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
    }
    Ok(())
}

#[tauri::command]
pub async fn set_agent_persona_color(
    role: String,
    color: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Vec<AgentPersonaInfo>, String> {
    if !is_valid_persona_role(&role) {
        return Err(format!("Некорректное имя роли персоны '{}'", role));
    }
    let norm_color = color.trim().to_lowercase();
    if FORBIDDEN_AGENT_COLORS.contains(&norm_color.as_str()) {
        return Err("Серый и зеленый цвета зарезервированы за Backlog и Ready и запрещены для агентов".to_string());
    }
    let root = state.get_root();
    let role_dir = crate::config::agent_dir(&root, &role);
    save_persona_color(&role_dir, &color).await?;
    let list = list_agent_personas_in(&root).await;
    let _ = app.emit("agent:personas_updated", &list);
    Ok(list)
}

#[tauri::command]
pub async fn set_agent_persona_icon(
    role: String,
    icon: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Vec<AgentPersonaInfo>, String> {
    if !is_valid_persona_role(&role) {
        return Err(format!("Некорректное имя роли персоны '{}'", role));
    }
    let root = state.get_root();
    let role_dir = crate::config::agent_dir(&root, &role);
    save_persona_icon(&role_dir, &icon).await?;
    let list = list_agent_personas_in(&root).await;
    let _ = app.emit("agent:personas_updated", &list);
    Ok(list)
}

// Helpers needed in crud
use crate::config::{load_boards_config, load_pipeline_config};
