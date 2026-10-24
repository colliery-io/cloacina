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
//! The World of a Sync scenario: the two sides of one sync, its stream, the
//! scenario's runner, and what the last pass did.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use cloacina::dal::DAL;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina_constructor_contract::sync::{ConfiguredStream, MappingSpec, SyncMode, WriteMode};
use cloacina_sync::dispatcher::{Dispatcher, PassOutcome, SyncDefinition};
use cloacina_sync::params::keys;
use serde_json::{Value, json};

use super::{pg, providers};

/// The one sync of a scenario.
pub const SYNC: &str = "bdd";

/// One side of the sync: a provider package, its config, its network grants.
#[derive(Debug, Clone)]
pub struct Side {
    pub provider: String,
    pub config: Value,
    pub grants: Option<Value>,
}

impl Side {
    /// The Postgres connector on a table of the scenario's Postgres.
    pub fn postgres(table: &str) -> Self {
        Self {
            provider: providers::POSTGRES.into(),
            config: json!({ "url": pg::connector_url(), "table": table }),
            grants: Some(json!({ "tcp": [format!("*:{}", pg::port())] })),
        }
    }
}

#[derive(cucumber::World)]
#[world(init = Self::new)]
pub struct SyncWorld {
    /// Unique per scenario: it prefixes the scenario's tables and names the
    /// runner's schema on Postgres.
    pub scope: String,
    runner: Option<DefaultRunner>,
    pub source: Option<Side>,
    pub destination: Option<Side>,
    pub stream: Option<ConfiguredStream>,
    pub outcome: Option<Result<PassOutcome, String>>,
    /// Tables to drop when the scenario ends.
    tables: Vec<String>,
}

impl fmt::Debug for SyncWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyncWorld")
            .field("scope", &self.scope)
            .field("source", &self.source)
            .field("destination", &self.destination)
            .field("stream", &self.stream)
            .field("outcome", &self.outcome)
            .finish_non_exhaustive()
    }
}

/// The database of a scenario's runner, from `CLOACINA_BDD_RUNNER`.
fn runner_on_postgres() -> bool {
    match std::env::var("CLOACINA_BDD_RUNNER").as_deref() {
        Ok("postgres") => true,
        Ok("sqlite") | Err(_) => false,
        Ok(other) => panic!("CLOACINA_BDD_RUNNER is `{other}`: use `sqlite` or `postgres`"),
    }
}

impl SyncWorld {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            scope: format!(
                "bdd_{}_{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ),
            runner: None,
            source: None,
            destination: None,
            stream: None,
            outcome: None,
            tables: Vec::new(),
        }
    }

    /// The physical name of the scenario's table `name`.
    pub fn table(&mut self, name: &str) -> String {
        let table = format!("{}_{name}", self.scope);
        if !self.tables.contains(&table) {
            self.tables.push(table.clone());
        }
        table
    }

    /// An incremental stream of the source table, upserted on `key`.
    pub fn copy_stream(&mut self, table: &str, cursor: &str, key: &str) {
        let table = self.table(table);
        self.stream = Some(ConfiguredStream {
            stream: table,
            sync_mode: SyncMode::Incremental,
            cursor_field: Some(cursor.into()),
            primary_key: Some(vec![key.into()]),
            write_mode: WriteMode::Upsert {
                business_keys: vec![key.into()],
            },
            mapping: MappingSpec::default(),
        });
    }

    /// The scenario's runner, made on first use.
    pub async fn runner(&mut self) -> DefaultRunner {
        if let Some(runner) = &self.runner {
            return runner.clone();
        }
        cloacina_sync::ensure_linked();
        providers::staged();
        let config = DefaultRunnerConfig::builder()
            .enable_registry_reconciler(false)
            .build()
            .expect("runner config");
        let runner = if runner_on_postgres() {
            DefaultRunner::builder()
                .database_url(&pg::url())
                .schema(&self.scope)
                .with_config(config)
                .build()
                .await
        } else {
            DefaultRunner::with_config(":memory:", config).await
        }
        .expect("the scenario's runner");
        self.runner = Some(runner.clone());
        runner
    }

    pub async fn dal(&mut self) -> DAL {
        DAL::new(self.runner().await.database().clone())
    }

    fn definition(&self) -> SyncDefinition {
        let source = self.source.as_ref().expect("the scenario names a source");
        let destination = self
            .destination
            .as_ref()
            .expect("the scenario names a destination");
        let stream = self.stream.as_ref().expect("the scenario names a stream");
        let mut p: HashMap<String, Value> = HashMap::new();
        p.insert(keys::SYNC.into(), json!(SYNC));
        p.insert(keys::SOURCE_PROVIDER.into(), json!(source.provider));
        p.insert(keys::SOURCE_MEMBER.into(), json!("source"));
        p.insert(keys::SOURCE_CONFIG.into(), json!(source.config.to_string()));
        if let Some(g) = &source.grants {
            p.insert(keys::SOURCE_GRANTS.into(), g.clone());
        }
        p.insert(
            keys::DESTINATION_PROVIDER.into(),
            json!(destination.provider),
        );
        p.insert(keys::DESTINATION_MEMBER.into(), json!("destination"));
        p.insert(
            keys::DESTINATION_CONFIG.into(),
            json!(destination.config.to_string()),
        );
        if let Some(g) = &destination.grants {
            p.insert(keys::DESTINATION_GRANTS.into(), g.clone());
        }
        p.insert(
            keys::STREAM.into(),
            json!(serde_json::to_string(stream).expect("stream json")),
        );
        SyncDefinition::new(p)
    }

    /// Run one pass of the sync to its end.
    pub async fn run_pass(&mut self) {
        let def = self.definition();
        let runner = self.runner().await;
        let dal = Arc::new(DAL::new(runner.database().clone()));
        let dispatcher = Dispatcher::new(Arc::new(runner), dal);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        self.outcome = Some(
            dispatcher
                .run_pass(&def, stop)
                .await
                .map_err(|e| e.to_string()),
        );
    }

    /// The cursor committed for the stream, if any.
    pub async fn committed_cursor(&mut self) -> Option<String> {
        let stream = self
            .stream
            .as_ref()
            .expect("the scenario names a stream")
            .stream
            .clone();
        self.dal()
            .await
            .sync_stream_state()
            .get(SYNC, &stream, "")
            .await
            .expect("read the stream state")
            .and_then(|s| s.cursor)
    }

    /// Stop the runner and drop what the scenario made.
    pub async fn teardown(&mut self) {
        if let Some(runner) = self.runner.take() {
            let _ = runner.shutdown().await;
        }
        pg::drop_tables(&self.tables).await;
        if runner_on_postgres() {
            pg::drop_schema(&self.scope).await;
        }
    }
}
