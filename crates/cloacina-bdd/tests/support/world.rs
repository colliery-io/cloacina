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

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use cloacina::dal::DAL;
use cloacina::dal::unified::workflow_execution::ExecutionListFilter;
use cloacina::database::universal_types::UniversalBool;
use cloacina::models::schedule::NewSchedule;
use cloacina::models::workflow_execution::WorkflowExecutionRecord;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina_constructor_contract::sync::{
    ConfiguredStream, MappingSpec, StreamSchema, SyncMode, WriteMode,
};
use cloacina_sync::dispatcher::{Dispatcher, PassOutcome, SyncDefinition};
use cloacina_sync::params::keys;
use cloacina_testing::connectors::rest_mock::RestMock;
use cloacina_workflow::secret::MapSecretResolver;
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
    /// The credential the host applies, a `CredentialSpec` as JSON.
    pub credential: Option<Value>,
    /// A connector definition the side names instead of a provider; `config`
    /// is then its user config, and `secret` the stored secret its auth reads.
    pub definition: Option<String>,
    pub secret: Option<String>,
}

impl Side {
    /// The Postgres connector on a table of the scenario's Postgres.
    pub fn postgres(table: &str) -> Self {
        Self {
            provider: providers::POSTGRES.into(),
            config: json!({ "url": pg::connector_url(), "table": table }),
            grants: Some(json!({ "tcp": [format!("*:{}", pg::port())] })),
            table: Some(table.into()),
            credential: None,
            definition: None,
            secret: None,
        }
    }

    /// A connector definition, with its user config and its secret.
    pub fn definition(name: &str, config: Value, secret: &str) -> Self {
        Self {
            provider: String::new(),
            config,
            grants: None,
            table: None,
            credential: None,
            definition: Some(name.into()),
            secret: Some(secret.into()),
        }
    }

