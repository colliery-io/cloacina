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

//! Pure-SVG/CSS data-viz (CLOACI-T-0935): TaskGantt and RunHeatmap, ports of
//! the React components onto Aurora tokens. No JS chart library — bars are
//! divs on a shared axis, exactly like the originals. Candidates to upstream
//! into aurora-leptos once stable (pack contract: generic rendering, app
//! vocab as data).

use aurora_leptos::tokens::{status_color, token};
use leptos::prelude::*;

use cloacina_api_types::{ExecutionSummary, TaskExecutionDetail};

use crate::util::format_duration;

const MONO: &str = "'IBM Plex Mono', monospace";

fn parse_ms(ts: &str) -> Option<f64> {
    let ms = js_sys::Date::parse(ts);
    if ms.is_nan() {
        None
    } else {
        Some(ms)
    }
}

fn is_terminal(status: &str) -> bool {
    matches!(
        status.to_lowercase().as_str(),
        "completed" | "failed" | "cancelled" | "canceled"
    )
}

fn local_id(name: &str) -> String {
    name.rsplit("::").next().unwrap_or(name).to_string()
}

fn fmt_ms(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{ms:.0}ms")
    } else if ms < 60_000.0 {
        format!("{:.1}s", ms / 1000.0)
    } else {
        format!("{:.1}m", ms / 60_000.0)
    }
}

/// Per-run task Gantt (CLOACI-I-0124, the Airflow "Gantt" tab): one bar per
/// task on a shared time axis; left = start offset, width = duration, color =
/// status; the wall-vs-work footer is the parallelism hint. Rows keep DAG
/// order and never reshuffle mid-run.
#[component]
pub fn TaskGantt(
    tasks: Vec<TaskExecutionDetail>,
    /// DAG topological rank (task id → position) for fixed nominal order.
    #[prop(optional)]
    order: Option<std::collections::HashMap<String, usize>>,
) -> impl IntoView {
    let now = js_sys::Date::now();
    struct Bar {
        id: String,
        name: String,
        full_name: String,
        status: String,
        start: f64,
        end: f64,
        running: bool,
    }
    let mut bars: Vec<Bar> = tasks
        .iter()
        .filter_map(|t| {
            let start = parse_ms(t.started_at.as_deref().unwrap_or(&t.created_at))?;
            let running = !is_terminal(&t.status);
            let end = t
                .completed_at
                .as_deref()
                .or(if running { None } else { Some(&t.updated_at) })
                .and_then(parse_ms)
                .unwrap_or(now)
                .max(start);
            Some(Bar {
                id: t.id.clone(),
                name: local_id(&t.task_name),
                full_name: t.task_name.clone(),
                status: t.status.clone(),
                start,
                end,
                running,
            })
        })
        .collect();

    if bars.is_empty() {
        return view! {
            <span style:color="var(--muted)" style:font-size="13px">
                "No task timing recorded for this run yet."
            </span>
        }
        .into_any();
    }

    let rank = |b: &Bar| {
        order
            .as_ref()
            .and_then(|o| o.get(&b.name))
            .copied()
            .unwrap_or(usize::MAX)
    };
    bars.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a.start.total_cmp(&b.start)));

    let t0 = bars.iter().map(|b| b.start).fold(f64::INFINITY, f64::min);
    let t1 = bars.iter().map(|b| b.end).fold(f64::NEG_INFINITY, f64::max);
    let span = (t1 - t0).max(1.0);
    let wall = t1 - t0;
    let work: f64 = bars.iter().map(|b| b.end - b.start).sum();

    view! {
        <div>
            <div style:display="flex" style:flex-direction="column" style:gap="4px">
                {bars
                    .into_iter()
                    .map(|b| {
                        let left = ((b.start - t0) / span) * 100.0;
                        let width = (((b.end - b.start) / span) * 100.0).max(0.6);
                        let color = status_color(&b.status);
                        let tip = format!(
                            "{} · {}{}",
                            b.status.to_lowercase(),
                            fmt_ms(b.end - b.start),
                            if b.running { " (running)" } else { "" }
                        );
                        view! {
                            <div
                                style:display="grid"
                                style:grid-template-columns="180px 1fr"
                                style:align-items="center"
                                style:gap="8px"
                            >
                                <span
                                    title=b.full_name.clone()
                                    style:font-size="11.5px"
                                    style:font-weight="500"
                                    style:color="var(--fg-2)"
                                    style:overflow="hidden"
                                    style:text-overflow="ellipsis"
                                    style:white-space="nowrap"
                                >
                                    {b.name.clone()}
                                </span>
                                <div
                                    style:position="relative"
                                    style:height="18px"
                                    style:background="var(--inset)"
                                    style:border-radius="3px"
                                >
                                    <div
                                        class:cl-pulse=b.running
                                        title=tip
                                        style:position="absolute"
                                        style:top="2px"
                                        style:bottom="2px"
                                        style:border-radius="3px"
                                        style:left=format!("{left}%")
                                        style:width=format!("{width}%")
                                        style:background=color
                                        style:opacity="0.85"
                                    ></div>
                                </div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
            <div
                style:margin-top="8px"
                style:font-family=MONO
                style:font-size="10.5px"
                style:color="var(--faint)"
            >
                {format!("wall {} · work {}", fmt_ms(wall), fmt_ms(work))}
            </div>
        </div>
    }
    .into_any()
}

/// Recent-runs heatmap (WorkflowDetail "Recent runs"): one bar per run,
/// height ∝ duration, color = status, hover = detail. Oldest → newest.
#[component]
pub fn RunHeatmap(runs: Vec<ExecutionSummary>) -> impl IntoView {
    let mut runs = runs;
    runs.reverse(); // list arrives newest-first; render oldest → newest
    let durations: Vec<f64> = runs
        .iter()
        .map(|r| {
            let start = parse_ms(&r.started_at).unwrap_or(0.0);
            let end = r
                .completed_at
                .as_deref()
                .and_then(parse_ms)
                .unwrap_or(start);
            (end - start).max(0.0)
        })
        .collect();
    let max = durations.iter().copied().fold(1.0_f64, f64::max);

    if runs.is_empty() {
        return view! {
            <span style:color="var(--muted)" style:font-size="13px">"No runs yet."</span>
        }
        .into_any();
    }

    view! {
        <div style:display="flex" style:align-items="flex-end" style:gap="3px" style:height="64px">
            {runs
                .into_iter()
                .zip(durations)
                .map(|(r, d)| {
                    let pct = ((d / max) * 100.0).max(8.0);
                    let tip = format!(
                        "{} · {} · {}",
                        r.status.to_lowercase(),
                        format_duration(Some(r.started_at.as_str()), r.completed_at.as_deref()),
                        crate::util::short_id(&r.id)
                    );
                    view! {
                        <div
                            title=tip
                            style:flex="1"
                            style:max-width="14px"
                            style:height=format!("{pct}%")
                            style:border-radius="2px"
                            style:background=status_color(&r.status)
                            style:opacity="0.8"
                        ></div>
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}

/// Wall-clock legend chip used beside charts.
#[component]
pub fn LegendDot(#[prop(into)] label: String, #[prop(into)] color: String) -> impl IntoView {
    let _ = token::MUTED;
    view! {
        <span
            style:display="inline-flex"
            style:align-items="center"
            style:gap="5px"
            style:font-family=MONO
            style:font-size="10.5px"
            style:color="var(--faint)"
        >
            <span
                style:width="8px"
                style:height="8px"
                style:border-radius="50%"
                style:background=color
            ></span>
            {label}
        </span>
    }
}
