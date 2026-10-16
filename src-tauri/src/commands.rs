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

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
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

pub use crate::config::normalize_pipeline_columns;

/// Канонический пак имен SVG-иконок для агентов
pub const AGENT_ICONS: &[&str] = &[
    "Bot", "Brain", "Cpu", "Code2", "ShieldCheck", "Sparkles",
    "Terminal", "Zap", "Flame", "Radar", "Workflow", "Boxes",
    "Wrench", "Rocket", "Fingerprint", "Compass", "Crosshair", "Layers",
];

/// Каноническая палитра цветов для агентов.
/// Серый (#64748b, #6b7280) и зеленый (#10b981, #22c55e) исключены,
/// так как они навсегда зарезервированы за Backlog и Ready.
pub const AGENT_PALETTE: &[&str] = &[
    "#f59e0b", "#a855f7", "#0284c7", "#ef4444", "#6366f1",
    "#14b8a6", "#ec4899", "#f97316", "#06b6d4", "#8b5cf6",
];

/// Цвета, навсегда зарезервированные за Backlog (#64748b) и Ready (#10b981).
pub const FORBIDDEN_AGENT_COLORS: &[&str] = &[
    "#64748b", "#6b7280", "#10b981", "#22c55e",
];

/// Детерминированный выбор цвета для роли агента из AGENT_PALETTE.
pub fn deterministic_color_for_role(role: &str) -> String {
    let trimmed = role.trim();
    if trimmed == "cvh-dev" {
        return "#6366f1".to_string();
    }
    if trimmed == "developer" || trimmed == "dev" {
        return "#0284c7".to_string();
    }
    if trimmed == "reviewer" || trimmed == "review" {
        return "#a855f7".to_string();
    }
    if trimmed == "retro" {
        return "#f59e0b".to_string();
    }
    let hash: usize = trimmed.bytes().fold(5381usize, |acc, b| {
        acc.wrapping_mul(33).wrapping_add(b as usize)
    });
    AGENT_PALETTE[hash % AGENT_PALETTE.len()].to_string()
}

/// Дефолтная иконка для колонки по её идентификатору
pub fn default_column_icon(col_id: &str) -> String {
    match col_id {
        "backlog" => "Inbox".to_string(),
        "dev" => "Code2".to_string(),
        "review" => "ShieldCheck".to_string(),
        "retro" => "Sparkles".to_string(),
        "ready" => "CheckCircle2".to_string(),
        _ => "Layers".to_string(),
    }
}

/// Извлекает иконку из YAML frontmatter (между первыми `---` и вторыми `---`)
pub fn extract_persona_icon_from_text(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    let rest = trimmed.strip_prefix("---")?;
    let end_idx = rest.find("\n---")?;
    let fm_block = &rest[..end_idx];
    for line in fm_block.lines() {
        let trimmed_line = line.trim();
        if let Some(val) = trimmed_line.strip_prefix("icon:") {
            let cleaned = val.trim().trim_matches(|c| c == '\'' || c == '"').trim();
            if !cleaned.is_empty() {
                return Some(cleaned.to_string());
            }
        }
    }
    None
}

/// Извлекает иконку из JSON файла (например, persona.json или setting)
pub fn extract_persona_icon_from_json(content: &str) -> Option<String> {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(icon) = val.get("icon").and_then(|v| v.as_str()) {
            let cleaned = icon.trim();
            if !cleaned.is_empty() {
                return Some(cleaned.to_string());
            }
        }
    }
    None
}

/// Считывает иконку персоны из каталога (persona.md -> agent.md -> persona.json -> setting).
/// Для встроенного агента `cvh-dev` по умолчанию возвращает "Cpu".
pub async fn read_persona_icon_from_dir(role_dir: &Path, role: &str) -> Option<String> {
    let persona_file = role_dir.join("persona.md");
    if let Ok(content) = tokio::fs::read_to_string(&persona_file).await {
        if let Some(icon) = extract_persona_icon_from_text(&content) {
            return Some(icon);
        }
    }
    let agent_file = role_dir.join("agent.md");
    if let Ok(content) = tokio::fs::read_to_string(&agent_file).await {
        if let Some(icon) = extract_persona_icon_from_text(&content) {
            return Some(icon);
        }
    }
    let json_file = role_dir.join("persona.json");
    if let Ok(content) = tokio::fs::read_to_string(&json_file).await {
        if let Some(icon) = extract_persona_icon_from_json(&content) {
            return Some(icon);
        }
    }
    let setting_file = role_dir.join("setting");
    if let Ok(content) = tokio::fs::read_to_string(&setting_file).await {
        if let Some(icon) = extract_persona_icon_from_json(&content) {
            return Some(icon);
        }
    }
    if role == "cvh-dev" {
        return Some("Cpu".to_string());
    }
    None
}

/// Вставляет или обновляет `icon: <icon>` в YAML frontmatter текста.
pub fn insert_or_update_frontmatter_icon(content: &str, icon: &str) -> String {
    let trimmed_start = content.trim_start();
    if let Some(rest) = trimmed_start.strip_prefix("---") {
        if let Some(end_idx) = rest.find("\n---") {
            let fm_block = &rest[..end_idx];
            let after_closing_dash = &rest[end_idx..]; // начинается с "\n---"
            let mut found = false;
            let mut new_lines = Vec::new();
            for line in fm_block.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("icon:") {
                    new_lines.push(format!("icon: {}", icon));
                    found = true;
                } else {
                    new_lines.push(line.to_string());
                }
            }
            if !found {
                new_lines.push(format!("icon: {}", icon));
            }
            let leading_ws = &content[..content.len() - trimmed_start.len()];
            return format!("{}---\n{}{}", leading_ws, new_lines.join("\n"), after_closing_dash);
        }
    }
    format!("---\nicon: {}\n---\n\n{}", icon, content)
}

/// Сохраняет иконку персоны в `persona.md` (или `agent.md`).
pub async fn save_persona_icon(role_dir: &Path, icon: &str) -> Result<(), String> {
    let persona_file = role_dir.join("persona.md");
    let agent_file = role_dir.join("agent.md");
    let target_file = if persona_file.exists() {
        persona_file
    } else if agent_file.exists() {
        agent_file
    } else {
        persona_file
    };

    if target_file.exists() {
        let content = tokio::fs::read_to_string(&target_file)
            .await
            .map_err(|e| format!("Не удалось прочитать {}: {}", target_file.display(), e))?;
        let updated = insert_or_update_frontmatter_icon(&content, icon);
        atomic_write_file(&target_file, &updated)
            .await
            .map_err(|e| format!("Не удалось записать {}: {}", target_file.display(), e))?;
    } else {
        tokio::fs::create_dir_all(role_dir)
            .await
            .map_err(|e| format!("Не удалось создать каталог {}: {}", role_dir.display(), e))?;
        let new_content = format!("---\nicon: {}\n---\n", icon);
        atomic_write_file(&target_file, &new_content)
            .await
            .map_err(|e| format!("Не удалось записать {}: {}", target_file.display(), e))?;
    }
    Ok(())
}

/// Извлекает цвет из YAML frontmatter (между первыми `---` и вторыми `---`)
pub fn extract_persona_color_from_text(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    let rest = trimmed.strip_prefix("---")?;
    let end_idx = rest.find("\n---")?;
    let fm_block = &rest[..end_idx];
    for line in fm_block.lines() {
        let trimmed_line = line.trim();
        if let Some(val) = trimmed_line.strip_prefix("color:") {
            let cleaned = val.trim().trim_matches(|c| c == '\'' || c == '"').trim();
            if !cleaned.is_empty() {
                return Some(cleaned.to_string());
            }
        }
    }
    None
}

/// Извлекает цвет из JSON файла (например, persona.json или setting)
pub fn extract_persona_color_from_json(content: &str) -> Option<String> {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(color) = val.get("color").and_then(|v| v.as_str()) {
            let cleaned = color.trim();
            if !cleaned.is_empty() {
                return Some(cleaned.to_string());
            }
        }
    }
    None
}

/// Считывает цвет персоны из каталога (persona.md -> agent.md -> persona.json -> setting).
/// Для встроенного агента `cvh-dev` по умолчанию возвращает "#6366f1".
pub async fn read_persona_color_from_dir(role_dir: &Path, role: &str) -> Option<String> {
    let persona_file = role_dir.join("persona.md");
    if let Ok(content) = tokio::fs::read_to_string(&persona_file).await {
        if let Some(color) = extract_persona_color_from_text(&content) {
            return Some(color);
        }
    }
    let agent_file = role_dir.join("agent.md");
    if let Ok(content) = tokio::fs::read_to_string(&agent_file).await {
        if let Some(color) = extract_persona_color_from_text(&content) {
            return Some(color);
        }
    }
    let json_file = role_dir.join("persona.json");
    if let Ok(content) = tokio::fs::read_to_string(&json_file).await {
        if let Some(color) = extract_persona_color_from_json(&content) {
            return Some(color);
        }
    }
    let setting_file = role_dir.join("setting");
    if let Ok(content) = tokio::fs::read_to_string(&setting_file).await {
        if let Some(color) = extract_persona_color_from_json(&content) {
            return Some(color);
        }
    }
    if role == "cvh-dev" {
        return Some("#6366f1".to_string());
    }
    None
}

/// Вставляет или обновляет `color: <color>` в YAML frontmatter текста.
pub fn insert_or_update_frontmatter_color(content: &str, color: &str) -> String {
    let trimmed_start = content.trim_start();
    if let Some(rest) = trimmed_start.strip_prefix("---") {
        if let Some(end_idx) = rest.find("\n---") {
            let fm_block = &rest[..end_idx];
            let after_closing_dash = &rest[end_idx..]; // начинается с "\n---"
            let mut found = false;
            let mut new_lines = Vec::new();
            for line in fm_block.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("color:") {
                    new_lines.push(format!("color: {}", color));
                    found = true;
                } else {
                    new_lines.push(line.to_string());
                }
            }
            if !found {
                new_lines.push(format!("color: {}", color));
            }
            let leading_ws = &content[..content.len() - trimmed_start.len()];
            return format!("{}---\n{}{}", leading_ws, new_lines.join("\n"), after_closing_dash);
        }
    }
    format!("---\ncolor: {}\n---\n\n{}", color, content)
}

