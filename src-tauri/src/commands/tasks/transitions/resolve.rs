//! Board & Pipeline Column Resolution Helpers for Transitions (§ 2.18, § 2.2)

use std::path::Path;
use crate::agent::Task;
use crate::config::{load_pipeline_config, BoardDef, BoardsConfig, PipelineStage};
use crate::commands::tasks::healing::stage_id_matches;

/// Resolves the effective board and assigned board_id for a stage transition (§ 2.18 ZERO INVISIBLE TASKS).
pub fn resolve_board_for_stage(
    target_stage_id: &str,
    current_board_id: Option<&str>,
    board_id_override: Option<&str>,
    boards_cfg: Option<&BoardsConfig>,
) -> (Option<BoardDef>, Option<String>) {
    let mut effective_board: Option<BoardDef> = None;
    let mut assigned_board_id = current_board_id.map(|s| s.to_string());

    if let Some(bc) = boards_cfg {
        let active_board = bc.boards.iter().find(|b| b.id == bc.active_board_id).cloned();
        let override_board = board_id_override.and_then(|bid| {
            bc.boards.iter().find(|b| b.id == bid).cloned()
        });
        let current_board = current_board_id.and_then(|bid| {
            bc.boards.iter().find(|b| b.id == bid).cloned()
        });

        let board_has_target = |b: &BoardDef| -> bool {
            b.stages.iter().any(|c| c.id == target_stage_id || stage_id_matches(&c.id, target_stage_id))
        };

        // 1. Explicit override board requested by caller (e.g. active board during user move)
        if let Some(ref ob) = override_board {
            if board_has_target(ob) {
                effective_board = Some(ob.clone());
                assigned_board_id = Some(ob.id.clone());
            }
        }

        // 2. Task's current board if it contains the target stage (preserves task on its board)
        if effective_board.is_none() {
            if let Some(ref cb) = current_board {
                if board_has_target(cb) {
                    effective_board = Some(cb.clone());
                }
            }
        }

        // 3. Active board if it contains the target stage
        if effective_board.is_none() {
            if let Some(ref ab) = active_board {
                if board_has_target(ab) {
                    effective_board = Some(ab.clone());
                    assigned_board_id = Some(ab.id.clone());
                }
            }
        }

        // 4. Any board that defines the target stage (cross-board pipeline advance)
        if effective_board.is_none() {
            let any_with_target = bc.boards.iter().find(|b| board_has_target(b)).cloned();
            if let Some(ref b) = any_with_target {
                effective_board = Some(b.clone());
                assigned_board_id = Some(b.id.clone());
            } else {
                effective_board = active_board.or_else(|| bc.boards.first().cloned());
                if let Some(ref b) = effective_board {
                    assigned_board_id = Some(b.id.clone());
                } else {
                    assigned_board_id = Some(bc.active_board_id.clone());
                }
            }
        }
    }

    (effective_board, assigned_board_id)
}

/// Resolves source and target pipeline columns during a stage transition.
pub async fn resolve_source_and_target_columns(
    root: &Path,
    effective_board: Option<&BoardDef>,
    source_stage_id: &str,
    target_stage_id: &str,
) -> Result<(Option<PipelineStage>, PipelineStage), String> {
    let source_col: Option<PipelineStage> = {
        let from_board = effective_board.and_then(|b| {
            b.stages.iter().find(|c| stage_id_matches(&c.id, source_stage_id)).cloned()
        });
        match from_board {
            Some(c) => Some(c),
            None => load_pipeline_config(root)
                .await
                .ok()
                .and_then(|p| p.stages.into_iter().find(|c| stage_id_matches(&c.id, source_stage_id))),
        }
    };

    let target_col = {
        let col_from_board = effective_board.and_then(|b| {
            b.stages.iter().find(|c| stage_id_matches(&c.id, target_stage_id)).cloned()
        });
        match col_from_board {
            Some(c) => c,
            None => {
                let pipeline = load_pipeline_config(root)
                    .await
                    .unwrap_or_else(|_| crate::config::default_pipeline_config());
                let col_from_pipeline = pipeline
                    .stages
                    .iter()
                    .find(|c| stage_id_matches(&c.id, target_stage_id))
                    .cloned();

                match col_from_pipeline {
                    Some(c) => c,
                    None => {
                        let is_explicit_terminal_request = target_stage_id.trim().is_empty()
                            || stage_id_matches(target_stage_id, "ready")
                            || stage_id_matches(target_stage_id, "done")
                            || stage_id_matches(target_stage_id, "terminal");

                        let terminal_opt = if is_explicit_terminal_request {
                            effective_board.and_then(|b| {
                                b.stages.iter().find(|c| c.terminal == Some(true) || stage_id_matches(&c.id, "ready") || stage_id_matches(&c.id, "done")).cloned()
                                    .or_else(|| b.stages.last().cloned())
                            }).or_else(|| {
                                pipeline.stages.iter().find(|c| {
                                    c.terminal == Some(true)
                                        || stage_id_matches(&c.id, "ready")
                                        || stage_id_matches(&c.id, "done")
                                        || stage_id_matches(&c.id, "terminal")
                                }).cloned()
                            })
                        } else {
                            None
                        };

                        if let Some(term) = terminal_opt {
                            term
                        } else {
                            let mut available_stages: Vec<String> = Vec::new();
                            if let Some(b) = effective_board {
                                for c in &b.stages {
                                    if !available_stages.contains(&c.id) {
                                        available_stages.push(c.id.clone());
                                    }
                                }
                            }
                            for c in &pipeline.stages {
                                if !available_stages.contains(&c.id) {
                                    available_stages.push(c.id.clone());
                                }
                            }
                            return Err(format!(
                                "Target stage '{}' not found in active board or pipeline columns. Available: {:?}",
                                target_stage_id,
                                available_stages
                            ));
                        }
                    }
                }
            }
        }
    };

    Ok((source_col, target_col))
}

/// Resolves the sequence of pipeline columns for task advancement.
pub async fn resolve_advance_columns(
    root: &Path,
    current_task: &Task,
    boards_cfg: Option<&BoardsConfig>,
) -> Result<Vec<PipelineStage>, String> {
    let from_board = boards_cfg.and_then(|bc| {
        if let Some(ref bid) = current_task.board_id {
            if let Some(b) = bc.boards.iter().find(|b| &b.id == bid) {
                if b.stages.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                    return Some(b.stages.clone());
                }
            }
        }
        if let Some(b) = bc.boards.iter().find(|b| b.id == bc.active_board_id) {
            if b.stages.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                return Some(b.stages.clone());
            }
        }
        if let Some(b) = bc.boards.iter().find(|b| {
            b.stages.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id))
        }) {
            return Some(b.stages.clone());
        }
        bc.boards
            .iter()
            .find(|b| b.id == bc.active_board_id)
            .or_else(|| bc.boards.first())
            .map(|b| b.stages.clone())
    });

    match from_board {
        Some(cols) => {
            if cols.iter().any(|c| stage_id_matches(&c.id, &current_task.stage_id)) {
                Ok(cols)
            } else {
                let pipeline = load_pipeline_config(root)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(pipeline.stages)
            }
        }
        None => {
            let pipeline = load_pipeline_config(root)
                .await
                .map_err(|e| e.to_string())?;
            Ok(pipeline.stages)
        }
    }
}
