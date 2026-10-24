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
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use cloacina::dal::DAL;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina_constructor_contract::sync::{ConfiguredStream, MappingSpec, SyncMode, WriteMode};
use cloacina_sync::dispatcher::{Dispatcher, PassOutcome, SyncDefinition};
use cloacina_sync::params::keys;
use serde_json::{Value, json};

use super::{pg, providers};

/// A port nothing listens on: a database there refuses connections.
const REFUSING_PORT: u16 = 9;

/// One side of the sync: a provider package, its config, its network grants.
#[derive(Debug, Clone)]
pub struct Side {
    pub provider: String,
    pub config: Value,
    pub grants: Option<Value>,
    /// The scenario table, for a Postgres side.
    pub table: Option<String>,
}

impl Side {
    /// The Postgres connector on a table of the scenario's Postgres.
    pub fn postgres(table: &str) -> Self {
        Self {
            provider: providers::POSTGRES.into(),
            config: json!({ "url": pg::connector_url(), "table": table }),
            grants: Some(json!({ "tcp": [format!("*:{}", pg::port())] })),
            table: Some(table.into()),
        }
    }

    /// The same table, behind a database that refuses connections.
    pub fn refusing(table: &str) -> Self {
        Self {
            provider: providers::POSTGRES.into(),
            config: json!({
                "url": format!("postgres://cloacina:cloacina@127.0.0.1:{REFUSING_PORT}/cloacina?sslmode=disable"),
                "table": table,
            }),
            grants: Some(json!({ "tcp": [format!("*:{REFUSING_PORT}")] })),
            table: Some(table.into()),
        }
    }
}

/// Where a scenario's runner keeps its state. Two runners on the same one
/// share the sync's executions, its claims and its cursor.
#[derive(Debug, Clone)]
enum RunnerDb {
    Sqlite(PathBuf),
    Postgres { schema: String },
}

/// The database of a scenario's runner, from `CLOACINA_BDD_RUNNER`.
fn runner_on_postgres() -> bool {
    match std::env::var("CLOACINA_BDD_RUNNER").as_deref() {
        Ok("postgres") => true,
        Ok("sqlite") | Err(_) => false,
        Ok(other) => panic!("CLOACINA_BDD_RUNNER is `{other}`: use `sqlite` or `postgres`"),
    }
}

/// A runner on `db`. Claims go stale after three seconds without a heartbeat,
/// so a runner that takes over from a dead one does so within a scenario.
async fn start_runner(db: &RunnerDb) -> DefaultRunner {
    cloacina_sync::ensure_linked();
    providers::staged();
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .heartbeat_interval(Duration::from_secs(1))
        .stale_claim_threshold(Duration::from_secs(3))
        .stale_claim_sweep_interval(Duration::from_secs(1))
        .build()
        .expect("runner config");
    match db {
        RunnerDb::Sqlite(path) => {
            DefaultRunner::with_config(&format!("sqlite://{}", path.display()), config).await
        }
        RunnerDb::Postgres { schema } => {
            DefaultRunner::builder()
                .database_url(&pg::url())
                .schema(schema)
                .with_config(config)
                .build()
                .await
        }
    }
    .expect("the scenario's runner")
}

#[derive(cucumber::World)]
#[world(init = Self::new)]
pub struct SyncWorld {
    /// Unique per scenario: the sync's name, the prefix of the scenario's
    /// tables, and the runner's schema on Postgres.
    pub scope: String,
    db: RunnerDb,
    runner: Option<DefaultRunner>,
    pub source: Option<Side>,
    pub destination: Option<Side>,
    pub stream: Option<ConfiguredStream>,
    /// A test-only fault of the write task (`cloacina_sync::fault`).
    pub fault: Option<String>,
    pub outcome: Option<Result<PassOutcome, String>>,
    /// Tables to drop when the scenario ends.
    tables: Vec<String>,
}

impl fmt::Debug for SyncWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyncWorld")
            .field("scope", &self.scope)
            .field("db", &self.db)
            .field("source", &self.source)
            .field("destination", &self.destination)
            .field("stream", &self.stream)
            .field("fault", &self.fault)
            .field("outcome", &self.outcome)
            .finish_non_exhaustive()
    }
}

