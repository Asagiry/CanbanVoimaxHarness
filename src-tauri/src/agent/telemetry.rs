//! Telemetry computation utilities for agent turns and subagents.
//! Adheres to ub-mode (Dryer & Zero-Risk Pure Extraction) and Telemetry Accuracy Invariants.

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Snapshot of turn telemetry metrics (cache hit rate and tokens per second).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TurnTelemetry {
    pub cache_hit_rate: f32,
    pub tps: f32,
}

/// Compute safe prompt cache hit rate percentage according to telemetry invariants.
///
/// Guaranteed to never produce NaN, infinite or negative values.
#[inline]
pub fn compute_cache_hit_rate(cache_read_tokens: u64, total_input: u64) -> f32 {
    if total_input > 0 {
        let rate = (cache_read_tokens as f32 / total_input as f32) * 100.0;
        if rate.is_nan() || rate.is_infinite() || rate < 0.0 {
            0.0
        } else {
            rate.min(100.0)
        }
    } else {
        0.0
    }
}

/// Compute active token generation throughput (tok/s).
///
/// Generation time must exceed minimum duration (50ms) and produce at least 1 token.
#[inline]
pub fn compute_tps(duration_secs: f64, output_tokens: u64) -> f32 {
    if duration_secs > 0.05 && output_tokens > 0 {
        let val = (output_tokens as f64 / duration_secs) as f32;
        if val.is_nan() || val.is_infinite() || val < 0.0 {
            0.0
        } else {
            val
        }
    } else {
        0.0
    }
}

/// Convenient helper for f32 duration_secs.
#[inline]
pub fn compute_tps_f32(duration_secs: f32, output_tokens: u64) -> f32 {
    compute_tps(duration_secs as f64, output_tokens)
}

/// Calculates timing metrics (duration_secs, duration_ms, ttft_ms) from turn instants.
#[inline]
pub fn compute_timing_metrics(
    turn_start_instant: Instant,
    first_token_instant: Option<Instant>,
    last_token_instant: Option<Instant>,
) -> (f64, f64, f64) {
    let start_time = first_token_instant.unwrap_or(turn_start_instant);
    let end_time = last_token_instant.unwrap_or_else(Instant::now);
    let duration_secs = end_time.duration_since(start_time).as_secs_f64();
    let duration_ms = duration_secs * 1000.0;
    let ttft_ms = first_token_instant
        .map(|t| t.duration_since(turn_start_instant).as_secs_f64() * 1000.0)
        .unwrap_or(0.0);
    (duration_secs, duration_ms, ttft_ms)
}

/// Unified telemetry calculation for an agent or subagent turn.
#[inline]
pub fn calculate_turn_telemetry(
    total_input: u64,
    cache_read_tokens: u64,
    output_tokens: u64,
    duration_secs: f64,
) -> TurnTelemetry {
    TurnTelemetry {
        cache_hit_rate: compute_cache_hit_rate(cache_read_tokens, total_input),
        tps: compute_tps(duration_secs, output_tokens),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_cache_hit_rate_zero() {
        assert_eq!(compute_cache_hit_rate(0, 0), 0.0);
        assert_eq!(compute_cache_hit_rate(100, 0), 0.0);
    }

    #[test]
    fn test_compute_cache_hit_rate_normal() {
        let rate = compute_cache_hit_rate(50, 100);
        assert!((rate - 50.0).abs() < 1e-4);

        let rate_full = compute_cache_hit_rate(100, 100);
        assert!((rate_full - 100.0).abs() < 1e-4);

        let rate_capped = compute_cache_hit_rate(150, 100);
        assert_eq!(rate_capped, 100.0);
    }

    #[test]
    fn test_compute_tps_thresholds() {
        assert_eq!(compute_tps(0.0, 10), 0.0);
        assert_eq!(compute_tps(0.05, 10), 0.0);
        assert_eq!(compute_tps(1.0, 0), 0.0);

        let tps = compute_tps(1.0, 50);
        assert!((tps - 50.0).abs() < 1e-4);

        let tps_fast = compute_tps(0.1, 10);
        assert!((tps_fast - 100.0).abs() < 1e-4);
    }

    #[test]
    fn test_calculate_turn_telemetry() {
        let telem = calculate_turn_telemetry(200, 50, 100, 2.0);
        assert!((telem.cache_hit_rate - 25.0).abs() < 1e-4);
        assert!((telem.tps - 50.0).abs() < 1e-4);
    }

    #[test]
    fn test_compute_timing_metrics() {
        let start = Instant::now();
        let (duration_secs, duration_ms, ttft_ms) = compute_timing_metrics(start, None, None);
        assert!(duration_secs >= 0.0);
        assert!(duration_ms >= 0.0);
        assert_eq!(ttft_ms, 0.0);
    }
}
