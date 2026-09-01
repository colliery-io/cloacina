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

//! Workflow detail — operational view (CLOACI-T-0764), parity port of
//! `WorkflowDetail.tsx`: header with build badge + execute/pause/delete,
//! build-error alert, task graph (pack graph.rs), named instances
//! (CLOACI-T-0927, read-only). The reliability overlays and chart panels
//! (StatusStrip, RunHeatmap, TaskHealthTable, CombinedTimeline, ScheduleCard,
//! InputsCard, TaskCodeModal) are Wave-4 work (CLOACI-T-0935).

use aurora_leptos::components::{Alert, Empty, Loading, Modal, Panel};
use aurora_leptos::graph::{Graph, GraphEdge, GraphNode};
use aurora_leptos::tokens::token;
use aurora_leptos::widgets::BuildStatusBadge;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use cloacina_api_types::ExecutionTasksResponse;

use crate::auth::{client_for, use_auth};
use crate::components::{RunWorkflowModal, TagPill, ViewTabs};
use crate::data::poll_resource;
use crate::routes::execution_detail::ExecutionView;
use crate::util::ago;

const MONO: &str = "'IBM Plex Mono', monospace";

fn ts_ms(ts: &str) -> Option<f64> {
    let v = js_sys::Date::parse(ts);
    (!v.is_nan()).then_some(v)
}

/// Per-task aggregate over the analyzed runs (UAT round 3, T-0938): exit-type
/// counts, retry count, and mean start-offset/duration with sample stddev.
#[derive(Clone, PartialEq)]
struct TaskAgg {
    name: String,
    completed: u32,
    failed: u32,
    skipped: u32,
    other: u32,
    retried: u32,
    /// Mean start offset from execution start, seconds (completed samples).
    avg_start: f64,
    /// Mean duration in seconds (completed samples) and its sample stddev.
    avg_dur: f64,
    sd_dur: f64,
    samples: u32,
}

