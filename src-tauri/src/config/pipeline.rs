//! Canonical pipeline stages, board definitions, and stage normalization logic.

use std::collections::HashMap;
use crate::config::models::{BoardDef, BoardsConfig, PipelineConfig, PipelineStage};

pub fn default_pipeline_config() -> PipelineConfig {
    PipelineConfig {
        id: "default-engineering".to_string(),
        title: "cvh-board".to_string(),
        tool_permissions: None,
        stages: get_canonical_board_stages(),
        boards: None,
        active_board_id: None,
    }
}

pub fn default_board_def() -> BoardDef {
    BoardDef {
        id: "development".to_string(),
        name: "cvh-board".to_string(),
        stages: get_canonical_board_stages(),
    }
}

pub fn default_boards_config() -> BoardsConfig {
    BoardsConfig {
        active_board_id: "development".to_string(),
        boards: vec![default_board_def()],
    }
}

pub fn get_canonical_board_stages() -> Vec<PipelineStage> {
    vec![
        PipelineStage {
            id: "backlog".to_string(),
            title: "Backlog".to_string(),
            color: Some("#64748b".to_string()),
            icon: Some("Inbox".to_string()),
            persona: None,
            wip_limit: None,
            on_enter_script: None,
            context_window: None,
            model_override: None,
            reasoning_effort: None,
            temperature: None,
            gatekeeper_sensor: None,
            allowed_tools: Some(vec![
                "read_file".to_string(),
                "glob".to_string(),
                "grep".to_string(),
                "web_search".to_string(),
                "web_fetch".to_string(),
            ]),
            hooks: HashMap::new(),
            auto_run: None,
            terminal: None,
            is_intake: Some(true),
        },
        PipelineStage {
            id: "dev".to_string(),
            title: "CVH Development".to_string(),
            color: Some("#6366f1".to_string()),
            icon: Some("Bot".to_string()),
            persona: Some("cvh-dev".to_string()),
            wip_limit: None,
            on_enter_script: None,
            context_window: None,
            model_override: None,
            reasoning_effort: None,
            temperature: Some(0.2),
            gatekeeper_sensor: None,
            allowed_tools: Some(vec![
                "execute_command".to_string(),
                "read_file".to_string(),
                "edit_file".to_string(),
                "write_file".to_string(),
                "glob".to_string(),
                "grep".to_string(),
                "web_search".to_string(),
            ]),
            hooks: HashMap::new(),
            auto_run: None,
            terminal: None,
            is_intake: None,
        },
        PipelineStage {
            id: "ready".to_string(),
            title: "Ready".to_string(),
            color: Some("#10b981".to_string()),
            icon: Some("CheckCircle2".to_string()),
            persona: None,
            wip_limit: None,
            on_enter_script: None,
            context_window: None,
            model_override: None,
            reasoning_effort: None,
            temperature: None,
            gatekeeper_sensor: None,
            allowed_tools: Some(vec![
                "read_file".to_string(),
                "glob".to_string(),
                "grep".to_string(),
            ]),
            hooks: HashMap::new(),
            auto_run: None,
            terminal: Some(true),
            is_intake: None,
        },
    ]
}

pub fn get_canonical_board_columns() -> Vec<PipelineStage> {
    get_canonical_board_stages()
}

pub fn get_board_template_stages(template: &str) -> Vec<PipelineStage> {
    match template.to_lowercase().as_str() {
        "modeler" | "3d" | "3d-modeler" => vec![
            PipelineStage {
                id: "concept".to_string(),
                title: "Концепт".to_string(),
                color: Some("#a855f7".to_string()),
                icon: Some("Lightbulb".to_string()),
                persona: Some("concept-artist".to_string()),
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: Some(0.4),
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "web_search".to_string(),
                    "web_fetch".to_string(),
                    "read_file".to_string(),
                ]),
                hooks: HashMap::new(),
                auto_run: None,
                terminal: None,
                is_intake: Some(true),
            },
            PipelineStage {
                id: "modeling".to_string(),
                title: "Моделирование".to_string(),
                color: Some("#3b82f6".to_string()),
                icon: Some("Box".to_string()),
                persona: Some("modeler".to_string()),
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: Some(0.2),
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "execute_command".to_string(),
                    "read_file".to_string(),
                    "edit_file".to_string(),
                    "write_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                ]),
                hooks: HashMap::new(),
                auto_run: None,
                terminal: None,
                is_intake: None,
            },
            PipelineStage {
                id: "texturing".to_string(),
                title: "Текстурирование".to_string(),
                color: Some("#f59e0b".to_string()),
                icon: Some("Palette".to_string()),
                persona: Some("texture-artist".to_string()),
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: Some(0.2),
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "execute_command".to_string(),
                    "read_file".to_string(),
                    "edit_file".to_string(),
                    "write_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                ]),
                hooks: HashMap::new(),
                auto_run: None,
                terminal: None,
                is_intake: None,
            },
            PipelineStage {
                id: "ready".to_string(),
                title: "Готово".to_string(),
                color: Some("#10b981".to_string()),
                icon: Some("CheckCircle2".to_string()),
                persona: None,
                wip_limit: None,
                on_enter_script: None,
                context_window: None,
                model_override: None,
                reasoning_effort: None,
                temperature: None,
                gatekeeper_sensor: None,
                allowed_tools: Some(vec![
                    "read_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                ]),
                hooks: HashMap::new(),
                auto_run: None,
                terminal: Some(true),
                is_intake: None,
            },
        ],
        _ => get_canonical_board_stages(),
    }
}

pub fn get_board_template_columns(template: &str) -> Vec<PipelineStage> {
    get_board_template_stages(template)
}

/// Normalizes pipeline stages according to dynamic stage invariants:
/// - Validates unique stage IDs (duplicate IDs return an Err).
/// - If empty, falls back to canonical board stages.
/// - Preserves user stage ordering verbatim without injecting synthetic backlog/ready.
/// - Sets stage 0 `is_intake = Some(true)` if unset.
/// - Sets final stage `terminal = Some(true)`.
pub fn normalize_pipeline_stages(
    stages: Vec<PipelineStage>,
) -> Result<Vec<PipelineStage>, String> {
    let mut seen = std::collections::HashSet::new();
    for c in &stages {
        if !seen.insert(c.id.clone()) {
            return Err(format!(
                "Дублирующийся id колонки: '{}' — id колонок должны быть уникальными",
                c.id
            ));
        }
    }

    if stages.is_empty() {
        return Ok(get_canonical_board_stages());
    }

    let mut out = stages;
    if out[0].is_intake.is_none() {
        out[0].is_intake = Some(true);
    }
    let last_idx = out.len() - 1;
    out[last_idx].terminal = Some(true);

    Ok(out)
}

pub fn normalize_pipeline_columns(
    columns: Vec<PipelineStage>,
) -> Result<Vec<PipelineStage>, String> {
    normalize_pipeline_stages(columns)
}

/// Minimal fresh-project pipeline: 3 stages — `backlog` (no persona, first),
/// `dev` (CVH Development with `cvh-dev` persona, middle), and `ready` (terminal, last).
pub fn minimal_pipeline_config() -> PipelineConfig {
    default_pipeline_config()
}