/// Сохраняет цвет персоны в `persona.md` (или `agent.md`).
pub async fn save_persona_color(role_dir: &Path, color: &str) -> Result<(), String> {
    let persona_file = role_dir.join("persona.md");
    let agent_file = role_dir.join("agent.md");
    let target_file = if persona_file.exists() {
        persona_file
    } else if agent_file.exists() {
        agent_file
    } else {
        persona_file
    };

    if target_file.exists() {
        let content = tokio::fs::read_to_string(&target_file)
            .await
            .map_err(|e| format!("Не удалось прочитать {}: {}", target_file.display(), e))?;
        let updated = insert_or_update_frontmatter_color(&content, color);
        atomic_write_file(&target_file, &updated)
            .await
            .map_err(|e| format!("Не удалось записать {}: {}", target_file.display(), e))?;
    } else {
        tokio::fs::create_dir_all(role_dir)
            .await
            .map_err(|e| format!("Не удалось создать каталог {}: {}", role_dir.display(), e))?;
        let new_content = format!("---\ncolor: {}\n---\n", color);
        atomic_write_file(&target_file, &new_content)
            .await
            .map_err(|e| format!("Не удалось записать {}: {}", target_file.display(), e))?;
    }
    Ok(())
}

/// Детерминированный выбор иконки для роли из пака AGENT_ICONS (предпочтительно не занятой)
pub fn deterministic_icon_for_role(role: &str, used_icons: &HashSet<String>) -> String {
    if role == "cvh-dev" {
        return "Cpu".to_string();
    }
    let unused: Vec<&'static str> = AGENT_ICONS
        .iter()
        .copied()
        .filter(|&icon| !used_icons.contains(icon))
        .collect();
    let pool = if !unused.is_empty() {
        &unused[..]
    } else {
        AGENT_ICONS
    };
    let hash: usize = role.bytes().fold(5381usize, |acc, b| {
        acc.wrapping_mul(33).wrapping_add(b as usize)
    });
    pool[hash % pool.len()].to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPersonaInfo {
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CortexDiskFile {
    pub name: String,
    pub path: String,
    pub file_type: String,
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

fn persona_placeholder_template(role: &str, icon: &str, color: &str) -> String {
    format!(
        "---\nname: {role}\nicon: {icon}\ncolor: {color}\n---\n\n# Роль: {role}\n\n> Этот файл создан автоматически командой `create_agent_persona`. Заполните его под свою персону — все секции ниже помечены TODO.\n\n## TODO: цель персоны\n\n- Опишите, за что отвечает эта персона и какие задачи решает.\n\n## TODO: стиль\n\n- Опишите стиль общения и работы: тон, детальность, формат ответов.\n\n## TODO: запреты\n\n- Перечислите, что персоне запрещено делать.\n\n## TODO: чеклист настройки\n\n- [ ] Заполнена цель персоны\n- [ ] Описан стиль\n- [ ] Перечислены запреты\n- [ ] Проверена работа персоны на тестовой задаче\n"
    )
}

/// Сканирует subдиректории `<root>/.cvh/agents/<role>/` (СТРОГО директории, flat .md файлы игнорируются).
/// Проверяет наличие `persona.md` или `agent.md`.
/// Считывает правила из `<root>/.cvh/agents/<role>/rules/*.md` и хуки из `<root>/.cvh/agents/<role>/hooks/*`.
/// Всегда включает встроенную персону `cvh-dev` (is_builtin = true).
pub async fn list_agent_personas_in(root: &Path) -> Vec<AgentPersonaInfo> {
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

        out.push(AgentPersonaInfo {
            role,
            has_prompt,
            is_builtin,
            persona_file,
            rules,
            hooks,
            icon: Some(icon),
            color: Some(color),
        });
    }
    out.sort_by(|a, b| a.role.cmp(&b.role));
    out
}

/// Удаляет персону: каталог `.cvh/agents/<role>`.
/// Удаление встроенной персоны `cvh-dev` строго запрещено.
/// При удалении сбрасывает ссылки на персону у колонок в boards.json и pipeline.json.
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

    // Сброс ссылок на persona в boards.json и pipeline.json с возвратом дефолтной иконки колонки
    if let Ok(mut boards_cfg) = crate::config::load_boards_config(root).await {
        let mut changed = false;
        for board in &mut boards_cfg.boards {
            for col in &mut board.columns {
                if col.persona.as_deref() == Some(role) {
                    col.persona = None;
                    col.icon = Some(default_column_icon(&col.id));
                    changed = true;
                }
            }
        }
        if changed {
            let _ = crate::config::save_boards_config(root, &boards_cfg).await;
        }
    }
    if let Ok(mut pipeline_cfg) = crate::config::load_pipeline_config(root).await {
        let mut changed = false;
        for col in &mut pipeline_cfg.columns {
            if col.persona.as_deref() == Some(role) {
                col.persona = None;
                col.icon = Some(default_column_icon(&col.id));
                changed = true;
            }
        }
        if let Some(boards) = &mut pipeline_cfg.boards {
            for board in boards {
                for col in &mut board.columns {
                    if col.persona.as_deref() == Some(role) {
                        col.persona = None;
                        col.icon = Some(default_column_icon(&col.id));
                        changed = true;
                    }
                }
            }
        }
        if changed {
            let _ = crate::config::save_pipeline_config(root, &pipeline_cfg).await;
        }
    }

    Ok(())
}

/// Переименовывает персону из `old_role` в `new_role`:
/// 1. Валидирует `new_role` через `is_valid_persona_role`.
/// 2. Запрещает переименование `cvh-dev`.
/// 3. Переименовывает директорию `.cvh/agents/<old_role>` в `.cvh/agents/<new_role>` (и flat-файл, если был).
/// 4. Обновляет ссылки на persona в boards.json и pipeline.json с old_role на new_role.
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

    // Обновление ссылок на persona в boards.json и pipeline.json
    if let Ok(mut boards_cfg) = crate::config::load_boards_config(root).await {
        let mut changed = false;
        for board in &mut boards_cfg.boards {
            for col in &mut board.columns {
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
    if let Ok(mut pipeline_cfg) = crate::config::load_pipeline_config(root).await {
        let mut changed = false;
        for col in &mut pipeline_cfg.columns {
            if col.persona.as_deref() == Some(old_role) {
                col.persona = Some(new_role.to_string());
                changed = true;
            }
        }
        if let Some(boards) = &mut pipeline_cfg.boards {
            for board in boards {
                for col in &mut board.columns {
                    if col.persona.as_deref() == Some(old_role) {
                        col.persona = Some(new_role.to_string());
                        changed = true;
                    }
                }
            }
        }
        if changed {
            let _ = crate::config::save_pipeline_config(root, &pipeline_cfg).await;
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
        atomic_write_file(&setting_path, crate::config::DEFAULT_AGENT_SETTING_JSON)
            .await
            .map_err(|e| format!("Не удалось создать setting для персоны '{}': {}", role, e))?;
    }
    Ok(format!(".cvh/agents/{}/persona.md", role))
}

#[derive(Clone)]
pub struct ActiveAgentHandle {
    pub abort_flag: Arc<AtomicBool>,
    pub join_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

pub struct AppState {
    pub repo_root: RwLock<PathBuf>,
    pub active_agents: Mutex<HashMap<String, ActiveAgentHandle>>,
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

pub async fn load_tasks_from_disk(repo_root: &Path) -> Vec<Task> {
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

pub async fn save_tasks_to_disk(repo_root: &Path, tasks: &[Task]) -> Result<(), String> {
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
    app: AppHandle,
) -> Result<(), String> {
    pipeline.columns = normalize_pipeline_columns(pipeline.columns)?;
    let root = state.get_root();
    save_pipeline_config(&root, &pipeline)
        .await
        .map_err(|e| format!("Failed to save pipeline: {}", e))?;

    let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipeline }));
    if let Ok(boards_cfg) = load_boards_config(&root).await {
        let _ = app.emit(
            "boards:updated",
            serde_json::json!({ "boards": &boards_cfg.boards, "activeBoardId": &boards_cfg.active_board_id }),
        );
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

    let _ = app.emit(
        "boards:updated",
        serde_json::json!({ "boards": &config.boards, "activeBoardId": &config.active_board_id }),
    );
    if let Ok(pipe) = load_pipeline_config(&root).await {
        let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
    }

    Ok(new_board)
}

#[tauri::command]
pub async fn save_board(
    mut board: BoardDef,
    state: State<'_, AppState>,
    app: AppHandle,
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

    let _ = app.emit(
        "boards:updated",
        serde_json::json!({ "boards": &config.boards, "activeBoardId": &config.active_board_id }),
    );
    if let Ok(pipe) = load_pipeline_config(&root).await {
        let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_board(
    board_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
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

    let _ = app.emit(
        "boards:updated",
        serde_json::json!({ "boards": &config.boards, "activeBoardId": &config.active_board_id }),
    );
    if let Ok(pipe) = load_pipeline_config(&root).await {
        let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
    }

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
    app: AppHandle,
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

    let _ = app.emit(
        "boards:updated",
        serde_json::json!({ "boards": &config.boards, "activeBoardId": &config.active_board_id }),
    );
    if let Ok(pipe) = load_pipeline_config(&root).await {
        let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
    }

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
pub async fn list_cortex_agent_files(
    role: String,
    state: State<'_, AppState>,
) -> Result<Vec<CortexDiskFile>, String> {
    let root = state.get_root();
    let role_dir = crate::config::agent_dir(&root, &role);
    let mut files = Vec::new();

    let persona_p = role_dir.join("persona.md");
    let agent_p = role_dir.join("agent.md");
    if persona_p.is_file() {
        files.push(CortexDiskFile {
            name: "persona.md".to_string(),
            path: format!(".cvh/agents/{}/persona.md", role),
            file_type: "persona".to_string(),
        });
    } else if agent_p.is_file() {
        files.push(CortexDiskFile {
            name: "agent.md".to_string(),
            path: format!(".cvh/agents/{}/agent.md", role),
            file_type: "persona".to_string(),
        });
    }

    let rules_dir = role_dir.join("rules");
    if let Ok(mut entries) = tokio::fs::read_dir(&rules_dir).await {
        let mut r_files = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(name) = entry.file_name().to_str() {
                    r_files.push(name.to_string());
                }
            }
        }
        r_files.sort();
        for r in r_files {
            files.push(CortexDiskFile {
                name: r.clone(),
                path: format!(".cvh/agents/{}/rules/{}", role, r),
                file_type: "rule".to_string(),
            });
        }
    }

    let setting_p = role_dir.join("setting");
    if setting_p.is_file() {
        files.push(CortexDiskFile {
            name: "setting".to_string(),
            path: format!(".cvh/agents/{}/setting", role),
            file_type: "settings".to_string(),
        });
    }

    Ok(files)
}

#[tauri::command]
pub async fn read_cortex_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let root = state.get_root();
    let target = if Path::new(&path).is_absolute() {
        PathBuf::from(path.clone())
    } else {
        root.join(&path)
    };

    if !target.exists() && path.contains("cvh-dev") && (path.ends_with("agent.md") || path.ends_with("persona.md") || path.ends_with("cvh-dev.md")) {
        return Ok(crate::agent::BUILTIN_CVH_DEV_PROMPT.to_string());
    }

    if !target.exists() && path.contains("developer") && (path.ends_with("agent.md") || path.ends_with("persona.md") || path.ends_with("developer.md")) {
        return Ok(crate::agent::BUILTIN_DEVELOPER_PROMPT.to_string());
    }

    tokio::fs::read_to_string(&target)
        .await
        .map_err(|e| format!("Failed to read cortex file {:?}: {}", target, e))
}

#[tauri::command]
pub async fn save_cortex_file(
    path: String,
    content: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let root = state.get_root();
    let target = if Path::new(&path).is_absolute() {
        PathBuf::from(&path)
    } else {
        root.join(&path)
    };

    atomic_write_file(&target, &content)
        .await
        .map_err(|e| format!("Failed to save cortex file {:?}: {}", target, e))?;

    let norm_path = path.replace('\\', "/");
    if norm_path.ends_with("boards.json") {
        if let Ok(bc) = load_boards_config(&root).await {
            let _ = app.emit(
                "boards:updated",
                serde_json::json!({ "boards": &bc.boards, "activeBoardId": &bc.active_board_id }),
            );
        }
        if let Ok(pipe) = load_pipeline_config(&root).await {
            let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
        }
    } else if norm_path.ends_with("pipeline.json") {
        if let Ok(pipe) = load_pipeline_config(&root).await {
            let _ = app.emit("pipeline:updated", serde_json::json!({ "pipeline": &pipe }));
        }
        if let Ok(bc) = load_boards_config(&root).await {
            let _ = app.emit(
                "boards:updated",
                serde_json::json!({ "boards": &bc.boards, "activeBoardId": &bc.active_board_id }),
            );
        }
    } else if norm_path.contains(".cvh/agents") || norm_path.contains("/agents/") {
        let list = list_agent_personas_in(&root).await;
        let _ = app.emit("agent:personas_updated", &list);
    }

    Ok(())
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
// Task Dialog Persistence (SQLite via db module)
// ============================================================================

pub use crate::db::ActivityItem;

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

#[tauri::command]
pub async fn save_task_dialog(
    task_id: String,
    items: Vec<ActivityItem>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    crate::db::append_dialog_items(&root, &safe_id, &items)?;
    eprintln!("CVH [save_task_dialog]: saved {} items to SQLite for task {}", items.len(), safe_id);
    Ok(())
}

pub async fn append_task_dialog_activity(
    root: &Path,
    task_id: &str,
    item_type: &str,
    content: &str,
    app_handle: Option<&AppHandle>,
) -> Result<(), String> {
    let safe_id = sanitize_task_id(task_id)?;
    let new_item = ActivityItem {
        id: uuid::Uuid::new_v4().to_string(),
        item_type: item_type.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        content: Some(content.to_string()),
        tool: None,
        duration_ms: None,
        images: None,
    };

    crate::db::append_dialog_items(root, &safe_id, std::slice::from_ref(&new_item))?;

    if let Some(app) = app_handle {
        let _ = app.emit("task:dialog", serde_json::json!({
            "taskId": task_id,
            "item": new_item,
        }));
    }

    Ok(())
}

#[tauri::command]
pub async fn load_task_dialog(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ActivityItem>, String> {
    let safe_id = sanitize_task_id(&task_id)?;
    let root = state.get_root();
    crate::db::load_dialog_items(&root, &safe_id)
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
    crate::db::clear_dialog_items(&root, &safe_id)
}
/// Маркер встроенного создания worktree на старте (on_enter).
/// Значение `col.hooks["on_enter"]`, при котором бэкенд сам вызывает
/// `create_worktree` вместо запуска shell-хука.
pub const BUILTIN_CREATE_WORKTREE_MARKER: &str = "builtin:create_worktree";

/// Явный builtin-маркер в `col.hooks["on_enter"]`.
pub fn is_builtin_worktree_hook(col: &crate::config::PipelineColumn) -> bool {
    col.hooks.get("on_enter").map(|s| s.as_str()) == Some(BUILTIN_CREATE_WORKTREE_MARKER)
}

/// Нужен ли builtin на старте для колонки:
/// явный маркер ИЛИ дефолт бэклога (id == "backlog" без явного on_enter).
pub fn should_builtin_on_enter(col: &crate::config::PipelineColumn) -> bool {
    if is_builtin_worktree_hook(col) {
        return true;
    }
    col.id == "backlog" && !col.hooks.contains_key("on_enter")
}

pub fn task_has_worktree(task: &Task) -> bool {
    task.worktree_path
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

/// Парсинг stdout хука формата `CVH_WORKTREE=<path>` (+ опционально
/// `CVH_BRANCH=<branch>`): пишется в task.worktree_path (+branch если пуст).
/// Общая для on_enter и on_finish.
pub fn apply_worktree_from_hook_stdout(task: &mut Task, stdout: &str) {
    for line in stdout.lines() {
        let t = line.trim();
        if let Some(v) = t.strip_prefix("CVH_WORKTREE=") {
            let v = v.trim();
            if !v.is_empty() {
                task.worktree_path = Some(v.to_string());
            }
        } else if let Some(v) = t.strip_prefix("CVH_BRANCH=") {
            let v = v.trim();
            let branch_empty = task
                .branch
                .as_deref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true);
            if !v.is_empty() && branch_empty {
                task.branch = Some(v.to_string());
            }
        }
    }
}

/// Чистое решение «нужен ли builtin сейчас»: skip-флаг и уже имеющийся
/// worktree (в т.ч. данный shell-хуком через stdout) отключают builtin.
pub fn needs_builtin_worktree(
    col: &crate::config::PipelineColumn,
    task: &Task,
    skip_worktree: bool,
) -> bool {
    if skip_worktree {
        return false;
    }
    if task_has_worktree(task) {
        return false;
    }
    should_builtin_on_enter(col)
}

/// Best-effort builtin: `create_worktree(root, lowercase(task.id), "HEAD")`
/// + branch `feat/<slug>` если пуст. Ошибки только варнят в лог.
pub fn stage_id_matches(col_id: &str, target: &str) -> bool {
    if col_id.eq_ignore_ascii_case(target) {
        return true;
    }
    let c = col_id.to_ascii_lowercase();
    let t = target.to_ascii_lowercase();
    if (t == "dev" || t == "development") && (c == "dev" || c == "development") {
        return true;
    }
    if (t == "backlog" || t == "waiting" || t == "ожидание") && (c == "backlog" || c == "waiting" || c == "ожидание") {
        return true;
    }
    if (t == "ready" || t == "done" || t == "готово") && (c == "ready" || c == "done" || c == "готово") {
        return true;
    }
    false
}

pub async fn ensure_builtin_worktree(root: &Path, task: &mut Task) {
    if let Some(ref p) = task.worktree_path {
        let pb = Path::new(p.trim());
        let abs = if pb.is_absolute() {
            pb.to_path_buf()
        } else {
            root.join(pb)
        };
        if abs.exists() {
            return;
        }
    }
    let slug = task.id.to_lowercase();
    let candidates = [
        root.join(".cvh").join("worktrees").join(&slug),
        root.join("worktrees").join(&slug),
    ];
    for cand in &candidates {
        if cand.exists() {
            task.worktree_path = Some(cand.to_string_lossy().to_string());
            if task.branch.as_deref().unwrap_or("").trim().is_empty() {
                task.branch = Some(format!("feat/{}", slug));
            }
            return;
        }
    }

    match create_worktree(root, &slug, "HEAD").await {
        Ok(wt_abs) => {
            task.worktree_path = Some(wt_abs.to_string_lossy().to_string());
            let branch_empty = task
                .branch
                .as_deref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true);
            if branch_empty {
                task.branch = Some(format!("feat/{}", slug));
            }
        }
        Err(e) => {
            eprintln!(
                "CVH [builtin_worktree]: creation failed for {}: {}",
                task.id, e
            );
            for cand in &candidates {
                if cand.exists() {
                    task.worktree_path = Some(cand.to_string_lossy().to_string());
                    break;
                }
            }
            if task.branch.as_deref().unwrap_or("").trim().is_empty() {
                task.branch = Some(format!("feat/{}", slug));
            }
        }
    }
}

#[tauri::command]
pub async fn create_task(
    title: String,
    description: String,
    stage_id: String,
    board_id: Option<String>,
    skip_worktree: Option<bool>,
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
    let now = Utc::now().timestamp_millis();

    // Ворктри выделяется на СТАРТЕ (on_enter): shell-хук через execute_hook
    // (stdout `CVH_WORKTREE=`/`CVH_BRANCH=`), иначе builtin create_worktree
    // при маркере `builtin:create_worktree` или дефолте бэклога.
    // `skip_worktree=true` (чекбокс в модалке) отключает только builtin.
    // Новая задача без worktree/branch до хуков.
    let mut new_task = Task {
        id: task_id.clone(),
        title: title.clone(),
        description: description.clone(),
        stage_id: stage_id.clone(),
        board_id: Some(effective_board_id.clone()),
        worktree_path: None,
        branch: None,
        verdict: None,
        custom_flow: None,
        status: None,
        model: None,
        parent_id: None,
        subtask_index: None,
        waiting_for_subtask_id: None,
        handover_summary: None,
        cover_image: None,
        attachments: None,
        todos: None,
        created_at: now,
        updated_at: now,
    };

    // Check pipeline config for stage hooks (board-aware: сначала колонки
    // доски effective_board_id, затем pipeline.json как фолбэк).
    let skip = skip_worktree.unwrap_or(false);
    {
        let boards_cfg = load_boards_config(&root).await.ok();
        let col_from_board = boards_cfg.as_ref().and_then(|bc| {
            let b = bc.boards.iter().find(|b| b.id == effective_board_id)
                .or_else(|| bc.boards.iter().find(|b| b.id == bc.active_board_id));
            b.and_then(|b| b.columns.iter().find(|c| c.id == stage_id).cloned())
        });
        let col_opt: Option<crate::config::PipelineColumn> = match col_from_board {
            Some(c) => Some(c),
            None => load_pipeline_config(&root)
                .await
                .ok()
                .and_then(|p| p.columns.into_iter().find(|c| c.id == stage_id)),
        };
        if let Some(col) = col_opt {
            trigger_column_on_enter_script(&col, &new_task, &stage_id, None, &root).await;
            let hook_rel_opt = col.hooks.get("on_enter").cloned();
            match hook_rel_opt {
                Some(hook_rel) if hook_rel == BUILTIN_CREATE_WORKTREE_MARKER => {
                    // Встроенный маркер: shell не запускаем, builtin ниже.
                }
                Some(hook_rel) => {
                    // Configured path first; missing file falls back to
                    // `.cvh/agents/<persona>/hooks/<basename>`.
                    let hook_path = crate::config::resolve_hook_path(
                        &root,
                        col.persona.as_deref(),
                        &hook_rel,
                    );
                    if hook_path.exists() {
                        let ctx = HookContext {
                            task_id: task_id.clone(),
                            task_title: title.clone(),
                            worktree_path: new_task.worktree_path.clone().unwrap_or_default(),
                            branch: new_task.branch.clone().unwrap_or_default(),
                            verdict: None,
                            stage_id: Some(stage_id.clone()),
                            board_id: Some(effective_board_id.clone()),
                            extra_env: HashMap::new(),
                        };
                        match execute_hook(&hook_path, &root, &ctx, None).await {
                            Ok(res) => {
                                if !res.success {
                                    eprintln!(
                                        "CVH [on_enter]: hook {:?} failed for new task {} (exit code {:?}):\nSTDOUT: {}\nSTDERR: {}",
                                        hook_path, task_id, res.exit_code, res.stdout, res.stderr
                                    );
                                }
                                // Shell-приоритет: путь из stdout отключает builtin.
                                apply_worktree_from_hook_stdout(&mut new_task, &res.stdout);
                            }
                            Err(e) => {
                                eprintln!(
                                    "CVH [on_enter]: error executing hook {:?} for new task {}: {}",
                                    hook_path, task_id, e
                                );
                            }
                        }
                    }
                }
                None => {}
            }
            if needs_builtin_worktree(&col, &new_task, skip) {
                ensure_builtin_worktree(&root, &mut new_task).await;
            }
        } else if stage_id == "backlog" && !skip {
            // Колонка не найдена, но stage — backlog: дефолт бэклога.
            ensure_builtin_worktree(&root, &mut new_task).await;
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
        handover_summary: None,
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

pub async fn internal_move_task_stage(
    root: &Path,
    task_id: &str,
    target_stage_id: &str,
    verdict: Option<String>,
    status_override: Option<String>,
    handover_summary: Option<String>,
    app_handle: Option<&AppHandle>,
) -> Result<Task, String> {
    let mut tasks = load_tasks_from_disk(root).await;

    let task_idx = tasks
        .iter()
        .position(|t| t.id == task_id)
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let current_task = &mut tasks[task_idx];

    // --- Board resolution & fallback (support alias and fallback when board_id missing/stale) ---
    let boards_cfg = load_boards_config(root).await.ok();
    let mut effective_board: Option<crate::config::BoardDef> = None;

    if let Some(ref bc) = boards_cfg {
        let active_board = bc.boards.iter().find(|b| b.id == bc.active_board_id).cloned();
        let board_switched = current_task.board_id.as_deref().is_some_and(|bid| bid != bc.active_board_id);

        let current_board = current_task.board_id.as_deref().and_then(|bid| {
            bc.boards.iter().find(|b| b.id == bid).cloned()
        });

        if board_switched {
            // Active board changed during task execution: target board is the active board!
            effective_board = active_board.or_else(|| bc.boards.first().cloned());
            if let Some(ref b) = effective_board {
                current_task.board_id = Some(b.id.clone());
            }
        } else {
            let current_board_has_target = current_board
                .as_ref()
                .map(|b| b.columns.iter().any(|c| stage_id_matches(&c.id, target_stage_id)))
                .unwrap_or(false);

            if current_board_has_target {
                effective_board = current_board;
            } else {
                // Fallback:
                // 1. Try active board if it has target_stage_id
                let active_has_target = active_board
                    .as_ref()
                    .map(|b| b.columns.iter().any(|c| stage_id_matches(&c.id, target_stage_id)))
                    .unwrap_or(false);

                if active_has_target {
                    effective_board = active_board;
                } else {
                    // 2. Find any board that has target_stage_id
                    let any_with_target = bc.boards.iter().find(|b| {
                        b.columns.iter().any(|c| stage_id_matches(&c.id, target_stage_id))
                    }).cloned();

                    if any_with_target.is_some() {
                        effective_board = any_with_target;
                    } else {
                        // 3. Fallback to active board, or first board
                        effective_board = active_board.or_else(|| bc.boards.first().cloned());
                    }
                }

                // Update current_task.board_id to the valid board
                if let Some(ref b) = effective_board {
                    current_task.board_id = Some(b.id.clone());
                } else {
                    current_task.board_id = Some(bc.active_board_id.clone());
                }
            }
        }
    }

    // --- on_finish / on_leave hooks of source column ---
    let source_stage_id = current_task.stage_id.clone();
    let source_col: Option<crate::config::PipelineColumn> = {
        let from_board = effective_board.as_ref().and_then(|b| {
            b.columns.iter().find(|c| stage_id_matches(&c.id, &source_stage_id)).cloned()
        });
        match from_board {
            Some(c) => Some(c),
            None => load_pipeline_config(root)
                .await
                .ok()
                .and_then(|p| p.columns.into_iter().find(|c| stage_id_matches(&c.id, &source_stage_id))),
        }
    };

    if let Some(ref scol) = source_col {
        if let Some(hook_rel) = scol.hooks.get("on_finish").or_else(|| scol.hooks.get("on_leave")) {
            let hook_path = crate::config::resolve_hook_path(
                root,
                scol.persona.as_deref(),
                hook_rel,
            );
            if hook_path.exists() {
                let ctx = HookContext {
                    task_id: current_task.id.clone(),
                    task_title: current_task.title.clone(),
                    worktree_path: current_task.worktree_path.clone().unwrap_or_default(),
                    branch: current_task.branch.clone().unwrap_or_default(),
                    verdict: verdict.clone().or_else(|| current_task.verdict.clone()),
                    stage_id: Some(scol.id.clone()),
                    board_id: current_task.board_id.clone(),
                    extra_env: HashMap::new(),
                };
                match execute_hook(&hook_path, root, &ctx, None).await {
                    Ok(res) => {
                        if !res.success {
                            eprintln!(
                                "CVH [on_finish]: hook {:?} failed for task {} leaving stage {} (exit code {:?}):\nSTDOUT: {}\nSTDERR: {}",
                                hook_path, current_task.id, scol.id, res.exit_code, res.stdout, res.stderr
                            );
                        }
                        apply_worktree_from_hook_stdout(current_task, &res.stdout);
                    }
                    Err(e) => {
                        eprintln!(
                            "CVH [on_finish]: error executing hook {:?} for task {}: {}",
                            hook_path, current_task.id, e
                        );
                    }
                }
            }
        }
    }

    let target_col = {
        let col_from_board = effective_board.as_ref().and_then(|b| {
            b.columns.iter().find(|c| stage_id_matches(&c.id, target_stage_id)).cloned()
        });
        match col_from_board {
            Some(c) => c,
            None => {
                // If board switched or target_stage_id is not found on target board,
                // fall back to terminal column of target board so task transitions to 'done' cleanly
                let terminal_fallback = effective_board.as_ref().and_then(|b| {
                    b.columns.iter().find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done")).cloned()
                        .or_else(|| b.columns.last().cloned())
                });
                if let Some(term) = terminal_fallback {
                    term
                } else {
                    let pipeline = load_pipeline_config(root)
                        .await
                        .map_err(|e| format!("Failed to load pipeline: {}", e))?;
                    pipeline
                        .columns
                        .iter()
                        .find(|c| stage_id_matches(&c.id, target_stage_id))
                        .cloned()
                        .or_else(|| {
                            pipeline
                                .columns
                                .iter()
                                .find(|c| {
                                    c.terminal == Some(true)
                                        || stage_id_matches(&c.id, "ready")
                                        || stage_id_matches(&c.id, "done")
                                })
                                .cloned()
                        })
                        .ok_or_else(|| {
                            format!(
                                "Target stage {} not found in pipeline or board",
                                target_stage_id
                            )
                        })?
                }
            }
        }
    };
    let canonical_target_stage_id = target_col.id.clone();

    // Worktree allocation: ensure worktree when moving to dev or when target column requires builtin
    if needs_builtin_worktree(&target_col, current_task, false)
        || stage_id_matches(&canonical_target_stage_id, "dev")
    {
        ensure_builtin_worktree(root, current_task).await;
    }

    // Gatekeeper Sensor Verification: do not block initial move from backlog to dev or new/empty worktree
    if let Some(sensor_cmd) = &target_col.gatekeeper_sensor {
        let is_from_backlog = source_stage_id.is_empty()
            || stage_id_matches(&source_stage_id, "backlog");
        let is_to_dev = stage_id_matches(&canonical_target_stage_id, "dev");
        let is_initial_move_to_dev = is_from_backlog && is_to_dev;

        let wt_empty_or_new = match &current_task.worktree_path {
            None => true,
            Some(p) => {
                let path = Path::new(p);
                !path.exists() || path.read_dir().map(|mut it| it.next().is_none()).unwrap_or(true)
            }
        };

        if is_initial_move_to_dev || wt_empty_or_new {
            println!(
                "CVH [gatekeeper]: skipping gatekeeper sensor '{}' for initial transition of task {} from '{}' to '{}' (initial move / empty worktree)",
                sensor_cmd, current_task.id, source_stage_id, canonical_target_stage_id
            );
        } else {
            let wt_path = current_task
                .worktree_path
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(|| root.to_path_buf());

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
    }

    // Trigger on_enter hook if configured
    let target_hook_rel = target_col.hooks.get("on_enter").cloned();
    match target_hook_rel {
        Some(hook_rel) if hook_rel == BUILTIN_CREATE_WORKTREE_MARKER => {
            // Builtin only
        }
        Some(hook_rel) => {
            let hook_path = crate::config::resolve_hook_path(
                root,
                target_col.persona.as_deref(),
                &hook_rel,
            );
            if hook_path.exists() {
                let ctx = HookContext {
                    task_id: current_task.id.clone(),
                    task_title: current_task.title.clone(),
                    worktree_path: current_task.worktree_path.clone().unwrap_or_default(),
                    branch: current_task.branch.clone().unwrap_or_default(),
                    verdict: verdict.clone().or_else(|| current_task.verdict.clone()),
                    stage_id: Some(canonical_target_stage_id.clone()),
                    board_id: current_task.board_id.clone(),
                    extra_env: HashMap::new(),
                };
                let hook_res = execute_hook(&hook_path, root, &ctx, None)
                    .await
                    .map_err(|e| format!("Stage transition hook failed: {}", e))?;

                if !hook_res.success {
                    return Err(format!(
                        "Stage transition hook rejected transition (Exit code: {:?}):\n{}",
                        hook_res.exit_code, hook_res.stderr
                    ));
                }
                apply_worktree_from_hook_stdout(current_task, &hook_res.stdout);
            }
        }
        None => {}
    }
    if needs_builtin_worktree(&target_col, current_task, false) {
        ensure_builtin_worktree(root, current_task).await;
    }

    // Trigger on_enter_script if configured on target column
    trigger_column_on_enter_script(
        &target_col,
        current_task,
        &canonical_target_stage_id,
        verdict.as_deref(),
        root,
    )
    .await;

    // Update task
    let old_stage = current_task.stage_id.clone();
    current_task.stage_id = canonical_target_stage_id.clone();
    if verdict.is_some() {
        current_task.verdict = verdict;
    }
    if let Some(summary) = handover_summary {
        current_task.handover_summary = Some(summary);
    }
    let is_terminal_col = target_col.terminal == Some(true)
        || stage_id_matches(&canonical_target_stage_id, "ready")
        || stage_id_matches(&canonical_target_stage_id, "done");
    if let Some(st) = status_override {
        current_task.status = Some(st);
    } else if is_terminal_col {
        current_task.status = Some("done".to_string());
    } else if target_col.persona.is_some() || target_col.auto_run == Some(true) {
        current_task.status = Some("running".to_string());
    } else {
        current_task.status = Some("idle".to_string());
    }
    current_task.updated_at = Utc::now().timestamp_millis();

    let updated = current_task.clone();
    if let Some(app) = app_handle {
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(mut guard) = state_ref.tasks.lock() {
                *guard = tasks.clone();
            }
        }
    }
    save_tasks_to_disk(root, &tasks).await?;

    if let Some(app) = app_handle {
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
        let _ = app.emit(
            "task:moved",
            serde_json::json!({
                "taskId": task_id,
                "fromStage": old_stage,
                "toStage": &canonical_target_stage_id,
                "stageId": &canonical_target_stage_id,
                "status": updated.status,
                "verdict": updated.verdict,
                "task": &updated
            }),
        );
        if target_col.persona.is_some() || target_col.auto_run == Some(true) {
            let _ = app.emit("agent:autorun", serde_json::json!({ "taskId": task_id }));
        }
    }

    Ok(updated)
}

#[tauri::command]
pub async fn move_task_stage(
    task_id: String,
    target_stage_id: String,
    verdict: Option<String>,
    handover_summary: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let root = state.get_root();
    internal_move_task_stage(&root, &task_id, &target_stage_id, verdict, None, handover_summary, Some(&app_handle)).await
}

pub async fn internal_advance_task(
    root: &Path,
    task_id: &str,
    summary: &str,
    app_handle: Option<&AppHandle>,
) -> Result<String, String> {
    let tasks = load_tasks_from_disk(root).await;
    let current_task = tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| format!("Task {} not found", task_id))?;

    let boards_cfg = load_boards_config(root).await.ok();
    let active_board_id = boards_cfg
        .as_ref()
        .map(|b| b.active_board_id.clone())
        .unwrap_or_else(|| "development".to_string());

    // Check if active board changed during task execution
    let board_switched = current_task
        .board_id
        .as_deref()
        .is_some_and(|b| b != active_board_id);

    if board_switched {
        // Active board changed: finalize task directly to terminal column ('ready') of target active board
        let target_board = boards_cfg.as_ref().and_then(|bc| {
            bc.boards.iter().find(|b| b.id == active_board_id).cloned()
        });

        let target_columns = match target_board {
            Some(ref b) => b.columns.clone(),
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .map_err(|e| e.to_string())?;
                pipeline.columns
            }
        };

        let terminal_col = target_columns
            .iter()
            .find(|c| {
                c.terminal == Some(true)
                    || stage_id_matches(&c.id, "ready")
                    || stage_id_matches(&c.id, "done")
            })
            .cloned()
            .or_else(|| target_columns.last().cloned())
            .ok_or_else(|| format!("Target board '{}' has no columns", active_board_id))?;

        let updated_task = internal_move_task_stage(
            root,
            task_id,
            &terminal_col.id,
            Some("APPROVED".to_string()),
            Some("done".to_string()),
            Some(summary.to_string()),
            app_handle,
        )
        .await?;

        let summary_content = format!(
            "### Саммари финализации (смена доски -> {}): {}\n\n{}",
            active_board_id, terminal_col.title, summary
        );
        let _ = append_task_dialog_activity(
            root,
            task_id,
            "summary",
            &summary_content,
            app_handle,
        )
        .await;

        return Ok(format!(
            "УСПЕШНО: Задача {} финализирована на активной доске '{}' в терминальной колонке '{}' ({}), статус: {}",
            task_id,
            active_board_id,
            terminal_col.id,
            terminal_col.title,
            updated_task.status.as_deref().unwrap_or("done")
        ));
    }

    // Normal progression within same board
    let columns = {
        let from_board = boards_cfg.as_ref().and_then(|bc| {
            let b = if let Some(ref bid) = current_task.board_id {
                bc.boards.iter().find(|b| &b.id == bid)
            } else {
                bc.boards.iter().find(|b| b.id == bc.active_board_id)
            };
            b.map(|b| b.columns.clone())
        });
        match from_board {
            Some(cols) => cols,
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .map_err(|e| e.to_string())?;
                pipeline.columns
            }
        }
    };

    let curr_idx = columns
        .iter()
        .position(|c| stage_id_matches(&c.id, &current_task.stage_id))
        .ok_or_else(|| {
            format!(
                "Current column '{}' not found in pipeline",
                current_task.stage_id
            )
        })?;

    if curr_idx + 1 >= columns.len() {
        return Err(format!(
            "Задача {} уже находится на последнем этапе пайплайна ({})",
            task_id, columns[curr_idx].title
        ));
    }

    let next_col = &columns[curr_idx + 1];
    let is_terminal = next_col.terminal == Some(true)
        || stage_id_matches(&next_col.id, "ready")
        || stage_id_matches(&next_col.id, "done");
    let status_override = if is_terminal {
        Some("done".to_string())
    } else {
        None
    };

    let updated_task = internal_move_task_stage(
        root,
        task_id,
        &next_col.id,
        Some("APPROVED".to_string()),
        status_override,
        Some(summary.to_string()),
        app_handle,
    )
    .await?;

    let summary_content = format!("### Саммари этапа: {}\n\n{}", next_col.title, summary);
    let _ = append_task_dialog_activity(
        root,
        task_id,
        "summary",
        &summary_content,
        app_handle,
    )
    .await;

    Ok(format!(
        "УСПЕШНО: Задача {} переведена на этап '{}' ({}), статус: {}",
        task_id,
        next_col.id,
        next_col.title,
        updated_task.status.as_deref().unwrap_or("idle")
    ))
}

#[tauri::command]
pub async fn advance_task(
    task_id: String,
    summary: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let root = state.get_root();
    internal_advance_task(&root, &task_id, &summary, Some(&app)).await
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

    // Per-agent overrides (best-effort: missing/broken setting → defaults).
    let agent_setting = crate::config::load_agent_setting(&root, &persona_role).await;
    if let Some(cw) = agent_setting.context_window {
        eprintln!(
            "CVH [run_agent]: task_id={} persona='{}' context_window={}",
            task.id, persona_role, cw
        );
    }

    fn non_empty_opt(v: &Option<String>) -> Option<String> {
        v.as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    // Model resolution precedence:
    // 1. Task-specific model override (task.model)
    // 2. Global LLM settings model (llm_settings.model)
    // 3. Per-agent setting model (agents/<role>/setting → model)
    // 4. Pipeline column model override (col.model_override)
    // 5. Project default model (proj_cfg.default_model)
    let llm_model = {
        let m = llm_settings.model.trim();
        if m.is_empty() { None } else { Some(m.to_string()) }
    };
    let model = non_empty_opt(&task.model)
        .or(llm_model)
        .or_else(|| non_empty_opt(&agent_setting.model))
        .or_else(|| non_empty_opt(&col.model_override))
        .or_else(|| non_empty_opt(&proj_cfg.default_model))
        .unwrap_or_else(|| "gemini-3.8-flash-high".to_string());

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

    // Temperature precedence: llm_settings → agent setting → column.
    let temperature = llm_settings
        .temperature
        .or(agent_setting.temperature)
        .or(col.temperature);

    // Ворктри выделяется ТОЛЬКО на СТАРТЕ (on_enter в create_task /
    // move_task_stage: shell-хук или builtin). Создавать ворктри здесь
    // ЗАПРЕЩЕНО: нет пути / не существует на диске → fallback корень репо.
    let worktree_dir = match &task.worktree_path {
        Some(p) if !p.trim().is_empty() => {
            let pb = Path::new(p.trim());
            let abs = if pb.is_absolute() {
                pb.to_path_buf()
            } else {
                root.join(pb)
            };
            if abs.exists() {
                abs
            } else {
                eprintln!(
                    "CVH [run_agent]: WARN task {} worktree {:?} does not exist — creating is forbidden, falling back to repo root {:?}",
                    task_id, p, root
                );
                root.clone()
            }
        }
        _ => {
            eprintln!(
                "CVH [run_agent]: WARN task {} has no worktree_path — creating is forbidden, falling back to repo root {:?}",
                task_id, root
            );
            root.clone()
        }
    };

    let abort_flag = Arc::new(AtomicBool::new(false));
    let join_slot = Arc::new(tokio::sync::Mutex::new(None));

    // Store in active agents registry
    {
        let state = app_handle.state::<AppState>();
        let mut map = state
            .active_agents
            .lock()
            .map_err(|_| "Failed to lock active agents".to_string())?;
        map.insert(
            task_id.clone(),
            ActiveAgentHandle {
                abort_flag: Arc::clone(&abort_flag),
                join_handle: Arc::clone(&join_slot),
            },
        );
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

    // Initial user prompt: когда у задачи есть worktree_path — сообщить агенту
    // его изолированный workdir.
    let mut initial_description = task.description.clone();
    if task
        .worktree_path
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        initial_description.push_str(&format!(
            "\n\n[HOOK] worktree: {} — твой изолированный workdir, работаешь тут.",
            worktree_dir.to_string_lossy()
        ));
    }

    let agent_cfg = AgentSessionConfig {
        repo_root: root.clone(),
        worktree_path: worktree_dir,
        role: persona_role,
        stage_id: task.stage_id.clone(),
        task_id: task.id.clone(),
        task_title: task.title.clone(),
        task_description: initial_description,
        handover_summary: task.handover_summary.clone(),
        steer_prompt,
        base_url,
        api_key,
        model,
        temperature,
        reasoning_effort: col.reasoning_effort.clone(),
        max_turns: 25,
        app_handle: Some(app_handle.clone()),
    };

    let tid = task.id.clone();
    let app = app_handle.clone();
    let root_task = root.clone();
    let abort_flag_task = Arc::clone(&abort_flag);
    let initial_active_board_id = {
        let boards_cfg = load_boards_config(&root).await.ok();
        boards_cfg.map(|b| b.active_board_id)
    };

    let handle = tokio::spawn(async move {
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
            Ok(_final_response) => {
                let _ = app.emit(
                    "agent:status",
                    serde_json::json!({ "taskId": tid, "status": "finished" }),
                );

                // Reload tasks from disk in case advance_task, reject_task, or todo_write modified them
                let mut tasks = load_tasks_from_disk(&root_task).await;
                if let Some(task_idx) = tasks.iter().position(|t| t.id == tid) {
                    let end_boards_cfg = load_boards_config(&root_task).await.ok();
                    let end_active_board_id = end_boards_cfg.as_ref().map(|b| b.active_board_id.clone());
                    let board_changed = end_active_board_id.is_some() && (
                        end_active_board_id != initial_active_board_id ||
                        end_active_board_id.as_deref() != tasks[task_idx].board_id.as_deref()
                    );

                    if board_changed {
                        // If active board changed during task execution, finalize and transition to 'done' in terminal column ('ready') of target board
                        let active_id = end_active_board_id.unwrap();
                        let target_board = end_boards_cfg.as_ref().and_then(|bc| {
                            bc.boards.iter().find(|b| b.id == active_id).cloned()
                        });
                        let terminal_col_id = target_board.as_ref().and_then(|b| {
                            b.columns.iter().find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done")).map(|c| c.id.clone())
                                .or_else(|| b.columns.last().map(|c| c.id.clone()))
                        }).unwrap_or_else(|| "ready".to_string());

                        tasks[task_idx].board_id = Some(active_id);
                        tasks[task_idx].stage_id = terminal_col_id;
                        tasks[task_idx].status = Some("done".to_string());
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                    } else if tasks[task_idx].status.as_deref() == Some("running") {
                        tasks[task_idx].status = Some("idle".to_string());
                        tasks[task_idx].updated_at = Utc::now().timestamp_millis();
                    }
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
                            "stageId": updated_task.stage_id,
                            "status": updated_task.status,
                            "task": updated_task
                        }),
                    );
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

        // Clean up from active_agents registry upon agent completion only if this entry still belongs to this run
        if let Some(state_ref) = app.try_state::<AppState>() {
            if let Ok(mut map) = state_ref.active_agents.lock() {
                if let Some(entry) = map.get(&tid) {
                    if Arc::ptr_eq(&entry.abort_flag, &abort_flag_task) {
                        map.remove(&tid);
                    }
                }
            }
        }
    });

    // Populate the JoinHandle slot for cooperative cancellation and awaiting in steer_agent
    {
        let mut guard = join_slot.lock().await;
        *guard = Some(handle);
    }

    Ok(format!("Agent launched for task {}", task_id))
}

#[tauri::command]
pub async fn abort_agent(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let entry = {
        let mut map = state
            .active_agents
            .lock()
            .map_err(|_| "Failed to lock active agents".to_string())?;
        map.remove(&task_id)
    };

    if let Some(agent_entry) = entry {
        agent_entry.abort_flag.store(true, Ordering::SeqCst);
        let maybe_handle = {
            let mut guard = agent_entry.join_handle.lock().await;
            guard.take()
        };
        if let Some(handle) = maybe_handle {
            let _ = tokio::time::timeout(tokio::time::Duration::from_millis(2000), handle).await;
        }
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

    let mut status_cmd = Command::new("git");
    status_cmd
        .current_dir(&workdir)
        .args(["status", "--porcelain=v1", "-uall"]);
    #[cfg(windows)]
    status_cmd.creation_flags(CREATE_NO_WINDOW);
    let status_output = status_cmd
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

        let status = if xy == "??" || xy.contains('A') {
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

    let mut head_cmd = Command::new("git");
    head_cmd
        .current_dir(&workdir)
        .args(["rev-parse", "--verify", "HEAD"]);
    #[cfg(windows)]
    head_cmd.creation_flags(CREATE_NO_WINDOW);
    let head_exists = head_cmd
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    let diff_args = if head_exists {
        vec!["diff", "HEAD"]
    } else {
        vec!["diff"]
    };

    let mut diff_cmd = Command::new("git");
    diff_cmd
        .current_dir(&workdir)
        .args(&diff_args);
    #[cfg(windows)]
    diff_cmd.creation_flags(CREATE_NO_WINDOW);
    let git_diff_out = diff_cmd
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
            let mut untracked_cmd = Command::new("git");
            untracked_cmd
                .current_dir(&workdir)
                .args(["diff", "--no-index", "--", "/dev/null", &file_path]);
            #[cfg(windows)]
            untracked_cmd.creation_flags(CREATE_NO_WINDOW);
            let untracked_diff = untracked_cmd
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
            let mut file_cmd = Command::new("git");
            file_cmd
                .current_dir(&workdir)
                .args(["diff", "HEAD", "--", &file_path]);
            #[cfg(windows)]
            file_cmd.creation_flags(CREATE_NO_WINDOW);
            let file_diff = file_cmd
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

    // If an agent is currently running for this task, signal abort and await its full termination
    // before re-triggering. This eliminates concurrent runs in the same worktree and avoids
    // the old agent deleting the new agent's abort flag upon exit.
    {
        let prev_entry = {
            let mut map = state
                .active_agents
                .lock()
                .map_err(|_| "Failed to lock active agents".to_string())?;
            map.remove(&task_id)
        };

        if let Some(agent_entry) = prev_entry {
            // 1. Signal abort
            agent_entry.abort_flag.store(true, Ordering::SeqCst);

            // 2. Await previous coroutine termination (defensive 5s ceiling)
            let maybe_handle = {
                let mut guard = agent_entry.join_handle.lock().await;
                guard.take()
            };
            if let Some(handle) = maybe_handle {
                if tokio::time::timeout(tokio::time::Duration::from_millis(5000), handle)
                    .await
                    .is_err()
                {
                    eprintln!(
                        "CVH [steer_agent]: Warning: previous agent task for {} did not finish within 5s timeout, proceeding with steer",
                        task_id
                    );
                }
            }
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

#[tauri::command]
pub async fn answer_agent_question(
    task_id: String,
    question_id: String,
    answer: String,
) -> Result<(), String> {
    let mut reg = crate::agent::QUESTION_REGISTRY.lock().await;
    if let Some(sender) = reg.remove(&question_id) {
        sender
            .send(answer)
            .map_err(|_| "Failed to send answer: agent question channel was dropped".to_string())?;
        Ok(())
    } else {
        Err(format!(
            "Question {} for task {} not found in registry (already answered or expired)",
            question_id, task_id
        ))
    }
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
    fn test_normalize_without_specials_reinjects_backlog_and_ready() {
        let cols = vec![test_col("a"), test_col("b"), test_col("c")];
        let out = normalize_pipeline_columns(cols).expect("normalize failed");
        assert_eq!(ids(&out), vec!["backlog", "a", "b", "c", "ready"]);
        assert_eq!(out.last().unwrap().terminal, Some(true));
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
        let t = persona_placeholder_template("reviewer", "ShieldCheck", "#a855f7");
        assert!(t.contains("name: reviewer"));
        assert!(t.contains("icon: ShieldCheck"));
        assert!(t.contains("color: #a855f7"));
        assert!(t.contains("# Роль: reviewer"));
        assert!(t.contains("TODO"));
        assert!(t.contains("создан автоматически"));
    }

    #[test]
    fn test_agent_persona_info_camel_case() {
        let info = AgentPersonaInfo {
            role: "developer".to_string(),
            has_prompt: true,
            is_builtin: false,
            persona_file: "persona.md".to_string(),
            rules: vec!["r1.md".to_string()],
            hooks: vec![],
            icon: Some("Code2".to_string()),
            color: Some("#0284c7".to_string()),
        };
        let v = serde_json::to_value(&info).expect("serialize");
        assert_eq!(v.get("role").and_then(|x| x.as_str()), Some("developer"));
        assert_eq!(v.get("hasPrompt").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("isBuiltin").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(v.get("personaFile").and_then(|x| x.as_str()), Some("persona.md"));
        assert_eq!(v.get("icon").and_then(|x| x.as_str()), Some("Code2"));
        assert_eq!(v.get("color").and_then(|x| x.as_str()), Some("#0284c7"));
        assert!(v.get("has_prompt").is_none());
        assert!(v.get("is_builtin").is_none());

        let builtin = AgentPersonaInfo {
            role: "cvh-dev".to_string(),
            has_prompt: true,
            is_builtin: true,
            persona_file: "persona.md".to_string(),
            rules: vec!["boards.md".to_string()],
            hooks: vec![],
            icon: Some("Cpu".to_string()),
            color: Some("#6366f1".to_string()),
        };
        let vb = serde_json::to_value(&builtin).expect("serialize");
        assert_eq!(vb.get("isBuiltin").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(vb.get("icon").and_then(|x| x.as_str()), Some("Cpu"));
        assert_eq!(vb.get("color").and_then(|x| x.as_str()), Some("#6366f1"));
    }

    #[tokio::test]
    async fn test_agent_colors_palette_and_forbidden_invariants() {
        // Серый (#64748b) и зеленый (#10b981) не должны быть в AGENT_PALETTE
        for color in AGENT_PALETTE {
            assert_ne!(*color, "#64748b", "Backlog gray is strictly forbidden in agent palette");
            assert_ne!(*color, "#6b7280", "Gray is strictly forbidden in agent palette");
            assert_ne!(*color, "#10b981", "Ready green is strictly forbidden in agent palette");
            assert_ne!(*color, "#22c55e", "Green is strictly forbidden in agent palette");
        }

        let dev_col = deterministic_color_for_role("developer");
        assert!(!FORBIDDEN_AGENT_COLORS.contains(&dev_col.as_str()));

        let text_with_color = "---\nname: foo\ncolor: #f59e0b\n---\n# Body";
        assert_eq!(extract_persona_color_from_text(text_with_color), Some("#f59e0b".to_string()));

        let updated = insert_or_update_frontmatter_color("# Title", "#a855f7");
        assert!(updated.contains("color: #a855f7"));
        assert_eq!(extract_persona_color_from_text(&updated), Some("#a855f7".to_string()));
    }

    #[tokio::test]
    async fn test_agent_svg_icons_generation_and_persistence() {
        assert_eq!(default_column_icon("backlog"), "Inbox");
        assert_eq!(default_column_icon("ready"), "CheckCircle2");
        assert_eq!(default_column_icon("other"), "Layers");

        let text_with_icon = "---\nname: foo\nicon: Rocket\n---\n# Body";
        assert_eq!(extract_persona_icon_from_text(text_with_icon), Some("Rocket".to_string()));

        let text_no_icon = "# Just Title";
        let updated = insert_or_update_frontmatter_icon(text_no_icon, "Flame");
        assert!(updated.contains("icon: Flame"));
        assert_eq!(extract_persona_icon_from_text(&updated), Some("Flame".to_string()));

        let text_update_icon = "---\nname: bar\nicon: Zap\n---\n# Bar";
        let updated2 = insert_or_update_frontmatter_icon(text_update_icon, "Radar");
        assert!(updated2.contains("icon: Radar"));
        assert!(!updated2.contains("icon: Zap"));
        assert_eq!(extract_persona_icon_from_text(&updated2), Some("Radar".to_string()));

        let base = PathBuf::from("target").join(format!("cvh_persona_icon_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        // Создаем персону через create_agent_persona_file — она должна сразу получить иконку
        let rel = create_agent_persona_file(&base, "bot-agent").await.expect("create bot-agent");
        let content = tokio::fs::read_to_string(base.join(&rel)).await.expect("read created");
        let icon_opt = extract_persona_icon_from_text(&content);
        assert!(icon_opt.is_some(), "Созданная персона должна содержать иконку в frontmatter");
        let icon = icon_opt.unwrap();
        assert!(AGENT_ICONS.contains(&icon.as_str()), "Иконка должна быть из канонического пака");

        // Проверяем листинг персон
        let listed = list_agent_personas_in(&base).await;
        let bot_info = listed.iter().find(|p| p.role == "bot-agent").expect("bot-agent in list");
        assert_eq!(bot_info.icon.as_deref(), Some(icon.as_str()));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_create_persona_placeholder_in_tmp_dir() {
        let base =
            PathBuf::from("target").join(format!("cvh_persona_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        // invalid role → Err
        let bad = create_agent_persona_file(&base, "Bad_Name").await;
        assert!(bad.is_err());

        // valid create → directory `.cvh/agents/<role>/persona.md`
        let rel = create_agent_persona_file(&base, "my-role")
            .await
            .expect("create failed");
        assert_eq!(rel, ".cvh/agents/my-role/persona.md");
        let agents_dir = base.join(".cvh").join("agents");
        let role_dir = agents_dir.join("my-role");
        let content = tokio::fs::read_to_string(role_dir.join("persona.md"))
            .await
            .expect("read created file");
        assert!(content.contains("# Роль: my-role"));
        assert!(role_dir.join("rules").is_dir(), "rules/ must exist");
        assert!(role_dir.join("hooks").is_dir(), "hooks/ must exist");
        let setting = tokio::fs::read_to_string(role_dir.join("setting"))
            .await
            .expect("read setting");
        let parsed_setting: crate::config::AgentSetting =
            serde_json::from_str(&setting).expect("parse setting");
        assert_eq!(parsed_setting.model.as_deref(), Some("gemini-3.8-flash-high"));
        assert_eq!(parsed_setting.context_window, Some(128000));
        assert_eq!(parsed_setting.temperature, Some(0.2));
        assert_eq!(parsed_setting.max_tokens, Some(8192));

        // duplicate → Err mentioning существование
        let dup = create_agent_persona_file(&base, "my-role").await;
        assert!(dup.is_err());
        assert!(dup.unwrap_err().contains("уже существует"));

        // creating another role in directory
        create_agent_persona_file(&base, "aaa").await.expect("create aaa");
        tokio::fs::write(agents_dir.join("notes.txt"), "ignore me")
            .await
            .expect("seed txt");
        let listed = list_agent_personas_in(&base).await;
        let roles: Vec<String> = listed.iter().map(|p| p.role.clone()).collect();
        assert_eq!(roles, vec!["aaa".to_string(), "cvh-dev".to_string(), "my-role".to_string()]);
        assert!(listed.iter().all(|p| p.has_prompt));
        let cvh_dev_info = listed.iter().find(|p| p.role == "cvh-dev").unwrap();
        assert!(cvh_dev_info.is_builtin);

        // missing folder → still always contains builtin cvh-dev
        let empty_base =
            PathBuf::from("target").join(format!("cvh_persona_empty_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&empty_base).await.expect("empty dir");
        let empty = list_agent_personas_in(&empty_base).await;
        assert_eq!(empty.len(), 1);
        assert_eq!(empty[0].role, "cvh-dev");
        assert!(empty[0].is_builtin);
        assert!(empty[0].has_prompt);

        let _ = tokio::fs::remove_dir_all(&base).await;
        let _ = tokio::fs::remove_dir_all(&empty_base).await;
    }

    #[tokio::test]
    async fn test_list_personas_dir_only() {
        // Directory-only structure: subdirectories with persona.md/agent.md are listed.
        // Flat *.md files in agents/ are strictly ignored. cvh-dev is always included.
        let base =
            PathBuf::from("target").join(format!("cvh_persona_mix_{}", uuid::Uuid::new_v4()));
        let agents = base.join(".cvh").join("agents");
        tokio::fs::create_dir_all(&agents).await.expect("test dir");
        // Flat files should be ignored
        tokio::fs::write(agents.join("ignored.md"), "# flat ignored
")
            .await
            .expect("seed flat");
        // Directories with persona.md or agent.md
        let a_dir = agents.join("a-role");
        tokio::fs::create_dir_all(&a_dir).await.expect("a-role dir");
        tokio::fs::write(a_dir.join("persona.md"), "# a-role
")
            .await
            .expect("seed a-role persona.md");

        let b_dir = agents.join("b-role");
        tokio::fs::create_dir_all(&b_dir).await.expect("b-role dir");
        tokio::fs::write(b_dir.join("agent.md"), "# b-role
")
            .await
            .expect("seed b-role agent.md");

        tokio::fs::write(agents.join("junk.txt"), "ignore").await.expect("seed txt");

        let listed = list_agent_personas_in(&base).await;
        let roles: Vec<String> = listed.iter().map(|p| p.role.clone()).collect();
        assert_eq!(roles, vec!["a-role".to_string(), "b-role".to_string(), "cvh-dev".to_string()]);
        assert!(listed.iter().all(|p| p.has_prompt));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn test_delete_agent_persona_protection() {
        let base =
            PathBuf::from("target").join(format!("cvh_persona_del_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        // Deleting cvh-dev must be strictly forbidden
        let err = delete_agent_persona_file(&base, "cvh-dev").await.unwrap_err();
        assert!(err.contains("Запрещено удалять встроенную персону 'cvh-dev'"));

        // Creating and then deleting a custom persona succeeds
        create_agent_persona_file(&base, "temp-agent").await.expect("create temp-agent");
        let agents_dir = base.join(".cvh").join("agents");
        assert!(agents_dir.join("temp-agent").join("persona.md").is_file());

        delete_agent_persona_file(&base, "temp-agent").await.expect("delete temp-agent");
        assert!(!agents_dir.join("temp-agent").exists());

        // Deleting non-existent persona returns error
        let missing_err = delete_agent_persona_file(&base, "non-existent").await.unwrap_err();
        assert!(missing_err.contains("не найдена"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }
    #[tokio::test]
    async fn test_rename_agent_persona_and_boards_sync() {
        let base =
            PathBuf::from("target").join(format!("cvh_persona_ren_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        // Renaming cvh-dev must be strictly forbidden
        let err = rename_agent_persona_file(&base, "cvh-dev", "new-dev").await.unwrap_err();
        assert!(err.contains("Запрещено переименовывать встроенную персону 'cvh-dev'"));

        // Invalid name
        let bad_err = rename_agent_persona_file(&base, "agent-a", "Bad_Name!").await.unwrap_err();
        assert!(bad_err.contains("Недопустимое имя роли"));

        // Non-existent persona
        let notfound_err = rename_agent_persona_file(&base, "non-existent", "new-role").await.unwrap_err();
        assert!(notfound_err.contains("не найдена"));

        // Create persona
        create_agent_persona_file(&base, "agent-a").await.expect("create agent-a");
        let agents_dir = base.join(".cvh").join("agents");
        assert!(agents_dir.join("agent-a").join("persona.md").is_file());

        // Create board and pipeline referencing agent-a
        let backlog_col: crate::config::PipelineColumn = serde_json::from_value(serde_json::json!({
            "id": "backlog",
            "title": "Backlog"
        })).unwrap();
        let dev_col: crate::config::PipelineColumn = serde_json::from_value(serde_json::json!({
            "id": "dev",
            "title": "Dev",
            "persona": "agent-a"
        })).unwrap();
        let ready_col: crate::config::PipelineColumn = serde_json::from_value(serde_json::json!({
            "id": "ready",
            "title": "Ready",
            "terminal": true
        })).unwrap();

        let boards_cfg = crate::config::BoardsConfig {
            active_board_id: "test-board".to_string(),
            boards: vec![crate::config::BoardDef {
                id: "test-board".to_string(),
                name: "Test Board".to_string(),
                columns: vec![backlog_col, dev_col, ready_col],
            }],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Rename agent-a to agent-b
        rename_agent_persona_file(&base, "agent-a", "agent-b").await.expect("rename agent-a to agent-b");
        assert!(!agents_dir.join("agent-a").exists());
        assert!(agents_dir.join("agent-b").join("persona.md").is_file());

        // Check boards.json updated
        let loaded_boards = crate::config::load_boards_config(&base).await.expect("load boards");
        let dev_col = loaded_boards.boards[0].columns.iter().find(|c| c.id == "dev").unwrap();
        assert_eq!(dev_col.persona.as_deref(), Some("agent-b"));

        // Check pipeline.json updated
        let loaded_pipe = crate::config::load_pipeline_config(&base).await.expect("load pipeline");
        let pipe_dev_col = loaded_pipe.columns.iter().find(|c| c.id == "dev").unwrap();
        assert_eq!(pipe_dev_col.persona.as_deref(), Some("agent-b"));

        // Delete agent-b and verify references reset to None
        delete_agent_persona_file(&base, "agent-b").await.expect("delete agent-b");
        assert!(!agents_dir.join("agent-b").exists());

        let loaded_boards_after_del = crate::config::load_boards_config(&base).await.expect("load boards after del");
        let dev_col_del = loaded_boards_after_del.boards[0].columns.iter().find(|c| c.id == "dev").unwrap();
        assert_eq!(dev_col_del.persona, None);

        let _ = tokio::fs::remove_dir_all(&base).await;
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
            images: None,
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
    async fn test_task_dialog_sqlite_roundtrip() {
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
                images: None,
            },
            ActivityItem {
                id: "th-1".to_string(),
                item_type: "thought".to_string(),
                timestamp: "10:00:01".to_string(),
                content: Some("Анализирую требования к персистентности...".to_string()),
                tool: None,
                duration_ms: Some(2500.0),
                images: None,
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
                images: None,
            },
            ActivityItem {
                id: "out-1".to_string(),
                item_type: "output".to_string(),
                timestamp: "10:00:05".to_string(),
                content: Some("Диалоги успешно сохраняются в SQLite.".to_string()),
                tool: None,
                duration_ms: None,
                images: None,
            },
        ];

        crate::db::append_dialog_items(&base, "CVH-001", &items).expect("write dialog to sqlite");
        let loaded = crate::db::load_dialog_items(&base, "CVH-001").expect("load dialog from sqlite");

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

        let task_without_board: Task = serde_json::from_str(json_without_board).expect("deserialize task without board");
        assert!(task_without_board.board_id.is_none());

        let fixed = ensure_task_board_id(vec![task_without_board]);
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

    fn hook_col(id: &str, on_enter: Option<&str>) -> PipelineColumn {
        let mut col = test_col(id);
        if let Some(v) = on_enter {
            col.hooks.insert("on_enter".to_string(), v.to_string());
        }
        col
    }

    fn task_with_wt(id: &str, wt: Option<&str>) -> Task {
        let mut t: Task = serde_json::from_value(serde_json::json!({
            "id": id, "title": "t", "stageId": "backlog"
        }))
        .expect("deserialize task");
        t.worktree_path = wt.map(|s| s.to_string());
        t
    }

    #[test]
    fn test_builtin_marker_detected() {
        let col = hook_col("dev", Some("builtin:create_worktree"));
        assert!(is_builtin_worktree_hook(&col));
        assert!(should_builtin_on_enter(&col));
        let plain = hook_col("dev", Some(".cvh/hooks/on_enter_dev.bat"));
        assert!(!is_builtin_worktree_hook(&plain));
        assert!(!should_builtin_on_enter(&plain));
    }

    #[test]
    fn test_backlog_default_without_explicit_on_enter() {
        let col = hook_col("backlog", None);
        assert!(should_builtin_on_enter(&col));
        // Явный shell-хук у бэклога отменяет дефолт.
        let col_shell = hook_col("backlog", Some(".cvh/hooks/custom.sh"));
        assert!(!should_builtin_on_enter(&col_shell));
        // Не-backlog без хука — не builtin.
        let ready = hook_col("ready", None);
        assert!(!should_builtin_on_enter(&ready));
    }

    #[test]
    fn test_skip_flag_disables_builtin() {
        let col = hook_col("backlog", None);
        let task = task_with_wt("CVH-001", None);
        assert!(needs_builtin_worktree(&col, &task, false));
        assert!(!needs_builtin_worktree(&col, &task, true));
    }

    #[test]
    fn test_shell_priority_existing_worktree_skips_builtin() {
        let col = hook_col("backlog", None);
        // Shell дал путь через stdout → worktree уже есть → builtin скипается.
        let mut task = task_with_wt("CVH-001", None);
        apply_worktree_from_hook_stdout(
            &mut task,
            "some log\nCVH_WORKTREE=/tmp/wt-cvh-001\nCVH_BRANCH=feat/cvh-001\n",
        );
        assert_eq!(task.worktree_path.as_deref(), Some("/tmp/wt-cvh-001"));
        assert_eq!(task.branch.as_deref(), Some("feat/cvh-001"));
        assert!(!needs_builtin_worktree(&col, &task, false));
        // CVH_BRANCH не затирает непустой branch.
        let mut task2 = task_with_wt("CVH-002", None);
        task2.branch = Some("feat/keep".to_string());
        apply_worktree_from_hook_stdout(&mut task2, "CVH_BRANCH=feat/other");
        assert_eq!(task2.branch.as_deref(), Some("feat/keep"));
        // Пустой stdout ничего не меняет → builtin всё ещё нужен.
        let mut task3 = task_with_wt("CVH-003", None);
        apply_worktree_from_hook_stdout(&mut task3, "plain log, no markers\n");
        assert!(task3.worktree_path.is_none());
        assert!(needs_builtin_worktree(&col, &task3, false));
    }

    #[tokio::test]
    async fn test_scaffold_backlog_hook_with_and_without_git() {
        // С .git → backlog получает builtin-маркер.
        let with_git =
            PathBuf::from("target").join(format!("cvh_scaffold_git_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(with_git.join(".git")).await.expect("git dir");
        crate::config::scaffold_cvh_project(&with_git).await.expect("scaffold");
        let raw = tokio::fs::read_to_string(with_git.join(".cvh").join("pipeline.json"))
            .await
            .expect("read pipeline");
        let pipe: crate::config::PipelineConfig =
            serde_json::from_str(&raw).expect("parse pipeline");
        let backlog = pipe.columns.iter().find(|c| c.id == "backlog").expect("backlog");
        assert_eq!(
            backlog.hooks.get("on_enter").map(|s| s.as_str()),
            Some("builtin:create_worktree")
        );
        // Повторный скаффолд не затирает существующий pipeline.json.
        crate::config::scaffold_cvh_project(&with_git).await.expect("rescaffold");
        let raw2 = tokio::fs::read_to_string(with_git.join(".cvh").join("pipeline.json"))
            .await
            .expect("reread");
        assert_eq!(raw, raw2);

        // Без .git → hooks пустые.
        let no_git =
            PathBuf::from("target").join(format!("cvh_scaffold_nogit_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&no_git).await.expect("plain dir");
        crate::config::scaffold_cvh_project(&no_git).await.expect("scaffold");
        let raw_n = tokio::fs::read_to_string(no_git.join(".cvh").join("pipeline.json"))
            .await
            .expect("read pipeline");
        let pipe_n: crate::config::PipelineConfig =
            serde_json::from_str(&raw_n).expect("parse pipeline");
        let backlog_n = pipe_n.columns.iter().find(|c| c.id == "backlog").expect("backlog");
        assert!(backlog_n.hooks.is_empty());

        let _ = tokio::fs::remove_dir_all(&with_git).await;
        let _ = tokio::fs::remove_dir_all(&no_git).await;
    }
    #[tokio::test]
    async fn test_advance_task_and_board_switch_finalization() {
        let base = PathBuf::from("target").join(format!("cvh_adv_switch_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&base).await.expect("test dir");

        let b1_cols = vec![
            crate::config::PipelineColumn {
                id: "backlog".to_string(),
                title: "Backlog".to_string(),
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
            },
            crate::config::PipelineColumn {
                id: "dev".to_string(),
                title: "Dev".to_string(),
                color: None,
                icon: None,
                persona: Some("developer".to_string()),
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
            },
            crate::config::PipelineColumn {
                id: "ready".to_string(),
                title: "Ready".to_string(),
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
                terminal: Some(true),
            },
        ];

        let b2_cols = vec![
            crate::config::PipelineColumn {
                id: "todo".to_string(),
                title: "Todo".to_string(),
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
            },
            crate::config::PipelineColumn {
                id: "in_progress".to_string(),
                title: "In Progress".to_string(),
                color: None,
                icon: None,
                persona: Some("developer".to_string()),
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
            },
            crate::config::PipelineColumn {
                id: "done".to_string(),
                title: "Done".to_string(),
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
                terminal: Some(true),
            },
        ];

        let mut boards_cfg = crate::config::BoardsConfig {
            active_board_id: "dev-board".to_string(),
            boards: vec![
                crate::config::BoardDef {
                    id: "dev-board".to_string(),
                    name: "Development".to_string(),
                    columns: b1_cols,
                },
                crate::config::BoardDef {
                    id: "kanban-board".to_string(),
                    name: "Kanban".to_string(),
                    columns: b2_cols,
                },
            ],
        };
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards");

        // Seed task on dev-board in stage dev
        let task1 = Task {
            id: "CVH-001".to_string(),
            title: "Task 1".to_string(),
            description: "Desc 1".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("dev-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            created_at: 0,
            updated_at: 0,
            handover_summary: None,
            attachments: None,
            cover_image: None,
        };
        save_tasks_to_disk(&base, &[task1]).await.expect("save task1");

        // 1. Normal advance on same board -> transitions to 'ready' with status 'done'
        let adv_res = internal_advance_task(&base, "CVH-001", "Summary for task 1", None).await.expect("advance task1");
        assert!(adv_res.contains("УСПЕШНО"));
        let loaded = load_tasks_from_disk(&base).await;
        let t1 = loaded.iter().find(|t| t.id == "CVH-001").unwrap();
        assert_eq!(t1.stage_id, "ready");
        assert_eq!(t1.status.as_deref(), Some("done"));

        // 2. Board switch scenario:
        // Seed task 2 on dev-board in stage dev
        let task2 = Task {
            id: "CVH-002".to_string(),
            title: "Task 2".to_string(),
            description: "Desc 2".to_string(),
            stage_id: "dev".to_string(),
            board_id: Some("dev-board".to_string()),
            worktree_path: None,
            branch: None,
            verdict: None,
            custom_flow: None,
            status: Some("running".to_string()),
            model: None,
            parent_id: None,
            subtask_index: None,
            waiting_for_subtask_id: None,
            todos: None,
            created_at: 0,
            updated_at: 0,
            handover_summary: None,
            attachments: None,
            cover_image: None,
        };
        save_tasks_to_disk(&base, &[t1.clone(), task2]).await.expect("save task2");

        // Now active board switches to kanban-board
        boards_cfg.active_board_id = "kanban-board".to_string();
        crate::config::save_boards_config(&base, &boards_cfg).await.expect("save boards switch");

        // Advance task 2: active board changed -> task finalizes to terminal column ('done') on kanban-board
        let adv_switch_res = internal_advance_task(&base, "CVH-002", "Summary after board switch", None).await.expect("advance task2 switch");
        assert!(adv_switch_res.contains("УСПЕШНО"));
        assert!(adv_switch_res.contains("финализирована"));

        let loaded2 = load_tasks_from_disk(&base).await;
        let t2 = loaded2.iter().find(|t| t.id == "CVH-002").unwrap();
        assert_eq!(t2.board_id.as_deref(), Some("kanban-board"));
        assert_eq!(t2.stage_id, "done");
        assert_eq!(t2.status.as_deref(), Some("done"));

        let _ = tokio::fs::remove_dir_all(&base).await;
    }
}