fn aggregate_tasks(resps: &[ExecutionTasksResponse]) -> Vec<TaskAgg> {
    #[derive(Default)]
    struct Acc {
        completed: u32,
        failed: u32,
        skipped: u32,
        other: u32,
        retried: u32,
        starts: Vec<f64>,
        durs: Vec<f64>,
    }
    let mut by_task = std::collections::BTreeMap::<String, Acc>::new();
    for r in resps {
        let exec_start = r
            .tasks
            .iter()
            .filter_map(|t| ts_ms(t.started_at.as_deref().unwrap_or(&t.created_at)))
            .fold(f64::INFINITY, f64::min);
        for t in &r.tasks {
            let a = by_task.entry(t.task_name.clone()).or_default();
            let status = t.status.to_lowercase();
            match status.as_str() {
                "completed" | "success" => a.completed += 1,
                "failed" | "error" => a.failed += 1,
                "skipped" => a.skipped += 1,
                _ => a.other += 1,
            }
            if t.attempt > 1 {
                a.retried += 1;
            }
            if matches!(status.as_str(), "completed" | "success") {
                let start = ts_ms(t.started_at.as_deref().unwrap_or(&t.created_at));
                let end = ts_ms(t.completed_at.as_deref().unwrap_or(&t.updated_at));
                if let (Some(s), Some(e)) = (start, end) {
                    if e >= s {
                        a.durs.push((e - s) / 1000.0);
                        if exec_start.is_finite() {
                            a.starts.push((s - exec_start) / 1000.0);
                        }
                    }
                }
            }
        }
    }
    let mean = |v: &[f64]| {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    by_task
        .into_iter()
        .map(|(name, a)| {
            let avg_dur = mean(&a.durs);
            let sd_dur = if a.durs.len() > 1 {
                (a.durs.iter().map(|d| (d - avg_dur).powi(2)).sum::<f64>()
                    / (a.durs.len() - 1) as f64)
                    .sqrt()
            } else {
                0.0
            };
            TaskAgg {
                name,
                completed: a.completed,
                failed: a.failed,
                skipped: a.skipped,
                other: a.other,
                retried: a.retried,
                avg_start: mean(&a.starts),
                avg_dur,
                sd_dur,
                samples: a.durs.len() as u32,
            }
        })
        .collect()
}

/// A count cell that stays visually quiet at zero.
#[component]
fn CountCell(n: u32, color: &'static str) -> impl IntoView {
    view! {
        <span
            class="cl-tnum"
            style:font-size="12px"
            style:color=if n == 0 { "var(--fainter)" } else { color }
        >
            {n}
        </span>
    }
}

#[component]
pub fn WorkflowDetail() -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());
    let params = use_params_map();
    let name = Signal::derive(move || params.read().get("name").unwrap_or_default());

    let detail = poll_resource(move |c| {
        let name = name.get();
        async move { c.get_workflow(&name, None).await }
    });
    let data = Signal::derive(move || detail.get().and_then(|r| r.ok()));
    let loading = Signal::derive(move || detail.get().is_none());
    let wf_name = Signal::derive(move || {
        data.get()
            .map(|d| {
                if d.workflow_name.is_empty() {
                    name.get()
                } else {
                    d.workflow_name
                }
            })
            .unwrap_or_else(|| name.get())
    });

    // Recent runs for the heatmap (last 40 of this workflow).
    let recent_runs_res = poll_resource(move |c| {
        let wf = wf_name.get();
        async move {
            c.list_executions(
                &cloacina_api_types::ListExecutionsQuery {
                    status: None,
                    workflow: Some(wf).filter(|w| !w.is_empty()),
                    limit: Some(40),
                    offset: Some(0),
                },
                None,
            )
            .await
        }
    });
    let recent_runs = Signal::derive(move || {
        recent_runs_res
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });

    let instances = poll_resource(move |c| {
        let wf = wf_name.get();
        async move {
            if wf.is_empty() {
                return Ok(None);
            }
            c.list_instances(&wf, Some(100), Some(0), None)
                .await
                .map(Some)
        }
    });
    let instance_items = Signal::derive(move || {
        instances
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|r| r.items)
            .unwrap_or_default()
    });

    // ---- Operational-history aggregates (UAT round 3, T-0938): task rows
    // for the last 20 runs, refetched only when the run-ID set changes.
    let history_ids = Memo::new(move |_| {
        recent_runs
            .get()
            .iter()
            .take(20)
            .map(|e| e.id.clone())
            .collect::<Vec<_>>()
    });
    let task_rows = LocalResource::new(move || {
        let ids = history_ids.get();
        let conn = auth.connection();
        async move {
            let Some(conn) = conn else {
                return Vec::new();
            };
            let Ok(client) = client_for(&conn) else {
                return Vec::new();
            };
            let tenant = conn.tenant.clone();
            futures_util::future::join_all(ids.iter().map(|id| {
                let client = client.clone();
                let tenant = tenant.clone();
                let id = id.clone();
                async move { client.get_execution_tasks(&tenant, &id).await.ok() }
            }))
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
        }
    });
    let task_aggs = Signal::derive(move || aggregate_tasks(&task_rows.get().unwrap_or_default()));
    let runs_analyzed = Signal::derive(move || task_rows.get().unwrap_or_default().len());

    // Dual views (UAT round 1, T-0938): current/most-recent execution is the
    // DEFAULT view (UAT round 2); operational history sits behind the second
    // tab, same orientation as GraphDetail. Prefer a live run.
    let view_mode = RwSignal::new("current");
    let current_exec_id = Signal::derive(move || {
        let runs = recent_runs.get();
        runs.iter()
            .find(|e| {
                matches!(
                    e.status.to_lowercase().as_str(),
                    "running" | "pending" | "queued"
                )
            })
            .or_else(|| runs.first())
            .map(|e| e.id.clone())
    });

    let exec_open = RwSignal::new(false);
    let exec_target = RwSignal::new(Option::<(String, String)>::None);
    let del_open = RwSignal::new(false);
    let del_error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let toggle_pause = move |_| {
        let Some(d) = data.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        let n = name.get_untracked();
        busy.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = if d.paused {
                    client.resume_workflow(&n, None).await
                } else {
                    client.pause_workflow(&n, None).await
                };
            }
            busy.set(false);
        });
    };

    let do_delete = move |_| {
        let Some(d) = data.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        let n = name.get_untracked();
        busy.set(true);
        del_error.set(String::new());
        leptos::task::spawn_local(async move {
            let result = async {
                let client = client_for(&conn)?;
                client
                    .delete_workflow(&n, &d.version, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            busy.set(false);
            match result {
                Ok(_) => navigate.with_value(|nav| nav("/workflows", Default::default())),
                Err(e) => del_error.set(e),
            }
        });
    };

    view! {
        <Show
            when=move || !loading.get()
            fallback=|| view! { <Loading label="Loading workflow…" /> }
        >
            <Show
                when=move || data.get().is_some()
                fallback=|| view! { <Empty message="Workflow not found." /> }
            >
                <div style:display="flex" style:flex-direction="column" style:gap="18px">
                    // Header
                    <div style:display="flex" style:justify-content="space-between" style:align-items="flex-start">
                        <div>
                            <a
                                href="/workflows"
                                style:font-family=MONO
                                style:font-size="11.5px"
                                style:color="var(--muted)"
                                style:text-decoration="none"
                            >
                                "← Workflows"
                            </a>
                            <div style:display="flex" style:gap="10px" style:align-items="center" style:margin-top="3px">
                                <h1
                                    style:font-size="23px"
                                    style:font-weight="600"
                                    style:color="var(--fg-bright)"
                                    style:letter-spacing="-.01em"
                                    style:margin="0"
                                >
                                    {move || name.get()}
                                </h1>
                                <Show when=move || data.get().map(|d| d.paused).unwrap_or(false)>
                                    <TagPill color=token::GOLD>"⏸ paused"</TagPill>
                                </Show>
                            </div>
                            <div style:display="flex" style:gap="8px" style:align-items="center" style:margin-top="5px">
                                {move || {
                                    data.get().map(|d| view! {
                                        <BuildStatusBadge status=d.build_status.clone() />
                                        <span style:font-family=MONO style:font-size="11.5px" style:color="var(--faint)">
                                            {format!(
                                                "v{} · created {} · workflow ",
                                                d.version,
                                                ago(Some(d.created_at.as_str()))
                                            )}
                                            <span style:color="var(--muted)">{d.workflow_name.clone()}</span>
                                        </span>
                                    })
                                }}
                            </div>
                        </div>
                        <div style:display="flex" style:gap="8px">
                            <Show when=move || auth.can_write()>
                                <button
                                    class="cl-btn cl-btn--filled"
                                    on:click=move |_| {
                                        exec_target.set(Some((name.get_untracked(), wf_name.get_untracked())));
                                        exec_open.set(true);
                                    }
                                >
                                    "▸ Execute"
                                </button>
                                <button
                                    class="cl-btn cl-btn--default"
                                    disabled=move || busy.get()
                                    on:click=toggle_pause
                                >
                                    {move || {
                                        if data.get().map(|d| d.paused).unwrap_or(false) {
                                            "▸ Resume"
                                        } else {
                                            "⏸ Pause"
                                        }
                                    }}
                                </button>
                            </Show>
                            <button
                                class="cl-btn cl-btn--subtle cl-btn--bad"
                                on:click=move |_| del_open.set(true)
                            >
                                "Delete"
                            </button>
                        </div>
                    </div>

                    // Build error
                    <Show when=move || data.get().and_then(|d| d.build_error).is_some()>
                        <Alert title="Build error" color="var(--bad)">
                            <span style:white-space="pre-wrap">
                                {move || data.get().and_then(|d| d.build_error).unwrap_or_default()}
                            </span>
                        </Alert>
                    </Show>

                    // View switcher (UAT round 1, T-0938)
                    <ViewTabs
                        tabs=vec![
                            ("current", "Current execution"),
                            ("history", "Operational history"),
                        ]
                        active=view_mode
                    />

                    // ---- Current-execution view ----
                    <Show when=move || view_mode.get() == "current">
                        <Show
                            when=move || current_exec_id.get().is_some()
                            fallback=|| view! { <Empty message="No executions of this workflow yet." /> }
                        >
                            {move || {
                                let id = Signal::derive(move || {
                                    current_exec_id.get().unwrap_or_default()
                                });
                                view! { <ExecutionView id=id embedded=true /> }
                            }}
                        </Show>
                    </Show>

                    // ---- Operational-history view ----
                    <Show when=move || view_mode.get() == "history">
                    // Run-level summary strip (UAT round 3)
                    {move || {
                        let runs = recent_runs.get();
                        let done: Vec<_> = runs
                            .iter()
                            .filter(|r| !r.status.eq_ignore_ascii_case("running"))
                            .collect();
                        let ok = done
                            .iter()
                            .filter(|r| r.status.eq_ignore_ascii_case("completed"))
                            .count();
                        let rate = if done.is_empty() {
                            "—".to_string()
                        } else {
                            format!("{:.0}%", 100.0 * ok as f64 / done.len() as f64)
                        };
                        let walls: Vec<f64> = runs
                            .iter()
                            .filter_map(|r| {
                                let s = ts_ms(&r.started_at)?;
                                let e = ts_ms(r.completed_at.as_deref()?)?;
                                (e >= s).then_some((e - s) / 1000.0)
                            })
                            .collect();
                        let avg_wall = if walls.is_empty() {
                            "—".to_string()
                        } else {
                            format!("{:.1}s", walls.iter().sum::<f64>() / walls.len() as f64)
                        };
                        let stat = |label: &str, value: String, color: &'static str| {
                            let label = label.to_string();
                            view! {
                                <div>
                                    <span
                                        style:font-family=MONO
                                        style:font-size="10px"
                                        style:letter-spacing=".07em"
                                        style:text-transform="uppercase"
                                        style:color="var(--muted)"
                                        style:display="block"
                                    >
                                        {label}
                                    </span>
                                    <span
                                        class="cl-tnum"
                                        style:font-size="19px"
                                        style:font-weight="600"
                                        style:color=color
                                    >
                                        {value}
                                    </span>
                                </div>
                            }
                        };
                        view! {
                            <div
                                style:display="flex"
                                style:gap="40px"
                                style:background="var(--panel)"
                                style:border="1px solid var(--border)"
                                style:border-radius="10px"
                                style:padding="12px 18px"
                            >
                                {stat("Runs analyzed", runs_analyzed.get().to_string(), "var(--fg)")}
                                {stat(
                                    "Success rate",
                                    rate.clone(),
                                    if rate.starts_with("100") { token::OK } else { token::GOLD },
                                )}
                                {stat("Avg wall-clock", avg_wall, token::ICE)}
                                {stat(
                                    "Failed runs",
                                    done.len().saturating_sub(ok).to_string(),
                                    if done.len() > ok { "var(--bad)" } else { "var(--fainter)" },
                                )}
                            </div>
                        }
                    }}

                    // Exit types per task (UAT round 3)
                    <Panel title="Task outcomes" caption="exit types over the analyzed runs">
                        {move || {
                            let aggs = task_aggs.get();
                            if aggs.is_empty() {
                                return view! { <Empty message="No run history to aggregate yet." /> }
                                    .into_any();
                            }
                            view! {
                                <table class="cl-table">
                                    <thead>
                                        <tr>
                                            <th>"Task"</th>
                                            <th>"Completed"</th>
                                            <th>"Failed"</th>
                                            <th>"Skipped"</th>
                                            <th>"Other"</th>
                                            <th>"Retried"</th>
                                            <th>"Avg duration"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {aggs
                                            .into_iter()
                                            .map(|a| {
                                                let dur = if a.samples == 0 {
                                                    "—".to_string()
                                                } else if a.sd_dur > 0.05 {
                                                    format!("{:.1}s ± {:.1}s", a.avg_dur, a.sd_dur)
                                                } else {
                                                    format!("{:.1}s", a.avg_dur)
                                                };
                                                view! {
                                                    <tr>
                                                        <td>
                                                            <span style:font-family=MONO style:font-size="12.5px" style:color="var(--fg)">
                                                                {a.name.clone()}
                                                            </span>
                                                        </td>
                                                        <td><CountCell n=a.completed color=token::OK /></td>
                                                        <td><CountCell n=a.failed color="var(--bad)" /></td>
                                                        <td><CountCell n=a.skipped color=token::VIOLET /></td>
                                                        <td><CountCell n=a.other color=token::GOLD /></td>
                                                        <td><CountCell n=a.retried color=token::GOLD /></td>
                                                        <td>
                                                            <span class="cl-tnum" style:font-size="12px" style:color="var(--fg-2)">
                                                                {dur}
                                                            </span>
                                                        </td>
                                                    </tr>
                                                }
                                            })
                                            .collect_view()}
                                    </tbody>
                                </table>
                            }
                            .into_any()
                        }}
                    </Panel>

                    // Average timing gantt with variance (UAT round 3)
                    <Panel
                        title="Average task timing"
                        caption="mean start → duration across the analyzed runs · gold band = ±1σ"
                    >
                        {move || {
                            let mut aggs = task_aggs.get();
                            aggs.retain(|a| a.samples > 0);
                            if aggs.is_empty() {
                                return view! { <Empty message="No completed runs to average yet." /> }
                                    .into_any();
                            }
                            aggs.sort_by(|x, y| {
                                x.avg_start
                                    .partial_cmp(&y.avg_start)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                            let total = aggs
                                .iter()
                                .map(|a| a.avg_start + a.avg_dur + a.sd_dur)
                                .fold(0.0f64, f64::max)
                                .max(0.001);
                            view! {
                                <div style:display="flex" style:flex-direction="column" style:gap="7px">
                                    {aggs
                                        .into_iter()
                                        .map(|a| {
                                            let left = 100.0 * a.avg_start / total;
                                            let width = (100.0 * a.avg_dur / total).max(0.8);
                                            let band_start =
                                                a.avg_start + (a.avg_dur - a.sd_dur).max(0.0);
                                            let band_left = 100.0 * band_start / total;
                                            let band_width = (100.0
                                                * ((a.avg_start + a.avg_dur + a.sd_dur) - band_start)
                                                / total)
                                                .max(0.0);
                                            let has_band = a.sd_dur > 0.02;
                                            let label =
                                                format!("{:.1}s ± {:.1}s", a.avg_dur, a.sd_dur);
                                            view! {
                                                <div style:display="flex" style:gap="12px" style:align-items="center">
                                                    <span
                                                        style:font-family=MONO
                                                        style:font-size="11.5px"
                                                        style:color="var(--fg-2)"
                                                        style:min-width="150px"
                                                        style:text-align="right"
                                                    >
                                                        {a.name.clone()}
                                                    </span>
                                                    <div
                                                        style:flex="1"
                                                        style:position="relative"
                                                        style:height="14px"
                                                        style:background="var(--inset)"
                                                        style:border-radius="3px"
                                                        style:overflow="hidden"
                                                    >
                                                        <Show when=move || has_band>
                                                            <div
                                                                style:position="absolute"
                                                                style:left=format!("{band_left:.2}%")
                                                                style:width=format!("{band_width:.2}%")
                                                                style:top="0"
                                                                style:bottom="0"
                                                                style:background=token::GOLD
                                                                style:opacity="0.3"
                                                            ></div>
                                                        </Show>
                                                        <div
                                                            style:position="absolute"
                                                            style:left=format!("{left:.2}%")
                                                            style:width=format!("{width:.2}%")
                                                            style:top="2px"
                                                            style:bottom="2px"
                                                            style:border-radius="2px"
                                                            style:background=token::ICE
                                                        ></div>
                                                    </div>
                                                    <span
                                                        class="cl-tnum"
                                                        style:font-size="11px"
                                                        style:color="var(--faint)"
                                                        style:min-width="96px"
                                                    >
                                                        {label}
                                                    </span>
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                            .into_any()
                        }}
                    </Panel>

                    // Task graph
                    <Panel title="Task graph">
                        {move || {
                            let d = data.get();
                            let graph = d.as_ref().map(|d| d.task_graph.clone()).unwrap_or_default();
                            if !graph.is_empty() {
                                let nodes = graph
                                    .iter()
                                    .map(|n| GraphNode::new(n.id.clone(), n.id.clone()).color(token::ICE))
                                    .collect::<Vec<_>>();
                                let edges = graph
                                    .iter()
                                    .flat_map(|n| {
                                        n.dependencies.iter().map(move |dep| GraphEdge {
                                            from: dep.clone(),
                                            to: n.id.clone(),
                                            active: false,
                                        })
                                    })
                                    .collect::<Vec<_>>();
                                view! { <Graph nodes=nodes edges=edges direction="LR" /> }.into_any()
                            } else {
                                let tasks = d.map(|d| d.tasks).unwrap_or_default();
                                if tasks.is_empty() {
                                    view! { <span style:color="var(--muted)" style:font-size="13px">"No tasks."</span> }
                                        .into_any()
                                } else {
                                    view! {
                                        <ul class="cl-list">
                                            {tasks
                                                .into_iter()
                                                .map(|t| view! { <li>{t}</li> })
                                                .collect_view()}
                                        </ul>
                                    }
                                    .into_any()
                                }
                            }
                        }}
                    </Panel>

                    // Recent runs (RunHeatmap, T-0935)
                    <Panel title="Recent runs" caption="last 40 · bar height = duration · hover for detail">
                        {move || view! { <crate::charts::RunHeatmap runs=recent_runs.get() /> }}
                    </Panel>

                    // Named instances (T-0927, read-only)
                    <Panel title="Named instances" caption="persistent param bindings, optionally scheduled">
                        <Show
                            when=move || !instance_items.get().is_empty()
                            fallback=|| view! {
                                <Empty message="No named instances. Create one with `cloacinactl instance create`." />
                            }
                        >
                            <div style:display="flex" style:flex-direction="column" style:gap="6px">
                                <For
                                    each=move || instance_items.get()
                                    key=|i| i.id.clone()
                                    children=|i| {
                                        let cron_pill = i.cron_expression.clone();
                                        view! {
                                            <div style:display="flex" style:gap="12px" style:align-items="baseline">
                                                <span
                                                    style:font-family=MONO
                                                    style:font-size="13px"
                                                    style:color="var(--fg)"
                                                    style:min-width="160px"
                                                >
                                                    {i.instance_name.clone()}
                                                </span>
                                                <TagPill color=if cron_pill.is_some() { token::TEAL } else { token::MUTED }>
                                                    {cron_pill.unwrap_or_else(|| "unscheduled".into())}
                                                </TagPill>
                                                <Show when=move || i.paused>
                                                    <TagPill color=token::GOLD>"⏸ paused"</TagPill>
                                                </Show>
                                                <span
                                                    style:font-family=MONO
                                                    style:font-size="11px"
                                                    style:color="var(--muted)"
                                                    style:flex="1"
                                                    style:overflow="hidden"
                                                    style:text-overflow="ellipsis"
                                                    style:white-space="nowrap"
                                                >
                                                    {i.params
                                                        .as_ref()
                                                        .map(|p| p.to_string())
                                                        .unwrap_or_else(|| "—".into())}
                                                </span>
                                                <span style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                                                    {i.next_run_at
                                                        .as_ref()
                                                        .map(|t| format!("next {t}"))
                                                        .unwrap_or_default()}
                                                </span>
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        </Show>
                    </Panel>
                    </Show>

                    // Modals
                    <RunWorkflowModal open=exec_open target=exec_target />
                    <Show when=move || del_open.get()>
                        <Modal open=del_open title="Delete workflow?">
                            <div style:display="flex" style:flex-direction="column" style:gap="14px">
                                <span style:font-size="13px" style:color="var(--fg-2)">
                                    {move || format!(
                                        "Unregister {} v{}? This removes the package from the tenant.",
                                        name.get(),
                                        data.get().map(|d| d.version).unwrap_or_default()
                                    )}
                                </span>
                                <Show when=move || !del_error.get().is_empty()>
                                    <span style:color="var(--bad)" style:font-size="12.5px">
                                        {move || del_error.get()}
                                    </span>
                                </Show>
                                <div style:display="flex" style:justify-content="flex-end" style:gap="10px">
                                    <button class="cl-btn cl-btn--default" on:click=move |_| del_open.set(false)>
                                        "Cancel"
                                    </button>
                                    <button
                                        class="cl-btn cl-btn--filled cl-btn--bad"
                                        disabled=move || busy.get()
                                        on:click=do_delete
                                    >
                                        "Delete"
                                    </button>
                                </div>
                            </div>
                        </Modal>
                    </Show>
                </div>
            </Show>
        </Show>
    }
}
