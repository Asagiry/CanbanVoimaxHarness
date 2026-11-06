use crate::db::get_db_conn;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    #[serde(default = "default_entry_id")]
    pub id: String,
    #[serde(rename = "taskId", alias = "task_id")]
    pub task_id: String,
    #[serde(default, rename = "commitHash", alias = "commit_hash")]
    pub commit_hash: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub files: Option<Vec<String>>,
    #[serde(default, rename = "createdAt", alias = "created_at")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryRecord {
    #[serde(default = "default_entry_id")]
    pub id: String,
    #[serde(rename = "taskId", alias = "task_id")]
    pub task_id: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default, rename = "inputTokens", alias = "input_tokens")]
    pub input_tokens: i64,
    #[serde(default, rename = "outputTokens", alias = "output_tokens")]
    pub output_tokens: i64,
    #[serde(default, rename = "cachedTokens", alias = "cached_tokens")]
    pub cached_tokens: i64,
    #[serde(default)]
    pub tps: f64,
    #[serde(default, rename = "ttftMs", alias = "ttft_ms")]
    pub ttft_ms: f64,
    #[serde(default, rename = "durationMs", alias = "duration_ms")]
    pub duration_ms: f64,
    #[serde(default, rename = "createdAt", alias = "created_at")]
    pub created_at: i64,
}

fn default_entry_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySummary {
    #[serde(default)]
    pub total_input_tokens: i64,
    #[serde(default)]
    pub total_cached_tokens: i64,
    #[serde(default)]
    pub total_output_tokens: i64,
    #[serde(default)]
    pub total_tokens: i64,
    #[serde(default)]
    pub cache_hit_rate: f64,
    #[serde(default)]
    pub turn_count: i64,
}

/// Records a telemetry metric row into SQLite table.
pub fn db_record_telemetry(root: &Path, record: &TelemetryRecord) -> Result<(), String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let id = if record.id.trim().is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        record.id.clone()
    };
    let created_at = if record.created_at == 0 {
        chrono::Utc::now().timestamp_millis()
    } else {
        record.created_at
    };
    conn.execute(
        "INSERT INTO telemetry (
            id, task_id, model, input_tokens, output_tokens, cached_tokens,
            tps, ttft_ms, duration_ms, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            record.task_id,
            record.model,
            record.input_tokens,
            record.output_tokens,
            record.cached_tokens,
            record.tps,
            record.ttft_ms,
            record.duration_ms,
            created_at,
        ],
    )
    .map_err(|e| format!("Failed to record telemetry: {}", e))?;
    Ok(())
}

/// Retrieves telemetry records for a specific task.
pub fn db_get_telemetry_for_task(root: &Path, task_id: &str) -> Result<Vec<TelemetryRecord>, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, model, input_tokens, output_tokens, cached_tokens, tps, ttft_ms, duration_ms, created_at
             FROM telemetry
             WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)
             ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Prepare failed: {}", e))?;

    let rows = stmt
        .query_map(params![task_id], |r| {
            Ok(TelemetryRecord {
                id: r.get(0)?,
                task_id: r.get(1)?,
                model: r.get(2)?,
                input_tokens: r.get(3)?,
                output_tokens: r.get(4)?,
                cached_tokens: r.get(5)?,
                tps: r.get(6)?,
                ttft_ms: r.get(7)?,
                duration_ms: r.get(8)?,
                created_at: r.get(9)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    Ok(rows.flatten().collect())
}

/// Cumulative telemetry summary for a task and its sub-agents.
///
/// Invariant: telemetry rows are written exactly once per turn — under the
/// parent `task_id` in `runner.rs` and under `subagent_id` (`<parent>.sub-*`)
/// in `react_engine.rs`. The duplicate emission under `tool_call_id` is
/// event-only and never persisted, so summing `task_id = parent OR
/// task_id LIKE parent || '.sub-%'` cannot double-count.
pub fn db_get_telemetry_summary(root: &Path, task_id: &str) -> Result<TelemetrySummary, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let sub_prefix = format!("{}.sub-%", task_id);
    let (total_input, total_cached, total_output, turn_count): (i64, i64, i64, i64) = conn
        .query_row(
            "SELECT COALESCE(SUM(input_tokens), 0), COALESCE(SUM(cached_tokens), 0), \
             COALESCE(SUM(output_tokens), 0), COUNT(*) \
             FROM telemetry \
             WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1) \
                OR task_id LIKE ?2 OR LOWER(task_id) LIKE LOWER(?2)",
            params![task_id, sub_prefix],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| format!("Failed to summarize telemetry: {}", e))?;
    let total_input = total_input.max(0);
    let total_cached = total_cached.max(0).min(total_input);
    let total_output = total_output.max(0);
    let cache_hit_rate = if total_input > 0 {
        ((total_cached as f64) / (total_input as f64) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    Ok(TelemetrySummary {
        total_input_tokens: total_input,
        total_cached_tokens: total_cached,
        total_output_tokens: total_output,
        total_tokens: total_input + total_output,
        cache_hit_rate,
        turn_count: turn_count.max(0),
    })
}

/// Records or updates an engineering ledger entry in SQLite.
pub fn db_record_ledger_entry(root: &Path, entry: &LedgerEntry) -> Result<(), String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let id = if entry.id.trim().is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        entry.id.clone()
    };
    let created_at = if entry.created_at == 0 {
        chrono::Utc::now().timestamp_millis()
    } else {
        entry.created_at
    };
    let files_json = entry.files.as_ref().and_then(|f| serde_json::to_string(f).ok());
    conn.execute(
        "INSERT INTO task_ledger (
            id, task_id, commit_hash, summary, verdict, author, files_json, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(id) DO UPDATE SET
            task_id = excluded.task_id,
            commit_hash = excluded.commit_hash,
            summary = excluded.summary,
            verdict = excluded.verdict,
            author = excluded.author,
            files_json = excluded.files_json,
            created_at = excluded.created_at",
        params![
            id,
            entry.task_id,
            entry.commit_hash,
            entry.summary,
            entry.verdict,
            entry.author,
            files_json,
            created_at,
        ],
    )
    .map_err(|e| format!("Failed to record ledger entry: {}", e))?;
    Ok(())
}

/// Retrieves ledger entries for a specific task.
pub fn db_get_ledger_for_task(root: &Path, task_id: &str) -> Result<Vec<LedgerEntry>, String> {
    let conn = get_db_conn(root).map_err(|e| format!("Failed to open database: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, commit_hash, summary, verdict, author, files_json, created_at
             FROM task_ledger
             WHERE task_id = ?1 OR LOWER(task_id) = LOWER(?1)
             ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Prepare failed: {}", e))?;

    let rows = stmt
        .query_map(params![task_id], |r| {
            let files_raw: Option<String> = r.get(6)?;
            let files = files_raw.and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok());
            Ok(LedgerEntry {
                id: r.get(0)?,
                task_id: r.get(1)?,
                commit_hash: r.get(2)?,
                summary: r.get(3)?,
                verdict: r.get(4)?,
                author: r.get(5)?,
                files,
                created_at: r.get(7)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    Ok(rows.flatten().collect())
}
