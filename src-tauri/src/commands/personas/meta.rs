//! Persona styling, palette constants, icons, and frontmatter helpers.

use crate::config::atomic_write_file;
use std::collections::HashSet;
use std::path::Path;

/// Канонический пак имен SVG-иконок для агентов
pub const AGENT_ICONS: &[&str] = &[
    "Bot", "Brain", "Cpu", "Code2", "ShieldCheck", "Sparkles",
    "History", "Terminal", "Zap", "Flame", "Radar", "Workflow", "Boxes",
    "Wrench", "Rocket", "Compass", "Crosshair", "Layers",
];

/// Каноническая палитра цветов для агентов.
/// Серый (#64748b, #6b7280) и зеленый (#10b981, #22c55e) исключены,
/// так как они навсегда зарезервированы за Backlog и Ready.
pub const AGENT_PALETTE: &[&str] = &[
    "#f59e0b", "#a855f7", "#0284c7", "#3b82f6", "#6366f1",
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
    if trimmed.eq_ignore_ascii_case("general") {
        return "#6366f1".to_string();
    }
    if trimmed.eq_ignore_ascii_case("oneshot") {
        return "#0284c7".to_string();
    }
    let hash: usize = trimmed.bytes().fold(5381usize, |acc, b| {
        acc.wrapping_mul(33).wrapping_add(b as usize)
    });
    AGENT_PALETTE[hash % AGENT_PALETTE.len()].to_string()
}

/// Дефолтная иконка для стадии/колонки по её идентификатору
pub fn default_stage_icon(stage_id: &str) -> String {
    match stage_id {
        "backlog" => "Inbox".to_string(),
        "dev" => "Code2".to_string(),
        "review" => "ShieldCheck".to_string(),
        "retro" => "Sparkles".to_string(),
        "ready" => "CheckCircle2".to_string(),
        _ => "Layers".to_string(),
    }
}

pub fn default_column_icon(col_id: &str) -> String {
    default_stage_icon(col_id)
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

/// Считывает иконку персоны из каталога (setting -> persona.md -> agent.md -> persona.json).
/// Для встроенного агента `cvh-dev` по умолчанию возвращает "Cpu".
pub async fn read_persona_icon_from_dir(role_dir: &Path, role: &str) -> Option<String> {
    let setting_file = role_dir.join("setting");
    if let Ok(content) = tokio::fs::read_to_string(&setting_file).await {
        if let Some(icon) = extract_persona_icon_from_json(&content) {
            return Some(icon);
        }
    }
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

/// Сохраняет иконку персоны в `persona.md` (или `agent.md`) и синхронизирует в `setting`.
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

    // Двусторонняя синхронизация с файлом setting, если он существует
    let setting_file = role_dir.join("setting");
    if setting_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&setting_file).await {
            let mut val: serde_json::Value = serde_json::from_str(&content)
                .unwrap_or_else(|_| serde_json::json!({}));
            if !val.is_object() {
                val = serde_json::json!({});
            }
            val["icon"] = serde_json::json!(icon);
            if let Ok(pretty) = serde_json::to_string_pretty(&val) {
                let _ = atomic_write_file(&setting_file, &pretty).await;
            }
        }
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

/// Считывает цвет персоны из каталога (setting -> persona.md -> agent.md -> persona.json).
/// Для встроенного агента `cvh-dev` по умолчанию возвращает "#6366f1".
pub async fn read_persona_color_from_dir(role_dir: &Path, role: &str) -> Option<String> {
    let setting_file = role_dir.join("setting");
    if let Ok(content) = tokio::fs::read_to_string(&setting_file).await {
        if let Some(color) = extract_persona_color_from_json(&content) {
            return Some(color);
        }
    }
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

/// Сохраняет цвет персоны в `persona.md` (или `agent.md`) и синхронизирует в `setting`.
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

    // Двусторонняя синхронизация с файлом setting, если он существует
    let setting_file = role_dir.join("setting");
    if setting_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&setting_file).await {
            let mut val: serde_json::Value = serde_json::from_str(&content)
                .unwrap_or_else(|_| serde_json::json!({}));
            if !val.is_object() {
                val = serde_json::json!({});
            }
            val["color"] = serde_json::json!(color);
            if let Ok(pretty) = serde_json::to_string_pretty(&val) {
                let _ = atomic_write_file(&setting_file, &pretty).await;
            }
        }
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
        "---\nname: {role}\nicon: {icon}\ncolor: {color}\n---\n\n# Role Persona: {role}\n\nIdentity & Scope:\n- Core Mission: <describe the primary responsibility and objectives of this role>\n- Permitted Operations: <list allowed tools, inspection, and verification scopes>\n- Prohibitions: <list strict prohibitions, e.g. never write code without tests, never commit broken builds>\n\nGuidelines:\n- Work strictly inside the isolated git worktree.\n- State only what runtime sensors prove.\n"
    )
}
