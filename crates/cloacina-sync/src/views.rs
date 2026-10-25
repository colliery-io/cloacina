/*
 *  Copyright 2025-2026 Colliery Software
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

//! The Sync read model (COLLIERY-I-0635 / CLOACINA-T-1004): syncs, their
//! passes, dead-letters and streams, built from the DAL for the
//! `/tenants/{tenant_id}/syncs` routes. A sync is a workflow instance of the
//! Sync workflow; these views never make a caller name the workflow.

use cloacina::dal::DAL;
use cloacina::dal::unified::workflow_execution::ExecutionListFilter;
use cloacina::models::schedule::Schedule;
use cloacina::models::workflow_execution::WorkflowExecutionRecord;
use cloacina_api_types::{
    DiscoveredField, ExecutionSummary, SyncDeadLetter, SyncDetail, SyncPass, SyncSide, SyncStream,
    SyncSummary,
};
use cloacina_constructor_contract::sync::{ConfiguredStream, StreamSchema, StreamState};
use serde_json::{Map, Value};

use crate::WORKFLOW_NAME;
use crate::params::keys;

/// A read failed.
#[derive(Debug, thiserror::Error)]
pub enum ViewError {
    #[error("no sync named `{0}`")]
    NotFound(String),
    #[error("database: {0}")]
    Database(String),
}

fn db(e: impl std::fmt::Display) -> ViewError {
    ViewError::Database(e.to_string())
}

fn params_of(s: &Schedule) -> Map<String, Value> {
    s.params
        .as_deref()
        .and_then(|p| serde_json::from_str(p).ok())
        .unwrap_or_default()
}

fn text(params: &Map<String, Value>, key: &str) -> Option<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// The stream a sync's params configure.
fn stream_of(params: &Map<String, Value>) -> Option<ConfiguredStream> {
    match params.get(keys::STREAM)? {
        Value::String(s) => serde_json::from_str(s).ok(),
        other => serde_json::from_value(other.clone()).ok(),
    }
}

/// The name a sync's cursor and dead-letters are kept under.
fn sync_name(instance: &str, params: &Map<String, Value>) -> String {
    text(params, keys::SYNC).unwrap_or_else(|| instance.to_string())
}

fn summary_of(e: &WorkflowExecutionRecord) -> ExecutionSummary {
    ExecutionSummary {
        id: e.id.0.to_string(),
        workflow_name: e.workflow_name.clone(),
        status: e.status.clone(),
        started_at: e.started_at.0.to_rfc3339(),
        completed_at: e.completed_at.map(|t| t.0.to_rfc3339()),
        trigger_origin: e.trigger_origin.clone(),
        instance_name: e.instance_name.clone(),
        pass_id: e.pass_id.map(|p| p.0.to_string()),
    }
}

/// The sync definitions: every named instance of the Sync workflow.
async fn definitions(dal: &DAL) -> Result<Vec<Schedule>, ViewError> {
    Ok(dal
        .schedule()
        .find_by_workflow(WORKFLOW_NAME)
        .await
        .map_err(db)?
        .into_iter()
        .filter(|s| s.instance_name.is_some())
        .collect())
}

async fn definition(dal: &DAL, name: &str) -> Result<Schedule, ViewError> {
    dal.schedule()
        .find_by_instance_name(WORKFLOW_NAME, name)
        .await
        .map_err(db)?
        .ok_or_else(|| ViewError::NotFound(name.to_string()))
}

/// What a pass's last segment left in its saved context: the cursor, and
/// whether the source reported its end.
async fn last_segment(dal: &DAL, last: &WorkflowExecutionRecord) -> (Option<String>, bool) {
    let Some(id) = last.context_id else {
        return (None, false);
    };
    let Ok(ctx) = dal.context().read::<Value>(id).await else {
        return (None, false);
    };
    let cursor = crate::params::handoff::<StreamState>(&ctx, keys::NEXT_STATE)
        .ok()
        .flatten()
        .and_then(|s| s.cursor);
    let exhausted = matches!(ctx.get(keys::EXHAUSTED), Some(Value::Bool(true)));
    (cursor, exhausted)
}

async fn pass(
    dal: &DAL,
    pass_id: cloacina::database::universal_types::UniversalUuid,
) -> Result<SyncPass, ViewError> {
    let mut runs = dal
        .workflow_execution()
        .list_filtered(ExecutionListFilter {
            pass_id: Some(pass_id),
            limit: 10_000,
            ..Default::default()
        })
        .await
        .map_err(db)?;
    runs.reverse();
    let (cursor, exhausted) = match runs.last() {
        Some(last) if last.status == "Completed" => last_segment(dal, last).await,
        _ => (None, false),
    };
    // A pass is done when a segment failed, or when its last segment saw the
    // source end; between segments it is still running.
    let status = if runs
        .iter()
        .any(|r| r.status == "Failed" || r.status == "Cancelled")
    {
        "Failed"
    } else if exhausted && runs.iter().all(|r| r.status == "Completed") {
        "Completed"
    } else {
        "Running"
    };
    let completed_at = (status != "Running")
        .then(|| {
            runs.last()
                .and_then(|r| r.completed_at.map(|t| t.0.to_rfc3339()))
        })
        .flatten();
    Ok(SyncPass {
        pass_id: pass_id.0.to_string(),
        status: status.to_string(),
        started_at: runs
            .first()
            .map(|r| r.started_at.0.to_rfc3339())
            .unwrap_or_default(),
        completed_at,
        segments: runs.len(),
        cursor,
        executions: runs.iter().map(summary_of).collect(),
    })
}

async fn summary(dal: &DAL, s: &Schedule) -> Result<SyncSummary, ViewError> {
    let name = s.instance_name.clone().unwrap_or_default();
    let params = params_of(s);
    let sync = sync_name(&name, &params);
    let stream = stream_of(&params);
    let last_pass = match dal
        .workflow_execution()
        .list_passes(&name, 1, 0)
        .await
        .map_err(db)?
        .first()
    {
        Some(id) => Some(pass(dal, *id).await?),
        None => None,
    };
    let schema_broken = dal
        .sync_stream_schemas()
        .list_for_sync(&sync)
        .await
        .map_err(db)?
        .iter()
        .any(|r| r.broken.is_some());
    let dead_letters = dal
        .sync_dead_letters()
        .count_for_sync(&sync)
        .await
        .map_err(db)?;
    Ok(SyncSummary {
        name,
        sync_name: sync,
        source: SyncSide {
            provider: text(&params, keys::SOURCE_PROVIDER),
            definition: text(&params, keys::SOURCE_DEFINITION),
        },
        destination: SyncSide {
            provider: text(&params, keys::DESTINATION_PROVIDER),
            definition: text(&params, keys::DESTINATION_DEFINITION),
        },
        stream: stream
            .as_ref()
            .map(|s| s.stream.clone())
            .unwrap_or_default(),
        sync_mode: stream
            .as_ref()
            .map(|s| format!("{:?}", s.sync_mode))
            .unwrap_or_default(),
        write_mode: stream
            .as_ref()
            .map(|s| match &s.write_mode {
                cloacina_constructor_contract::sync::WriteMode::Append => "Append".to_string(),
                cloacina_constructor_contract::sync::WriteMode::Upsert { .. } => {
                    "Upsert".to_string()
                }
                cloacina_constructor_contract::sync::WriteMode::Overwrite => {
                    "Overwrite".to_string()
                }
            })
            .unwrap_or_default(),
        cron: s.cron_expression.clone(),
        timezone: s.timezone.clone(),
        enabled: s.enabled.is_true(),
        next_run_at: s.next_run_at.map(|t| t.0.to_rfc3339()),
        last_pass,
        schema_broken,
        dead_letters,
    })
}

/// Every sync, by name.
pub async fn list(dal: &DAL) -> Result<Vec<SyncSummary>, ViewError> {
    let mut out = Vec::new();
    for s in definitions(dal).await? {
        out.push(summary(dal, &s).await?);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// One sync, with its stored params.
pub async fn get(dal: &DAL, name: &str) -> Result<SyncDetail, ViewError> {
    let s = definition(dal, name).await?;
    Ok(SyncDetail {
        summary: summary(dal, &s).await?,
        params: Some(Value::Object(params_of(&s))),
    })
}

/// A page of a sync's passes, newest first.
pub async fn passes(
    dal: &DAL,
    name: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<SyncPass>, ViewError> {
    definition(dal, name).await?;
    let mut out = Vec::new();
    for id in dal
        .workflow_execution()
        .list_passes(name, limit, offset)
        .await
        .map_err(db)?
    {
        out.push(pass(dal, id).await?);
    }
    Ok(out)
}

/// A page of a sync's dead-letters, newest first, and how many it holds.
pub async fn dead_letters(
    dal: &DAL,
    name: &str,
    limit: i64,
    offset: i64,
) -> Result<(Vec<SyncDeadLetter>, i64), ViewError> {
    let s = definition(dal, name).await?;
    let sync = sync_name(name, &params_of(&s));
    let rows = dal
        .sync_dead_letters()
        .page(&sync, limit, offset)
        .await
        .map_err(db)?;
    let total = dal
        .sync_dead_letters()
        .count_for_sync(&sync)
        .await
        .map_err(db)?;
    let items = rows
        .into_iter()
        .map(|d| SyncDeadLetter {
            id: d.id.0.to_string(),
            stream: d.stream,
            record: d.record,
            reason: d.reason,
            created_at: d.created_at.0.to_rfc3339(),
            execution_id: d.workflow_execution_id.map(|e| e.0.to_string()),
        })
        .collect();
    Ok((items, total))
}

fn fields_of(schema: &StreamSchema) -> Vec<DiscoveredField> {
    schema
        .fields
        .iter()
        .map(|f| DiscoveredField {
            name: f.name.clone(),
            field_type: format!("{:?}", f.field_type),
            nullable: f.nullable,
        })
        .collect()
}

/// A sync's streams: the configured one, with its cursor and schema.
pub async fn streams(dal: &DAL, name: &str) -> Result<Vec<SyncStream>, ViewError> {
    let s = definition(dal, name).await?;
    let params = params_of(&s);
    let sync = sync_name(name, &params);
    let states = dal
        .sync_stream_state()
        .list_for_sync(&sync)
        .await
        .map_err(db)?;
    let schemas = dal
        .sync_stream_schemas()
        .list_for_sync(&sync)
        .await
        .map_err(db)?;
    let mut names: Vec<String> = states.iter().map(|s| s.stream.clone()).collect();
    names.extend(schemas.iter().map(|s| s.stream.clone()));
    if let Some(stream) = stream_of(&params) {
        names.push(stream.stream);
    }
    names.sort();
    names.dedup();
    Ok(names
        .into_iter()
        .map(|stream| {
            let state = states.iter().find(|s| s.stream == stream);
            let schema = schemas.iter().find(|s| s.stream == stream);
            SyncStream {
                partition: state.map(|s| s.partition_id.clone()).unwrap_or_default(),
                cursor: state.and_then(|s| s.cursor.clone()),
                exhausted: state.map(|s| s.exhausted.0).unwrap_or(false),
                updated_at: state.map(|s| s.updated_at.0.to_rfc3339()),
                fields: schema
                    .and_then(|s| serde_json::from_str::<StreamSchema>(&s.schema).ok())
                    .map(|s| fields_of(&s))
                    .unwrap_or_default(),
                broken: schema.and_then(|s| s.broken.clone()),
                stream,
            }
        })
        .collect())
}

/// Accept the new shape of a sync's stream: the next write captures it.
/// Returns the stream.
pub async fn accept_schema(dal: &DAL, name: &str) -> Result<String, ViewError> {
    let s = definition(dal, name).await?;
    let params = params_of(&s);
    let sync = sync_name(name, &params);
    let stream = stream_of(&params)
        .map(|s| s.stream)
        .ok_or_else(|| ViewError::Database(format!("the sync `{name}` has no stream")))?;
    dal.sync_stream_schemas()
        .accept(&sync, &stream)
        .await
        .map_err(db)?;
    Ok(stream)
}

/// The context a fire of the sync `name` starts with, as its cron fire does:
/// its stored params, and the instance it belongs to (CLOACINA-T-1005).
pub async fn fire_context(dal: &DAL, name: &str) -> Result<cloacina::Context<Value>, ViewError> {
    let s = definition(dal, name).await?;
    let mut ctx = cloacina::Context::new();
    if let Some(params) = s.params.as_deref() {
        cloacina::workflow_instance::merge_instance_params(&mut ctx, params)
            .map_err(ViewError::Database)?;
    }
    cloacina::workflow_instance::stamp_instance(&mut ctx, Some(name));
    Ok(ctx)
}
