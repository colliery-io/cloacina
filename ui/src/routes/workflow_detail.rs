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

use aurora_leptos::components::{
    Alert, Button, ConfirmDialog, Empty, List, ListItem, Loading, PageHeader, Panel, Pill,
    RelativeTime, StatTile, TabItem, TabPanel, Table, Tabs,
};
use aurora_leptos::graph::{Graph, GraphEdge, GraphNode};
use aurora_leptos::tokens::token;
use aurora_leptos::widgets::BuildStatusBadge;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use cloacina_api_types::ExecutionTasksResponse;

use crate::auth::{client_for, use_auth};
use crate::components::RunWorkflowModal;
use crate::data::poll_resource;
use crate::routes::execution_detail::ExecutionView;

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
    // Aggregate under the short task name (last `::` segment) — the
    // fully-qualified form is noise at this level.
    let short = |n: &str| n.rsplit("::").next().unwrap_or(n).to_string();
    let mut by_task = std::collections::BTreeMap::<String, Acc>::new();
    for r in resps {
        let exec_start = r
            .tasks
            .iter()
            .filter_map(|t| ts_ms(t.started_at.as_deref().unwrap_or(&t.created_at)))
            .fold(f64::INFINITY, f64::min);
        for t in &r.tasks {
            let a = by_task.entry(short(&t.task_name)).or_default();
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
        // The hue is the outcome (data); zero stays quiet.
        <span class="app-count cl-tnum" style:color=if n == 0 { "var(--fainter)" } else { color }>
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
    let view_mode = RwSignal::new("current".to_string());
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

    let toggle_pause = move || {
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

    let do_delete = move || {
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

    let paused = Signal::derive(move || data.get().map(|d| d.paused).unwrap_or(false));

    let header = move || {
        view! {
            <PageHeader
                title=name.get()
                back_href="/workflows"
                back_label="Workflows"
                meta=Box::new(move || view! {
                    <Show when=move || paused.get()>
                        <Pill color=token::GOLD>"⏸ paused"</Pill>
                    </Show>
                    {move || data.get().map(|d| view! {
                        <BuildStatusBadge status=d.build_status.clone() />
                        <span class="app-meta app-meta--md app-faint">
                            {format!("v{} · created ", d.version)}
                            <RelativeTime iso=d.created_at.clone() />
                            " · workflow "
                            <span class="app-muted">{d.workflow_name.clone()}</span>
                        </span>
                    })}
                }.into_any())
                actions=Box::new(move || view! {
                    <Show when=move || auth.can_write()>
                        <Button on_click=Callback::new(move |_| {
                            exec_target.set(Some((name.get_untracked(), wf_name.get_untracked())));
                            exec_open.set(true);
                        })>
                            "▸ Execute"
                        </Button>
                        <Button variant="default" loading=busy on_click=Callback::new(move |_| toggle_pause())>
                            {move || if paused.get() { "▸ Resume" } else { "⏸ Pause" }}
                        </Button>
                    </Show>
                    <Button variant="subtle" bad=true on_click=Callback::new(move |_| del_open.set(true))>
                        "Delete"
                    </Button>
                }.into_any())
            />
        }
    };

    // Run-level summary strip (UAT round 3).
    let summary = move || {
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
        let failed = done.len().saturating_sub(ok);
        let rate_color = if rate.starts_with("100") {
            token::OK
        } else {
            token::GOLD
        };
        let failed_color = if failed > 0 {
            token::BAD
        } else {
            "var(--fainter)"
        };
        view! {
            <div class="app-grid-4">
                <StatTile label="Runs analyzed" value=runs_analyzed.get().to_string() />
                <StatTile label="Success rate" value=rate color=rate_color />
                <StatTile label="Avg wall-clock" value=avg_wall color=token::ICE />
                <StatTile label="Failed runs" value=failed.to_string() color=failed_color />
            </div>
        }
    };

    // Exit types per task (UAT round 3).
    let outcomes = move || {
        let aggs = task_aggs.get();
        if aggs.is_empty() {
            return view! { <Empty message="No run history to aggregate yet." /> }.into_any();
        }
        view! {
            <Table label="Task outcomes">
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
                                    <td class="app-mono app-small app-fg">{a.name.clone()}</td>
                                    <td><CountCell n=a.completed color=token::OK /></td>
                                    <td><CountCell n=a.failed color=token::BAD /></td>
                                    <td><CountCell n=a.skipped color=token::VIOLET /></td>
                                    <td><CountCell n=a.other color=token::GOLD /></td>
                                    <td><CountCell n=a.retried color=token::GOLD /></td>
                                    <td class="app-count cl-tnum app-fg2">{dur}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </Table>
        }
        .into_any()
    };

    // Average timing gantt with variance (UAT round 3).
    let timing = move || {
        let mut aggs = task_aggs.get();
        aggs.retain(|a| a.samples > 0);
        if aggs.is_empty() {
            return view! { <Empty message="No completed runs to average yet." /> }.into_any();
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
            <div class="app-avg">
                {aggs
                    .into_iter()
                    .map(|a| {
                        let left = 100.0 * a.avg_start / total;
                        let width = (100.0 * a.avg_dur / total).max(0.8);
                        let band_start = a.avg_start + (a.avg_dur - a.sd_dur).max(0.0);
                        let band_left = 100.0 * band_start / total;
                        let band_width =
                            (100.0 * ((a.avg_start + a.avg_dur + a.sd_dur) - band_start) / total).max(0.0);
                        let has_band = a.sd_dur > 0.02;
                        let label = format!("{:.1}s ± {:.1}s", a.avg_dur, a.sd_dur);
                        view! {
                            <div class="app-avg__row">
                                <span class="app-avg__name app-ellipsis" title=a.name.clone()>{a.name.clone()}</span>
                                <div class="app-avg__track">
                                    // Offsets and widths are computed from the task times.
                                    <Show when=move || has_band>
                                        <div
                                            class="app-avg__band"
                                            style:left=format!("{band_left:.2}%")
                                            style:width=format!("{band_width:.2}%")
                                        ></div>
                                    </Show>
                                    <div
                                        class="app-avg__bar"
                                        style:left=format!("{left:.2}%")
                                        style:width=format!("{width:.2}%")
                                    ></div>
                                </div>
                                <span class="app-avg__label cl-tnum">{label}</span>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        }
        .into_any()
    };

    let task_graph = move || {
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
            view! {
                <div data-testid="workflow-graph">
                    <Graph nodes=nodes edges=edges direction="LR" />
                </div>
            }
            .into_any()
        } else {
            let tasks = d.map(|d| d.tasks).unwrap_or_default();
            if tasks.is_empty() {
                view! { <span class="app-hint">"No tasks."</span> }.into_any()
            } else {
                view! {
                    <List>
                        {tasks.into_iter().map(|t| view! { <ListItem>{t}</ListItem> }).collect_view()}
                    </List>
                }
                .into_any()
            }
        }
    };

    let instances_view = move || {
        view! {
            <Show
                when=move || !instance_items.get().is_empty()
                fallback=|| view! {
                    <Empty message="No named instances. Create one with `cloacinactl instance create`." />
                }
            >
                <Table label="Named instances">
                    <thead>
                        <tr>
                            <th>"Instance"</th>
                            <th>"Schedule"</th>
                            <th>"Params"</th>
                            <th>"Next run"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || instance_items.get()
                            key=|i| i.id.clone()
                            children=|i| {
                                let cron_pill = i.cron_expression.clone();
                                let params = i
                                    .params
                                    .as_ref()
                                    .map(|p| p.to_string())
                                    .unwrap_or_else(|| "—".into());
                                view! {
                                    <tr>
                                        <td class="app-mono app-small app-fg">{i.instance_name.clone()}</td>
                                        <td>
                                            <span class="app-row app-row--tight">
                                                <Pill color=if cron_pill.is_some() { token::TEAL } else { token::MUTED }>
                                                    {cron_pill.unwrap_or_else(|| "unscheduled".into())}
                                                </Pill>
                                                <Show when=move || i.paused>
                                                    <Pill color=token::GOLD>"⏸ paused"</Pill>
                                                </Show>
                                            </span>
                                        </td>
                                        <td>
                                            <span class="app-meta app-meta--sm app-muted app-ellipsis app-clip" title=params.clone()>
                                                {params.clone()}
                                            </span>
                                        </td>
                                        <td class="app-meta">
                                            {i.next_run_at.as_ref().map(|t| format!("next {t}")).unwrap_or_default()}
                                        </td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </Table>
            </Show>
        }
    };

    let history = move || {
        view! {
            <div class="app-col app-col--loose">
                {summary}
                <Panel title="Task outcomes" caption="exit types over the analyzed runs">{outcomes}</Panel>
                <Panel
                    title="Average task timing"
                    caption="mean start → duration across the analyzed runs · gold band = ±1σ"
                >
                    {timing}
                </Panel>
                <Panel title="Task graph">{task_graph}</Panel>
                // Recent runs (RunHeatmap, T-0935)
                <Panel title="Recent runs" caption="last 40 · bar height = duration · hover for detail">
                    {move || view! { <crate::charts::RunHeatmap runs=recent_runs.get() /> }}
                </Panel>
                // Named instances (T-0927, read-only)
                <Panel title="Named instances" caption="persistent param bindings, optionally scheduled">
                    {instances_view}
                </Panel>
            </div>
        }
    };

    let current = move || {
        view! {
            <Show
                when=move || current_exec_id.get().is_some()
                fallback=|| view! { <Empty message="No executions of this workflow yet." /> }
            >
                {move || {
                    let id = Signal::derive(move || current_exec_id.get().unwrap_or_default());
                    view! { <ExecutionView id=id embedded=true /> }
                }}
            </Show>
        }
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
                <div class="app-page app-page--loose">
                    {header}

                    // Build error
                    <Show when=move || data.get().and_then(|d| d.build_error).is_some()>
                        <Alert title="Build error" color=token::BAD>
                            <span class="app-prewrap">
                                {move || data.get().and_then(|d| d.build_error).unwrap_or_default()}
                            </span>
                        </Alert>
                    </Show>

                    // Dual views (UAT round 1, T-0938): the current execution
                    // is the default; operational history is the second tab.
                    <Tabs
                        tabs=vec![
                            TabItem::new("current", "Current execution"),
                            TabItem::new("history", "Operational history"),
                        ]
                        value=view_mode
                        label="Workflow views"
                    >
                        <TabPanel value="current">{current}</TabPanel>
                        <TabPanel value="history">{history}</TabPanel>
                    </Tabs>

                    // Modals
                    <RunWorkflowModal open=exec_open target=exec_target />
                    <ConfirmDialog
                        open=del_open
                        title="Delete workflow?"
                        confirm_label="Delete"
                        busy=busy
                        on_confirm=Callback::new(move |_| do_delete())
                        on_cancel=Callback::new(move |_| del_error.set(String::new()))
                    >
                        <div class="app-col">
                            <span class="app-text app-fg2">
                                {move || format!(
                                    "Unregister {} v{}? This removes the package from the tenant.",
                                    name.get(),
                                    data.get().map(|d| d.version).unwrap_or_default()
                                )}
                            </span>
                            <Show when=move || !del_error.get().is_empty()>
                                <span class="app-error" role="alert">{move || del_error.get()}</span>
                            </Show>
                        </div>
                    </ConfirmDialog>
                </div>
            </Show>
        </Show>
    }
}