impl SyncWorld {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let scope = format!(
            "bdd_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let db = if runner_on_postgres() {
            RunnerDb::Postgres {
                schema: scope.clone(),
            }
        } else {
            let dir = std::env::temp_dir().join("cloacina-bdd");
            std::fs::create_dir_all(&dir).expect("runner database dir");
            RunnerDb::Sqlite(dir.join(format!("{scope}.db")))
        };
        Self {
            scope,
            db,
            runner: None,
            source: None,
            destination: None,
            stream: None,
            fault: None,
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

    /// The scenario's runner, started on first use.
    pub async fn runner(&mut self) -> DefaultRunner {
        if let Some(runner) = &self.runner {
            return runner.clone();
        }
        let runner = start_runner(&self.db).await;
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
        p.insert(keys::SYNC.into(), json!(self.scope));
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
        if let Some(f) = &self.fault {
            p.insert(keys::FAULT.into(), json!(f));
        }
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

    /// Start a pass on a runner of its own runtime, wait until the destination
    /// holds every source record, then drop that runtime: the runner dies with
    /// its write task held before the commit, without a shutdown. The write
    /// task is held for `hold`.
    pub async fn kill_runner_mid_write(&mut self, hold: Duration) {
        assert!(
            self.runner.is_none(),
            "the doomed runner must be the first on the scenario's database"
        );
        self.fault = Some(format!("slow_write_ms:{}", hold.as_millis()));
        let def = self.definition();
        let db = self.db.clone();
        let source = self.side_table(true);
        let destination = self.side_table(false);
        let expected = pg::count(&source).await;

        let (kill_tx, kill_rx) = std::sync::mpsc::channel::<()>();
        let doomed = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("the doomed runner's runtime");
            rt.spawn(async move {
                let runner = start_runner(&db).await;
                let dal = Arc::new(DAL::new(runner.database().clone()));
                let (_stop_tx, stop) = tokio::sync::watch::channel(false);
                let _ = Dispatcher::new(Arc::new(runner.clone()), dal)
                    .run_pass(&def, stop)
                    .await;
            });
            let _ = kill_rx.recv();
            rt.shutdown_background();
        });

        let deadline = Instant::now() + Duration::from_secs(120);
        while pg::count_if_exists(&destination).await < expected {
            assert!(
                Instant::now() < deadline,
                "the destination never received the segment"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        kill_tx.send(()).expect("kill the doomed runner");
        doomed.join().expect("the doomed runner's thread");
    }

    fn side_table(&self, source: bool) -> String {
        let side = if source {
            &self.source
        } else {
            &self.destination
        };
        side.as_ref()
            .and_then(|s| s.table.clone())
            .expect("a Postgres side")
    }

    /// The cursor committed for the stream, if any.
    pub async fn committed_cursor(&mut self) -> Option<String> {
        let stream = self
            .stream
            .as_ref()
            .expect("the scenario names a stream")
            .stream
            .clone();
        let sync = self.scope.clone();
        self.dal()
            .await
            .sync_stream_state()
            .get(&sync, &stream, "")
            .await
            .expect("read the stream state")
            .and_then(|s| s.cursor)
    }

    /// The attempt numbers of the write task of each Sync execution, sorted.
    pub async fn write_attempts(&mut self) -> Vec<i32> {
        let dal = self.dal().await;
        let mut attempts = Vec::new();
        for run in dal
            .workflow_execution()
            .list_recent(100)
            .await
            .expect("list executions")
            .into_iter()
            .filter(|e| e.workflow_name == cloacina_sync::WORKFLOW_NAME)
        {
            let tasks = dal
                .task_execution()
                .get_all_tasks_for_workflow(run.id)
                .await
                .expect("list tasks");
            if let Some(write) = tasks.iter().find(|t| t.task_name.ends_with("::write")) {
                attempts.push(write.attempt);
            }
        }
        attempts.sort();
        attempts
    }

    /// Stop the runner and drop what the scenario made.
    pub async fn teardown(&mut self) {
        if let Some(runner) = self.runner.take() {
            let _ = runner.shutdown().await;
        }
        pg::drop_tables(&self.tables).await;
        match &self.db {
            RunnerDb::Postgres { schema } => pg::drop_schema(schema).await,
            RunnerDb::Sqlite(path) => {
                for suffix in ["", "-wal", "-shm"] {
                    let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
                }
            }
        }
    }
}
