/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Small formatting helpers (parity with the React `util/format.ts` + the
//! Overview's `ago`). Time math rides `js_sys::Date` — wall-clock in the
//! browser.

/// Milliseconds since the epoch for an RFC3339-ish timestamp, via the
/// browser's parser (handles the server's ISO strings).
fn parse_ms(ts: &str) -> Option<f64> {
    let ms = js_sys::Date::parse(ts);
    if ms.is_nan() {
        None
    } else {
        Some(ms)
    }
}

/// `1.2s` / `3m 04s` style duration between two timestamps; em-dash when
/// either side is missing/unparseable.
pub fn format_duration(started: Option<&str>, completed: Option<&str>) -> String {
    let (Some(s), Some(c)) = (started.and_then(parse_ms), completed.and_then(parse_ms)) else {
        return "—".to_string();
    };
    let ms = c - s;
    if ms < 0.0 {
        return "—".to_string();
    }
    let secs = ms / 1000.0;
    if secs < 60.0 {
        return format!("{secs:.1}s");
    }
    let m = (secs / 60.0).floor() as u64;
    let rem = (secs % 60.0).floor() as u64;
    if m < 60 {
        return format!("{m}m {rem:02}s");
    }
    let h = m / 60;
    format!("{h}h {:02}m", m % 60)
}

/// `42s ago` / `3m ago` / `2h ago` / `5d ago`; empty when unknown.
pub fn ago(ts: Option<&str>) -> String {
    let Some(then) = ts.and_then(parse_ms) else {
        return String::new();
    };
    let ms = js_sys::Date::now() - then;
    if ms.is_nan() || ms < 0.0 {
        return String::new();
    }
    let s = (ms / 1000.0).floor() as u64;
    if s < 60 {
        return format!("{s}s ago");
    }
    let m = s / 60;
    if m < 60 {
        return format!("{m}m ago");
    }
    let h = m / 60;
    if h < 24 {
        return format!("{h}h ago");
    }
    format!("{}d ago", h / 24)
}

/// First 8 chars of an id (the run-id chip convention).
pub fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

use aurora_leptos::tokens::token;

/// Health-state → accent color (app vocabulary; the pack renders).
pub fn health_color(state: &str) -> &'static str {
    match state.to_lowercase().as_str() {
        // socket_only is availability, not activity: the ingest endpoint is
        // up with no source attached — "healthy by definition" per the core
        // AccumulatorHealth enum (UAT round 4, T-0938).
        "running" | "live" | "ok" | "healthy" | "socket_only" => token::OK,
        // disconnected = was live, socket still up, retrying the source.
        "degraded" | "stale" | "paused" | "disconnected" | "starting" | "connecting" => token::GOLD,
        "error" | "failed" | "stopped" | "dead" => token::BAD,
        _ => token::MUTED,
    }
}

/// Node-kind → accent color (graph legend vocabulary).
pub fn node_kind_color(kind: &str) -> &'static str {
    match kind {
        "accumulator" => token::TEAL,
        "reactor" => token::VIOLET,
        _ => token::ICE,
    }
}

/// Events/min derived from a monotonic counter (the React
/// `useGraphThroughput` hook): remembers the last (total, at) per name and
/// reports the delta rate once two samples exist.
#[derive(Default, Clone)]
pub struct Throughput {
    samples: std::collections::HashMap<String, (f64, f64, Option<f64>)>,
}

impl Throughput {
    /// Feed a fresh `total` for `name`; returns the current ~per-minute rate.
    pub fn sample(&mut self, name: &str, total: f64) -> Option<f64> {
        let now = js_sys::Date::now();
        match self.samples.get(name).copied() {
            Some((last_total, last_at, last_rate)) => {
                let dt_min = (now - last_at) / 60_000.0;
                if dt_min <= 0.0 {
                    return last_rate;
                }
                let rate = ((total - last_total).max(0.0) / dt_min).round();
                self.samples
                    .insert(name.to_string(), (total, now, Some(rate)));
                Some(rate)
            }
            None => {
                self.samples.insert(name.to_string(), (total, now, None));
                None
            }
        }
    }
}
