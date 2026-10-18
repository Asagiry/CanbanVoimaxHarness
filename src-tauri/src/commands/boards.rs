use crate::config::{
    atomic_write_file, get_board_template_columns, load_boards_config, load_pipeline_config,
    normalize_pipeline_columns, save_boards_config, save_pipeline_config, BoardDef, PipelineConfig,
};
use crate::commands::state::AppState;
use crate::commands::tasks::{heal_tasks, load_tasks_from_disk, save_tasks_to_disk};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

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
    #[serde(default)]
    pub context_window: Option<u64>,
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

pub fn persona_placeholder_template(role: &str, icon: &str, color: &str) -> String {
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

        let setting = crate::config::load_agent_setting(root, &role).await;
        let context_window = setting.context_window;

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
// IPC Commands: Boards, Pipelines & Personas
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
        let mut tasks = load_tasks_from_disk(&root).await;
        let modified = heal_tasks(&mut tasks, &boards_cfg);
        if modified {
            if let Ok(mut guard) = state.tasks.lock() {
                *guard = tasks.clone();
            }
            let _ = save_tasks_to_disk(&root, &tasks).await;
        }
        let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));
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

    let mut tasks = load_tasks_from_disk(&root).await;
    let modified = heal_tasks(&mut tasks, &config);
    if modified {
        if let Ok(mut guard) = state.tasks.lock() {
            *guard = tasks.clone();
        }
        let _ = save_tasks_to_disk(&root, &tasks).await;
    }
    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

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

    // Verify and heal all tasks:
    // Any task on a deleted/renamed board must be mapped to config.active_board_id,
    // and any task in a removed column mapped to the board's first column (backlog).
    let mut tasks = load_tasks_from_disk(&root).await;
    let modified = heal_tasks(&mut tasks, &config);
    if modified {
        if let Ok(mut guard) = state.tasks.lock() {
            *guard = tasks.clone();
        }
        let _ = save_tasks_to_disk(&root, &tasks).await;
    }
    // Emit both boards:updated and tasks:updated so the UI immediately updates!
    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

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

    // Verify and heal all tasks:
    // Any task on the deleted board must be mapped to config.active_board_id,
    // and any task in a removed column mapped to the target board's first column (backlog).
    let mut tasks = load_tasks_from_disk(&root).await;
    let _modified = heal_tasks(&mut tasks, &config);
    if let Ok(mut guard) = state.tasks.lock() {
        *guard = tasks.clone();
    }
    let _ = save_tasks_to_disk(&root, &tasks).await;
    let _ = app.emit("tasks:updated", serde_json::json!({ "tasks": &tasks }));

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