    /// The REST connector on `path` of the scenario's API. It does not retry,
    /// so a refused request fails the read at once.
    pub fn rest(api: &RestMock, path: &str, credential: Option<Value>) -> Self {
        Self {
            provider: providers::REST.into(),
            config: json!({ "base_url": api.base_url(), "path": path, "max_retries": 0 }),
            grants: Some(json!({ "http": [api.grant()] })),
            table: None,
            credential,
            definition: None,
            secret: None,
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
            credential: None,
            definition: None,
            secret: None,
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

/// The scenario's stored secrets: name, then field, then value.
type Secrets = BTreeMap<String, BTreeMap<String, String>>;

/// A runner on `db`, resolving `secrets`. Claims go stale after three seconds
/// without a heartbeat, so a runner that takes over from a dead one does so
/// within a scenario.
async fn start_runner(db: &RunnerDb, secrets: &Secrets) -> DefaultRunner {
    cloacina_sync::ensure_linked();
    providers::staged();
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(false)
        .heartbeat_interval(Duration::from_secs(1))
        .stale_claim_threshold(Duration::from_secs(3))
        .stale_claim_sweep_interval(Duration::from_secs(1))
        .build()
        .expect("runner config");
    let builder = DefaultRunner::builder()
        .with_config(config)
        .secret_resolver(Arc::new(MapSecretResolver::new(secrets.clone())));
    match db {
        RunnerDb::Sqlite(path) => builder.database_url(&format!("sqlite://{}", path.display())),
        RunnerDb::Postgres { schema } => builder.database_url(&pg::url()).schema(schema),
    }
    .build()
    .await
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
    /// The stream's schema, which records must meet or become dead-letters.
    pub schema: Option<StreamSchema>,
    /// The secrets the scenario's runner resolves.
    pub secrets: Secrets,
    /// The scenario's mock JSON API, started on first use.
    api: Option<RestMock>,
    /// The mock's token endpoints, whose requests carry no credential.
    pub token_paths: Vec<String>,
    /// The records the mock's GitHub repositories endpoint serves.
    pub repositories: Vec<Value>,
    /// What the server said when the sync was created.
    pub created: Option<Result<(), String>>,
    /// The answer of the last check or discover.
    pub probed: Option<Result<Value, String>>,
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
            .field("schema", &self.schema)
            .field("secrets", &self.secrets.keys().collect::<Vec<_>>())
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
            schema: None,
            secrets: Secrets::new(),
            api: None,
            token_paths: Vec::new(),
            repositories: Vec::new(),
            created: None,
            probed: None,
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

    /// An incremental stream named as the source names it, upserted on `key`.
    pub fn incremental_stream(&mut self, name: &str, cursor: &str, key: &str) {
        self.stream = Some(ConfiguredStream {
            stream: name.into(),
            sync_mode: SyncMode::Incremental,
            cursor_field: Some(cursor.into()),
            primary_key: Some(vec![key.into()]),
            write_mode: WriteMode::Upsert {
                business_keys: vec![key.into()],
            },
            mapping: MappingSpec::default(),
        });
    }

    /// A full-refresh stream of the source, upserted on `key`.
    pub fn refresh_stream(&mut self, name: &str, key: &str) {
        self.stream = Some(ConfiguredStream {
            stream: name.into(),
            sync_mode: SyncMode::FullRefresh,
            cursor_field: None,
            primary_key: Some(vec![key.into()]),
            write_mode: WriteMode::Upsert {
                business_keys: vec![key.into()],
            },
            mapping: MappingSpec::default(),
        });
    }

    /// The scenario's mock JSON API, started on first use.
    pub fn api(&mut self) -> &RestMock {
        self.api.get_or_insert_with(RestMock::start)
    }

    /// The scenario's runner, started on first use.
    pub async fn runner(&mut self) -> DefaultRunner {
        if let Some(runner) = &self.runner {
            return runner.clone();
        }
        let runner = start_runner(&self.db, &self.secrets).await;
        self.runner = Some(runner.clone());
        runner
    }

    pub async fn dal(&mut self) -> DAL {
        DAL::new(self.runner().await.database().clone())
    }

    fn definition(&self) -> SyncDefinition {
        SyncDefinition::new(self.params())
    }

    /// Create the sync: the check the server runs on a new sync definition.
    pub fn create(&mut self) {
        let params: serde_json::Map<String, Value> = self.params().into_iter().collect();
        self.created = Some(
            cloacina_sync::params::validate(&self.scope, &params)
                .map(|_| ())
                .map_err(|e| e.to_string()),
        );
    }

    /// The sync definition's params.
    fn params(&self) -> HashMap<String, Value> {
        let source = self.source.as_ref().expect("the scenario names a source");
        let destination = self
            .destination
            .as_ref()
            .expect("the scenario names a destination");
        let stream = self.stream.as_ref().expect("the scenario names a stream");
        let mut p: HashMap<String, Value> = HashMap::new();
        p.insert(keys::SYNC.into(), json!(self.scope));
        if let Some(def) = &source.definition {
            // The definition supplies the provider, the grants and the auth.
            p.insert(keys::SOURCE_DEFINITION.into(), json!(def));
            p.insert(keys::SOURCE_CONFIG.into(), source.config.clone());
            if let Some(secret) = &source.secret {
                p.insert(keys::SOURCE_SECRET.into(), json!(secret));
            }
        } else {
            p.insert(keys::SOURCE_PROVIDER.into(), json!(source.provider));
            p.insert(keys::SOURCE_MEMBER.into(), json!("source"));
            p.insert(keys::SOURCE_CONFIG.into(), json!(source.config.to_string()));
        }
        if let Some(g) = &source.grants {
            p.insert(keys::SOURCE_GRANTS.into(), g.clone());
        }
        if let Some(c) = &source.credential {
            p.insert(keys::SOURCE_CREDENTIAL.into(), json!(c.to_string()));
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
        if let Some(s) = &self.schema {
            p.insert(
                keys::SCHEMA.into(),
                json!(serde_json::to_string(s).expect("schema json")),
            );
        }
        if let Some(f) = &self.fault {
            p.insert(keys::FAULT.into(), json!(f));
        }
        p
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
        let secrets = self.secrets.clone();
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
                let runner = start_runner(&db, &secrets).await;
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

    /// The dead-letters of the sync: each record (as JSON) and its reason.
    pub async fn dead_letters(&mut self) -> Vec<(Value, String)> {
        let sync = self.scope.clone();
        self.dal()
            .await
            .sync_dead_letters()
            .list_for_sync(&sync, 1000)
            .await
            .expect("list the dead-letters")
            .into_iter()
            .map(|d| {
                let record = serde_json::from_str(&d.record).unwrap_or(Value::String(d.record));
                (record, d.reason)
            })
            .collect()
    }

    /// The saved context of every Sync execution, as JSON text.
    pub async fn saved_contexts(&mut self) -> Vec<String> {
        let dal = self.dal().await;
        let mut saved = Vec::new();
        for run in dal
            .workflow_execution()
            .list_recent(100)
            .await
            .expect("list executions")
            .into_iter()
            .filter(|e| e.workflow_name == cloacina_sync::WORKFLOW_NAME)
        {
            if let Some(id) = run.context_id
                && let Ok(ctx) = dal.context().read::<Value>(id).await
            {
                saved.push(ctx.to_json().expect("context json"));
            }
        }
        saved
    }

    /// Store a sync definition named `name`, as the server does: an instance
    /// of the Sync workflow. Its source is the echo connector with `segments`
    /// segments; its destination counts the records.
    pub async fn define_echo_sync(&mut self, name: &str, segments: u64) {
        let params = echo_sync_params(name, segments);
        self.dal()
            .await
            .schedule()
            .create(NewSchedule {
                schedule_type: "cron".into(),
                workflow_name: cloacina_sync::WORKFLOW_NAME.into(),
                enabled: Some(UniversalBool(true)),
                cron_expression: None,
                timezone: None,
                catchup_policy: None,
                start_date: None,
                end_date: None,
                trigger_name: None,
                poll_interval_ms: None,
                allow_concurrent: None,
                next_run_at: None,
                params: Some(params.to_string()),
                instance_name: Some(name.to_string()),
            })
            .await
            .expect("store the sync definition");
    }

    /// Change the stored sync definition `name`, with the checks the server
    /// runs on an update: its source now has `segments` segments, under the
    /// sync name `sync`. The outcome is kept in `created`.
    pub async fn change_echo_sync(&mut self, name: &str, sync: &str, segments: u64) {
        let dal = self.dal().await;
        let existing = dal
            .schedule()
            .find_by_instance_name(cloacina_sync::WORKFLOW_NAME, name)
            .await
            .expect("look up the sync definition")
            .expect("the sync definition exists");
        let old: serde_json::Map<String, Value> = existing
            .params
            .as_deref()
            .and_then(|p| serde_json::from_str(p).ok())
            .unwrap_or_default();
        let new = echo_sync_params(sync, segments);
        let checked = cloacina_sync::params::validate_update(
            name,
            &old,
            new.as_object().expect("params are an object"),
        );
        if let Err(e) = checked {
            self.created = Some(Err(e.to_string()));
            return;
        }
        dal.schedule()
            .update_instance(
                existing.id,
                Some(new.to_string()),
                None,
                Some("UTC".into()),
                None,
                true,
            )
            .await
            .expect("update the sync definition");
        self.created = Some(Ok(()));
    }

    /// The cursor committed for the echo stream of the sync `sync`.
    pub async fn sync_cursor(&mut self, sync: &str) -> Option<String> {
        self.dal()
            .await
            .sync_stream_state()
            .get(sync, "echo", "")
            .await
            .expect("read the stream state")
            .and_then(|s| s.cursor)
    }
}

/// The params of a sync named `sync` from the echo source, with `segments`
/// segments, to the echo destination.
fn echo_sync_params(sync: &str, segments: u64) -> Value {
    let stream = ConfiguredStream {
        stream: "echo".into(),
        sync_mode: SyncMode::Incremental,
        cursor_field: Some("n".into()),
        primary_key: Some(vec!["id".into()]),
        write_mode: WriteMode::Upsert {
            business_keys: vec!["id".into()],
        },
        mapping: MappingSpec::default(),
    };
    json!({
        keys::SYNC: sync,
        keys::SOURCE_PROVIDER: providers::ECHO,
        keys::SOURCE_MEMBER: "source",
        keys::SOURCE_CONFIG: json!({ "segments": segments, "rows": ["{\"id\":1}"] }).to_string(),
        keys::DESTINATION_PROVIDER: providers::ECHO,
        keys::DESTINATION_MEMBER: "destination",
        keys::DESTINATION_CONFIG: "{}",
        keys::STREAM: serde_json::to_string(&stream).expect("stream json"),
    })
}

impl SyncWorld {
    /// Run one pass of the stored sync definition `name`, by its name.
    pub async fn run_definition_pass(&mut self, name: &str) {
        let runner = self.runner().await;
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        self.outcome = Some(
            cloacina_sync::run_pass(&runner, name, stop)
                .await
                .map_err(|e| e.to_string()),
        );
    }

    /// The executions a fire of the instance `name` started, oldest first.
    pub async fn instance_executions(&mut self, name: &str) -> Vec<WorkflowExecutionRecord> {
        let mut runs = self
            .dal()
            .await
            .workflow_execution()
            .list_filtered(ExecutionListFilter {
                instance_name: Some(name.to_string()),
                limit: 1000,
                ..Default::default()
            })
            .await
            .expect("list the instance's executions");
        runs.reverse();
        runs
    }

    /// Run a check or a discover of one side through the probe workflow, as
    /// the server's routes do: the answer, or why it could not run.
    pub async fn probe(
        &mut self,
        action: &str,
        request: &cloacina_api_types::ConnectorProbeRequest,
    ) -> Result<Value, String> {
        use cloacina::executor::{WorkflowExecutor, WorkflowStatus};
        cloacina_sync::probe::validate(action, request).map_err(|e| e.to_string())?;
        let runner = self.runner().await;
        let result = runner
            .execute(
                cloacina_sync::probe::PROBE_WORKFLOW,
                cloacina_sync::probe::context(action, request),
            )
            .await
            .map_err(|e| e.to_string())?;
        if result.status != WorkflowStatus::Completed {
            let dal = self.dal().await;
            return Err(cloacina_sync::dispatcher::failure(&dal, result.execution_id).await);
        }
        Ok(result
            .final_context
            .get(cloacina_sync::probe::keys::RESULT)
            .cloned()
            .unwrap_or_default())
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
