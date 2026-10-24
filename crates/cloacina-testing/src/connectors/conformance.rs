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

//! The connector conformance kit (CLOACINA-A-0014, CLOACINA-T-0983).
//!
//! A connector's conformance file is TOML beside the connector crate. It names
//! the connector's config, the cases for each of the five methods, and the
//! expected outputs. [`run`] loads the connector through a [`Host`], runs every
//! case, and returns a [`Report`] that names each failure by method, case and
//! difference.
//!
//! The kit does not depend on the engine: the caller supplies the [`Host`] that
//! stages and loads the connector (the Sync crate's tests wrap the real
//! connector host), and a [`Service`] that runs the file's setup and verify
//! steps against the connector's test service.
//!
//! # Matching
//!
//! Expected values are JSON (TOML in the file). An expected object matches an
//! actual one when every field it names matches; fields it leaves out are not
//! compared. Arrays match as multisets unless the contract promises an order:
//! checkpoints are compared in order, and records when the case sets
//! `ordered = true`. Record and cursor text is parsed as JSON before it is
//! compared. Numbers compare by value.
//!
//! # The file
//!
//! ```toml
//! connector = "echo"          # names the report
//! crate = "."                 # the connector crate, relative to this file
//!
//! [source]                    # absent when the connector is no source
//! member = "source"           # default: "source"
//! config = { rows = ['{"id":1}'] }
//! grants = { tcp = ["*:5432"] }   # optional, a GrantSpec
//!
//! [destination]
//! config = {}
//! credential = { kind = "aws_sigv4", secret = "store", region = "us-east-1" }
//!                             # optional; a credential spec as a sync names it
//!
//! [secrets.store]             # the secrets credentials name; test values only
//! access_key_id = "minioadmin"
//! secret_access_key = "${MINIO_SECRET:-minioadmin}"
//!
//! [[setup]]                   # steps run once, before every case
//! sql = "CREATE TABLE ..."
//!
//! [spec]                      # fields the spec must have
//! name = "echo"
//!
//! [[check]]
//! case = "not json"
//! side = "source"             # default: source, else destination
//! secret = "wrong"            # optional; another [secrets] entry for the credential
//! config_text = "{not json"   # or `config = {...}`, merged over the side's
//! success = false
//! message_contains = "JSON"   # optional
//!
//! [discover]
//! streams = [{ name = "echo" }]
//! exact = true                # default: the catalog has no other streams
//!
//! [[read]]
//! case = "fresh"
//! stream = { stream = "echo" }    # a ConfiguredStream; sync_mode, write_mode
//!                                 # and mapping default to FullRefresh,
//!                                 # Append and none
//! state = { cursor = "2" }        # optional; the cursor as a JSON value
//! max_checkpoints = 1             # optional; stop the read after this many
//! resume_after = 2                # optional; read to this checkpoint, then
//!                                 # expect what a read from its state gives
//! records = [{ id = 1 }]
//! checkpoints = [{ cursor = "2" }]
//! dead_letters = [{ record = { id = "x" }, reason_contains = "not" }]
//!
//! [[write]]
//! case = "two rows"
//! stream = { stream = "echo" }
//! batches = [[{ id = 1 }, { id = 2 }]]
//! change_batches = [[{ op = "Delete", data = { id = 1 } }]]   # optional, CDC
//! accepted = 2
//! verify = [{ sql = "SELECT id FROM t", rows = [{ id = 1 }, { id = 2 }] }]
//! ```
//!
//! `${NAME}` and `${NAME:-default}` in the file are replaced before it is
//! parsed: from the variables the caller passes to [`run_with`] first, then
//! from the environment. So a file can name its test service's address, or a
//! mock's port.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use cloacina_constructor_contract::sync::{
    ChangeOp, ChangeRecord, CheckResult, Config, ConfiguredStream, ConnectorRole, ConnectorSpec,
    DiscoverOutcome, Partition, ReadContext, ReadMessage, RecordBatch, StreamState, WriteContext,
    WriteOutcome, WriteResult,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

/// A read as an iterator of decoded messages.
pub type ReadIter<'a> = Box<dyn Iterator<Item = Result<ReadMessage, String>> + 'a>;

/// One loaded connector member. Errors are host errors (the call failed),
/// not connector-domain results.
pub trait Connector {
    fn spec(&self) -> Result<ConnectorSpec, String>;
    fn check(&self, config: &Config) -> Result<CheckResult, String>;
    fn discover(&self, config: &Config) -> Result<DiscoverOutcome, String>;
    fn read(&self, ctx: &ReadContext) -> Result<ReadIter<'_>, String>;
    fn write(&self, ctx: &WriteContext, batches: Vec<RecordBatch>) -> Result<WriteOutcome, String>;
}

/// A side's credential with its secret resolved: the spec as a sync names it,
/// and the secret's fields from the file's `[secrets]`.
#[derive(Clone)]
pub struct Credential {
    pub spec: Value,
    pub secret: BTreeMap<String, String>,
}

// Test secrets still stay out of failure messages.
impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credential")
            .field("spec", &self.spec)
            .field("fields", &self.secret.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Stages and loads connectors.
pub trait Host {
    /// Build and stage the connector crate at `crate_dir`; return the package
    /// name [`Host::load`] takes.
    fn prepare(&self, crate_dir: &Path) -> Result<String, String>;
    /// Load `member` of `package` with `config` bound, `grants` (a JSON
    /// `GrantSpec`, or none for deny-all), and the credential the host applies
    /// to the connector's requests, if any.
    fn load(
        &self,
        package: &str,
        member: &str,
        config: &Config,
        grants: Option<&Value>,
        credential: Option<&Credential>,
    ) -> Result<Box<dyn Connector>, String>;
}

/// Runs a file's setup and verify steps against the connector's test service.
pub trait Service {
    /// Run one step: the step's fields other than `rows` and `ordered`, for
    /// example `{"sql": "..."}`. Return the rows it produced as JSON objects,
    /// or none for a statement.
    fn run(&self, step: &Map<String, Value>) -> Result<Vec<Value>, String>;
}

/// A conformance file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConformanceFile {
    pub connector: String,
    #[serde(rename = "crate", default = "dot")]
    pub crate_dir: String,
    pub source: Option<Side>,
    pub destination: Option<Side>,
    /// The secrets the sides' credentials name, by name.
    #[serde(default)]
    pub secrets: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub setup: Vec<Step>,
    pub spec: Option<Value>,
    #[serde(default)]
    pub check: Vec<CheckCase>,
    pub discover: Option<DiscoverCase>,
    #[serde(default)]
    pub read: Vec<ReadCase>,
    #[serde(default)]
    pub write: Vec<WriteCase>,
}

fn dot() -> String {
    ".".to_string()
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

fn yes() -> bool {
    true
}

/// One side of the connector: which member, with what config and grants.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Side {
    pub member: Option<String>,
    #[serde(default = "empty_object")]
    pub config: Value,
    pub grants: Option<Value>,
    /// A credential spec, as a sync's `source_credential` names it; its
    /// `secret` is an entry of the file's `[secrets]`.
    pub credential: Option<Value>,
}

/// A setup or verify step. With `rows`, the step's output must match them.
#[derive(Debug, Deserialize)]
pub struct Step {
    pub rows: Option<Vec<Value>>,
    #[serde(default)]
    pub ordered: bool,
    #[serde(flatten)]
    pub action: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckCase {
    pub case: String,
    pub side: Option<String>,
    pub config: Option<Value>,
    pub config_text: Option<String>,
    /// Another `[secrets]` entry for the side's credential.
    pub secret: Option<String>,
    pub success: bool,
    pub message_contains: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoverCase {
    pub config: Option<Value>,
    pub streams: Vec<Value>,
    #[serde(default = "yes")]
    pub exact: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadCase {
    pub case: String,
    pub config: Option<Value>,
    pub stream: Value,
    pub partition: Option<Value>,
    pub state: Option<Value>,
    pub max_checkpoints: Option<usize>,
    /// Read up to this checkpoint first, then read again from its state; the
    /// expected values are for the second read. Tests a resume mid-stream.
    pub resume_after: Option<usize>,
    pub records: Option<Vec<Value>>,
    #[serde(default)]
    pub ordered: bool,
    pub changes: Option<Vec<Value>>,
    pub checkpoints: Option<Vec<Value>>,
    pub dead_letters: Option<Vec<Value>>,
    pub fatal: Option<Value>,
    #[serde(default)]
    pub setup: Vec<Step>,
    #[serde(default)]
    pub verify: Vec<Step>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteCase {
    pub case: String,
    pub config: Option<Value>,
    pub stream: Value,
    #[serde(default)]
    pub batches: Vec<Vec<Value>>,
    /// CDC batches, each a list of `{op = "Insert" | "Update" | "Delete",
    /// data = {...}}`, written after `batches`.
    #[serde(default)]
    pub change_batches: Vec<Vec<Value>>,
    pub accepted: Option<u64>,
    pub dead_letters: Option<Vec<Value>>,
    pub error: Option<Value>,
    #[serde(default)]
    pub setup: Vec<Step>,
    #[serde(default)]
    pub verify: Vec<Step>,
}

/// One failed case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// `spec`, `check`, `discover`, `read` or `write`; or `file`, `load`,
    /// `setup` and `verify` for the kit's own steps.
    pub method: &'static str,
    pub case: String,
    pub difference: String,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} `{}`: {}", self.method, self.case, self.difference)
    }
}

/// What a run found.
#[derive(Debug, Default)]
pub struct Report {
    pub connector: String,
    /// The cases that passed, as `method case`.
    pub passed: Vec<String>,
    pub failures: Vec<Failure>,
}

impl Report {
    pub fn is_pass(&self) -> bool {
        self.failures.is_empty()
    }

    /// Panic with every failure when the run did not pass.
    pub fn assert_passed(&self) {
        if !self.is_pass() {
            panic!("{self}");
        }
    }

    fn fail(&mut self, method: &'static str, case: &str, difference: impl Into<String>) {
        self.failures.push(Failure {
            method,
            case: case.to_string(),
            difference: difference.into(),
        });
    }

    fn pass(&mut self, method: &'static str, case: &str) {
        self.passed.push(format!("{method} {case}"));
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.failures.is_empty() {
            return write!(
                f,
                "conformance `{}`: {} cases passed",
                self.connector,
                self.passed.len()
            );
        }
        writeln!(
            f,
            "conformance `{}`: {} failed, {} passed",
            self.connector,
            self.failures.len(),
            self.passed.len()
        )?;
        for failure in &self.failures {
            writeln!(f, "  - {failure}")?;
        }
        Ok(())
    }
}

/// Run the conformance file at `path`.
pub fn run(path: &Path, host: &dyn Host, service: Option<&dyn Service>) -> Report {
    run_with(path, &BTreeMap::new(), host, service)
}

/// Run the conformance file at `path`, with `vars` for its `${NAME}`s.
pub fn run_with(
    path: &Path,
    vars: &BTreeMap<String, String>,
    host: &dyn Host,
    service: Option<&dyn Service>,
) -> Report {
    let base = path.parent().unwrap_or(Path::new("."));
    match std::fs::read_to_string(path) {
        Ok(text) => run_text_with(&text, vars, base, host, service),
        Err(e) => {
            let mut report = Report {
                connector: path.display().to_string(),
                ..Report::default()
            };
            report.fail("file", &path.display().to_string(), e.to_string());
            report
        }
    }
}

/// Run a conformance file's text; `base` is the directory `crate` is relative to.
pub fn run_text(text: &str, base: &Path, host: &dyn Host, service: Option<&dyn Service>) -> Report {
    run_text_with(text, &BTreeMap::new(), base, host, service)
}

/// [`run_text`] with `vars` for the file's `${NAME}`s.
pub fn run_text_with(
    text: &str,
    vars: &BTreeMap<String, String>,
    base: &Path,
    host: &dyn Host,
    service: Option<&dyn Service>,
) -> Report {
    let mut report = Report::default();
    let file = match interpolate_with(text, vars).and_then(|t| {
        toml::from_str::<ConformanceFile>(&t).map_err(|e| format!("does not parse: {e}"))
    }) {
        Ok(f) => f,
        Err(e) => {
            report.connector = "<unparsed>".into();
            report.fail("file", "conformance file", e);
            return report;
        }
    };
    report.connector = file.connector.clone();
    Runner {
        file: &file,
        base,
        host,
        service,
        report: &mut report,
    }
    .run();
    report
}

/// Replace `${NAME}` and `${NAME:-default}` from the environment.
pub fn interpolate(text: &str) -> Result<String, String> {
    interpolate_with(text, &BTreeMap::new())
}

/// Replace `${NAME}` and `${NAME:-default}` from `vars`, then the environment.
pub fn interpolate_with(text: &str, vars: &BTreeMap<String, String>) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find('}')
            .ok_or_else(|| "an unclosed `${` in the file".to_string())?;
        let body = &after[..end];
        let (name, default) = match body.split_once(":-") {
            Some((n, d)) => (n, Some(d)),
            None => (body, None),
        };
        let value = vars
            .get(name)
            .cloned()
            .ok_or(())
            .or_else(|()| std::env::var(name));
        match (value, default) {
            (Ok(v), _) => out.push_str(&v),
            (Err(_), Some(d)) => out.push_str(d),
            (Err(_), None) => {
                return Err(format!("`${{{name}}}` is not set and has no default"));
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

struct Runner<'a> {
    file: &'a ConformanceFile,
    base: &'a Path,
    host: &'a dyn Host,
    service: Option<&'a dyn Service>,
    report: &'a mut Report,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Source,
    Destination,
}

impl Role {
    fn name(self) -> &'static str {
        match self {
            Role::Source => "source",
            Role::Destination => "destination",
        }
    }
}

impl Runner<'_> {
    fn run(&mut self) {
        let crate_dir = self.base.join(&self.file.crate_dir);
        let package = match self.host.prepare(&crate_dir) {
            Ok(p) => p,
            Err(e) => {
                self.report
                    .fail("load", &crate_dir.display().to_string(), e);
                return;
            }
        };
        if self.file.source.is_none() && self.file.destination.is_none() {
            self.report.fail(
                "file",
                "conformance file",
                "names neither a [source] nor a [destination]",
            );
            return;
        }
        if !self.steps("setup", "file setup", &self.file.setup) {
            return;
        }
        self.spec(&package);
        for case in &self.file.check {
            self.check(&package, case);
        }
        if let Some(case) = &self.file.discover {
            self.discover(&package, case);
        }
        for case in &self.file.read {
            self.read(&package, case);
        }
        for case in &self.file.write {
            self.write(&package, case);
        }
    }

    fn side(&self, role: Role) -> Option<&Side> {
        match role {
            Role::Source => self.file.source.as_ref(),
            Role::Destination => self.file.destination.as_ref(),
        }
    }

    fn member(&self, role: Role) -> String {
        self.side(role)
            .and_then(|s| s.member.clone())
            .unwrap_or_else(|| role.name().to_string())
    }

    /// The side's config with `overlay`'s fields merged over it.
    fn config(&self, role: Role, overlay: Option<&Value>) -> Value {
        let mut base = self
            .side(role)
            .map(|s| s.config.clone())
            .unwrap_or_else(empty_object);
        if let (Value::Object(b), Some(Value::Object(o))) = (&mut base, overlay) {
            for (k, v) in o {
                b.insert(k.clone(), v.clone());
            }
        }
        base
    }

    /// The side's credential with its secret, or with `secret` in its place.
    fn credential(&self, role: Role, secret: Option<&str>) -> Result<Option<Credential>, String> {
        let Some(spec) = self.side(role).and_then(|s| s.credential.as_ref()) else {
            return Ok(None);
        };
        let name = match secret {
            Some(n) => n.to_string(),
            None => spec
                .get("secret")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("the {} credential names no `secret`", role.name()))?
                .to_string(),
        };
        let fields = self
            .file
            .secrets
            .get(&name)
            .ok_or_else(|| format!("[secrets] has no `{name}`"))?;
        Ok(Some(Credential {
            spec: spec.clone(),
            secret: fields.clone(),
        }))
    }

    fn load(
        &mut self,
        package: &str,
        role: Role,
        config: &Config,
        method: &'static str,
        case: &str,
    ) -> Option<Box<dyn Connector>> {
        self.load_with(package, role, config, None, method, case)
    }

    fn load_with(
        &mut self,
        package: &str,
        role: Role,
        config: &Config,
        secret: Option<&str>,
        method: &'static str,
        case: &str,
    ) -> Option<Box<dyn Connector>> {
        let credential = match self.credential(role, secret) {
            Ok(c) => c,
            Err(e) => {
                self.report.fail("file", case, e);
                return None;
            }
        };
        let grants = self.side(role).and_then(|s| s.grants.as_ref());
        match self.host.load(
            package,
            &self.member(role),
            config,
            grants,
            credential.as_ref(),
        ) {
            Ok(c) => Some(c),
            Err(e) => {
                self.report.fail(
                    method,
                    case,
                    format!("loading the {} failed: {e}", role.name()),
                );
                None
            }
        }
    }

    /// Run steps; a failed step fails the case and stops it.
    fn steps(&mut self, method: &'static str, case: &str, steps: &[Step]) -> bool {
        if steps.is_empty() {
            return true;
        }
        let Some(service) = self.service else {
            self.report.fail(
                method,
                case,
                "the file has setup or verify steps, but the run has no service",
            );
            return false;
        };
        for step in steps {
            match service.run(&step.action) {
                Err(e) => {
                    self.report.fail(
                        method,
                        case,
                        format!("step {} failed: {e}", Value::Object(step.action.clone())),
                    );
                    return false;
                }
                Ok(rows) => {
                    if let Some(expected) = &step.rows {
                        let diff = if step.ordered {
                            ordered_diff(expected, &rows)
                        } else {
                            multiset_diff(expected, &rows, true)
                        };
                        if let Some(d) = diff {
                            self.report.fail(
                                method,
                                case,
                                format!(
                                    "step {} rows differ: {d}",
                                    Value::Object(step.action.clone())
                                ),
                            );
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    fn spec(&mut self, package: &str) {
        let role = if self.file.source.is_some() {
            Role::Source
        } else {
            Role::Destination
        };
        let config = Config {
            json: self.config(role, None).to_string(),
        };
        let Some(conn) = self.load(package, role, &config, "spec", "spec") else {
            return;
        };
        let spec = match conn.spec() {
            Ok(s) => s,
            Err(e) => {
                return self
                    .report
                    .fail("spec", "spec", format!("the call failed: {e}"));
            }
        };
        let mut diffs = Vec::new();
        if serde_json::from_str::<Value>(&spec.config_schema)
            .map(|v| !v.is_object())
            .unwrap_or(true)
        {
            diffs.push(format!(
                "config_schema is not a JSON object: {}",
                spec.config_schema
            ));
        }
        for (side, declared) in [
            (Role::Source, ConnectorRole::Source),
            (Role::Destination, ConnectorRole::Destination),
        ] {
            if self.side(side).is_some() && !spec.roles.contains(&declared) {
                diffs.push(format!(
                    "the file tests a {} but roles are {:?}",
                    side.name(),
                    spec.roles
                ));
            }
        }
        if spec.supported_sync_modes.is_empty() {
            diffs.push("supported_sync_modes is empty".into());
        }
        if let Some(expected) = &self.file.spec {
            let actual = serde_json::to_value(&spec).unwrap_or(Value::Null);
            if let Some(d) = value_diff(expected, &actual) {
                diffs.push(d);
            }
        }
        if diffs.is_empty() {
            self.report.pass("spec", "spec");
        } else {
            self.report.fail("spec", "spec", diffs.join("; "));
        }
    }

    fn check(&mut self, package: &str, case: &CheckCase) {
        let role = match case.side.as_deref() {
            Some("source") => Role::Source,
            Some("destination") => Role::Destination,
            Some(other) => {
                return self.report.fail(
                    "check",
                    &case.case,
                    format!("side `{other}` is not `source` or `destination`"),
                );
            }
            None if self.file.source.is_some() => Role::Source,
            None => Role::Destination,
        };
        let json = match &case.config_text {
            Some(text) => text.clone(),
            None => self.config(role, case.config.as_ref()).to_string(),
        };
        let config = Config { json };
        // A connector binds its config at load, so a bad config is checked on a
        // handle loaded with it.
        let Some(conn) = self.load_with(
            package,
            role,
            &config,
            case.secret.as_deref(),
            "check",
            &case.case,
        ) else {
            return;
        };
        let result = match conn.check(&config) {
            Ok(r) => r,
            Err(e) => {
                return self
                    .report
                    .fail("check", &case.case, format!("the call failed: {e}"));
            }
        };
        let mut diffs = Vec::new();
        if result.success != case.success {
            diffs.push(format!(
                "success is {} (message {:?}), expected {}",
                result.success, result.message, case.success
            ));
        }
        if let Some(want) = &case.message_contains {
            let msg = result.message.clone().unwrap_or_default();
            if !msg.contains(want.as_str()) {
                diffs.push(format!("message {msg:?} does not contain {want:?}"));
            }
        }
        if diffs.is_empty() {
            self.report.pass("check", &case.case);
        } else {
            self.report.fail("check", &case.case, diffs.join("; "));
        }
    }

    fn discover(&mut self, package: &str, case: &DiscoverCase) {
        let name = "discover";
        let config = Config {
            json: self.config(Role::Source, case.config.as_ref()).to_string(),
        };
        let Some(conn) = self.load(package, Role::Source, &config, "discover", name) else {
            return;
        };
        match conn.discover(&config) {
            Err(e) => self
                .report
                .fail("discover", name, format!("the call failed: {e}")),
            Ok(DiscoverOutcome::Error(e)) => self.report.fail(
                "discover",
                name,
                format!(
                    "the connector returned an error: {:?}: {}",
                    e.kind, e.message
                ),
            ),
            Ok(DiscoverOutcome::Catalog(catalog)) => {
                let actual: Vec<Value> = catalog
                    .streams
                    .iter()
                    .map(|s| serde_json::to_value(s).unwrap_or(Value::Null))
                    .collect();
                match multiset_diff(&case.streams, &actual, case.exact) {
                    None => self.report.pass("discover", name),
                    Some(d) => self
                        .report
                        .fail("discover", name, format!("streams differ: {d}")),
                }
            }
        }
    }

    fn read(&mut self, package: &str, case: &ReadCase) {
        if self.file.source.is_none() {
            return self
                .report
                .fail("read", &case.case, "the file has no [source]");
        }
        if !self.steps("setup", &case.case, &case.setup) {
            return;
        }
        let mut ctx = match read_context(case) {
            Ok(c) => c,
            Err(e) => return self.report.fail("file", &case.case, e),
        };
        let config = Config {
            json: self.config(Role::Source, case.config.as_ref()).to_string(),
        };
        let Some(conn) = self.load(package, Role::Source, &config, "read", &case.case) else {
            return;
        };
        if let Some(n) = case.resume_after {
            match checkpoint_at(conn.as_ref(), &ctx, n) {
                Ok(state) => ctx.state = state,
                Err(e) => return self.report.fail("read", &case.case, e),
            }
        }
        let iter = match conn.read(&ctx) {
            Ok(i) => i,
            Err(e) => {
                return self
                    .report
                    .fail("read", &case.case, format!("the call failed: {e}"));
            }
        };
        let mut records = Vec::new();
        let mut changes = Vec::new();
        let mut checkpoints = Vec::new();
        let mut dead = Vec::new();
        let mut fatal = None;
        for msg in iter {
            match msg {
                Err(e) => {
                    return self
                        .report
                        .fail("read", &case.case, format!("the stream failed: {e}"));
                }
                Ok(ReadMessage::Records(RecordBatch::Rows(rows))) => {
                    records.extend(rows.iter().map(|r| parse_text(r)))
                }
                Ok(ReadMessage::Records(RecordBatch::Changes(cs))) => {
                    changes.extend(cs.iter().map(|c| {
                        json!({ "op": serde_json::to_value(c.op).unwrap_or(Value::Null),
                                "data": parse_text(&c.data) })
                    }))
                }
                Ok(ReadMessage::Records(RecordBatch::Arrow(_))) => {
                    return self.report.fail(
                        "read",
                        &case.case,
                        "the read yielded an Arrow batch, which the kit does not compare yet",
                    );
                }
                Ok(ReadMessage::Checkpoint(state)) => {
                    checkpoints.push(state_value(&state));
                    if case.max_checkpoints.is_some_and(|m| checkpoints.len() >= m) {
                        break;
                    }
                }
                Ok(ReadMessage::DeadLettered(d)) => {
                    dead.push(json!({ "record": parse_text(&d.record), "reason": d.reason }))
                }
                Ok(ReadMessage::Log(_)) => {}
                Ok(ReadMessage::Fatal(e)) => {
                    fatal = Some(serde_json::to_value(&e).unwrap_or(Value::Null));
                    break;
                }
            }
        }
        drop(conn);

        let mut diffs = Vec::new();
        match (&case.fatal, &fatal) {
            (None, Some(f)) => diffs.push(format!("the read ended with a fatal error: {f}")),
            (Some(want), None) => diffs.push(format!("expected a fatal error {want}, got none")),
            (Some(want), Some(got)) => {
                if let Some(d) = value_diff(want, got) {
                    diffs.push(format!("fatal error differs: {d}"));
                }
            }
            (None, None) => {}
        }
        if let Some(want) = &case.records {
            let d = if case.ordered {
                ordered_diff(want, &records)
            } else {
                multiset_diff(want, &records, true)
            };
            if let Some(d) = d {
                diffs.push(format!("records differ: {d}"));
            }
        }
        if let Some(want) = &case.changes
            && let Some(d) = ordered_diff(want, &changes)
        {
            diffs.push(format!("changes differ: {d}"));
        }
        if let Some(want) = &case.checkpoints
            && let Some(d) = ordered_diff(want, &checkpoints)
        {
            diffs.push(format!("checkpoints differ: {d}"));
        }
        if let Some(want) = &case.dead_letters
            && let Some(d) = dead_letter_diff(want, &dead)
        {
            diffs.push(format!("dead-letters differ: {d}"));
        }
        if !diffs.is_empty() {
            return self.report.fail("read", &case.case, diffs.join("; "));
        }
        if self.steps("verify", &case.case, &case.verify) {
            self.report.pass("read", &case.case);
        }
    }

    fn write(&mut self, package: &str, case: &WriteCase) {
        if self.file.destination.is_none() {
            return self
                .report
                .fail("write", &case.case, "the file has no [destination]");
        }
        if !self.steps("setup", &case.case, &case.setup) {
            return;
        }
        let stream = match configured_stream(&case.stream) {
            Ok(s) => s,
            Err(e) => return self.report.fail("file", &case.case, e),
        };
        let config = Config {
            json: self
                .config(Role::Destination, case.config.as_ref())
                .to_string(),
        };
        let Some(conn) = self.load(package, Role::Destination, &config, "write", &case.case) else {
            return;
        };
        let mut batches: Vec<RecordBatch> = case
            .batches
            .iter()
            .map(|rows| RecordBatch::Rows(rows.iter().map(row_text).collect()))
            .collect();
        for changes in &case.change_batches {
            let mut out = Vec::with_capacity(changes.len());
            for c in changes {
                let op = match c
                    .get("op")
                    .map(|o| serde_json::from_value::<ChangeOp>(o.clone()))
                {
                    Some(Ok(op)) => op,
                    _ => {
                        return self.report.fail(
                            "file",
                            &case.case,
                            format!("a change needs `op` (Insert, Update or Delete): {c}"),
                        );
                    }
                };
                out.push(ChangeRecord {
                    op,
                    data: row_text(c.get("data").unwrap_or(&Value::Null)),
                });
            }
            batches.push(RecordBatch::Changes(out));
        }
        let outcome = match conn.write(&WriteContext { stream }, batches) {
            Ok(o) => o,
            Err(e) => {
                return self
                    .report
                    .fail("write", &case.case, format!("the call failed: {e}"));
            }
        };
        drop(conn);

        let mut diffs = Vec::new();
        match (&outcome.result, &case.error) {
            (WriteResult::Ok(receipt), None) => {
                if let Some(want) = case.accepted
                    && receipt.accepted != want
                {
                    diffs.push(format!("accepted {}, expected {want}", receipt.accepted));
                }
            }
            (WriteResult::Ok(receipt), Some(want)) => diffs.push(format!(
                "expected the error {want}, but the write accepted {}",
                receipt.accepted
            )),
            (WriteResult::Err(e), None) => {
                diffs.push(format!("the write failed: {:?}: {}", e.kind, e.message))
            }
            (WriteResult::Err(e), Some(want)) => {
                let got = serde_json::to_value(e).unwrap_or(Value::Null);
                if let Some(d) = value_diff(want, &got) {
                    diffs.push(format!("error differs: {d}"));
                }
            }
        }
        if let Some(want) = &case.dead_letters {
            let got: Vec<Value> = outcome
                .dead_letters
                .iter()
                .map(|d| json!({ "record": parse_text(&d.record), "reason": d.reason }))
                .collect();
            if let Some(d) = dead_letter_diff(want, &got) {
                diffs.push(format!("dead-letters differ: {d}"));
            }
        }
        if !diffs.is_empty() {
            return self.report.fail("write", &case.case, diffs.join("; "));
        }
        if self.steps("verify", &case.case, &case.verify) {
            self.report.pass("write", &case.case);
        }
    }
}

/// The state of the `n`th checkpoint of a read (from 1).
fn checkpoint_at(conn: &dyn Connector, ctx: &ReadContext, n: usize) -> Result<StreamState, String> {
    let mut seen = 0;
    for msg in conn
        .read(ctx)
        .map_err(|e| format!("the first read failed: {e}"))?
    {
        match msg {
            Ok(ReadMessage::Checkpoint(state)) => {
                seen += 1;
                if seen == n {
                    return Ok(state);
                }
            }
            Ok(ReadMessage::Fatal(e)) => {
                return Err(format!(
                    "the first read ended with a fatal error: {}",
                    e.message
                ));
            }
            Ok(_) => {}
            Err(e) => return Err(format!("the first read failed: {e}")),
        }
    }
    Err(format!(
        "the first read ended after {seen} checkpoints, before checkpoint {n}"
    ))
}

/// A `ConfiguredStream` from the file, with `sync_mode`, `write_mode` and
/// `mapping` defaulted.
fn configured_stream(v: &Value) -> Result<ConfiguredStream, String> {
    let mut obj = match v {
        Value::Object(o) => o.clone(),
        other => return Err(format!("stream must be a table, got {other}")),
    };
    obj.entry("sync_mode").or_insert(json!("FullRefresh"));
    obj.entry("write_mode").or_insert(json!("Append"));
    obj.entry("mapping").or_insert(json!({ "ops": [] }));
    serde_json::from_value(Value::Object(obj)).map_err(|e| format!("stream does not parse: {e}"))
}

fn read_context(case: &ReadCase) -> Result<ReadContext, String> {
    let stream = configured_stream(&case.stream)?;
    let partition = match &case.partition {
        None => Partition {
            id: String::new(),
            bounds: None,
        },
        Some(v) => serde_json::from_value(v.clone())
            .map_err(|e| format!("partition does not parse: {e}"))?,
    };
    let state = match &case.state {
        None => StreamState::default(),
        Some(Value::Object(o)) => StreamState {
            cursor: o.get("cursor").map(|c| c.to_string()),
            opaque: match o.get("opaque") {
                None => Vec::new(),
                Some(v) => serde_json::from_value(v.clone())
                    .map_err(|e| format!("state.opaque does not parse: {e}"))?,
            },
        },
        Some(other) => return Err(format!("state must be a table, got {other}")),
    };
    Ok(ReadContext {
        stream,
        partition,
        state,
    })
}

/// A checkpoint as JSON: the cursor parsed, the opaque bytes as a list.
fn state_value(s: &StreamState) -> Value {
    json!({
        "cursor": s.cursor.as_deref().map(parse_text).unwrap_or(Value::Null),
        "opaque": s.opaque,
    })
}

/// JSON text parsed, or the text itself when it is not JSON.
fn parse_text(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string()))
}

/// A row for a batch: a string goes as written, anything else as JSON text.
fn row_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Whether `actual` has every field `expected` names, recursively.
pub fn matches(expected: &Value, actual: &Value) -> bool {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => e
            .iter()
            .all(|(k, ev)| a.get(k).is_some_and(|av| matches(ev, av))),
        (Value::Array(e), Value::Array(a)) => multiset_diff(e, a, true).is_none(),
        (Value::Number(e), Value::Number(a)) => match (e.as_f64(), a.as_f64()) {
            (Some(x), Some(y)) => x == y,
            _ => e == a,
        },
        _ => expected == actual,
    }
}

fn value_diff(expected: &Value, actual: &Value) -> Option<String> {
    (!matches(expected, actual)).then(|| format!("expected {expected}, got {actual}"))
}

/// Match `expected` against `actual` as multisets. With `exact`, `actual` may
/// hold nothing else. Greedy: each expected value takes the first unused actual
/// value it matches.
pub fn multiset_diff(expected: &[Value], actual: &[Value], exact: bool) -> Option<String> {
    let mut used = vec![false; actual.len()];
    let mut missing = Vec::new();
    for e in expected {
        match (0..actual.len()).find(|&i| !used[i] && matches(e, &actual[i])) {
            Some(i) => used[i] = true,
            None => missing.push(e),
        }
    }
    let extra: Vec<&Value> = actual
        .iter()
        .zip(&used)
        .filter(|(_, u)| !**u)
        .map(|(a, _)| a)
        .collect();
    if missing.is_empty() && (!exact || extra.is_empty()) {
        return None;
    }
    let mut parts = Vec::new();
    if !missing.is_empty() {
        parts.push(format!("missing {}", list(&missing)));
    }
    if exact && !extra.is_empty() {
        parts.push(format!("unexpected {}", list(&extra)));
    }
    Some(format!(
        "{} (expected {}, got {})",
        parts.join(", "),
        expected.len(),
        actual.len()
    ))
}

/// Match in order, element by element.
pub fn ordered_diff(expected: &[Value], actual: &[Value]) -> Option<String> {
    for (i, (e, a)) in expected.iter().zip(actual).enumerate() {
        if !matches(e, a) {
            return Some(format!("at {i}: expected {e}, got {a}"));
        }
    }
    if expected.len() != actual.len() {
        return Some(format!(
            "expected {} items, got {}: {}",
            expected.len(),
            actual.len(),
            list(&actual.iter().collect::<Vec<_>>())
        ));
    }
    None
}

/// Dead-letters match on `record`, and on `reason_contains` as a substring
/// of the reason.
fn dead_letter_diff(expected: &[Value], actual: &[Value]) -> Option<String> {
    let mut want = Vec::with_capacity(expected.len());
    let mut contains = Vec::with_capacity(expected.len());
    for e in expected {
        let mut e = e.clone();
        let needle = e
            .as_object_mut()
            .and_then(|o| o.remove("reason_contains"))
            .and_then(|v| v.as_str().map(str::to_string));
        want.push(e);
        contains.push(needle);
    }
    if let Some(d) = multiset_diff(&want, actual, true) {
        return Some(d);
    }
    for (e, needle) in want.iter().zip(&contains) {
        let Some(needle) = needle else { continue };
        let found = actual.iter().any(|a| {
            matches(e, a)
                && a.get("reason")
                    .and_then(|r| r.as_str())
                    .is_some_and(|r| r.contains(needle.as_str()))
        });
        if !found {
            return Some(format!(
                "no dead-letter {e} has a reason containing {needle:?}"
            ));
        }
    }
    None
}

fn list(items: &[&Value]) -> String {
    const SHOWN: usize = 5;
    let mut s: Vec<String> = items.iter().take(SHOWN).map(|v| v.to_string()).collect();
    if items.len() > SHOWN {
        s.push(format!("and {} more", items.len() - SHOWN));
    }
    format!("[{}]", s.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_constructor_contract::sync::{
        ArrowSchemaIpc, Catalog, PartitionScheme, StreamInfo, SyncMode, WriteReceipt,
    };
    use std::cell::RefCell;

    /// A source and destination in memory: the read yields `rows` and one
    /// checkpoint; the write counts rows. `drop_row` breaks the read.
    struct Fake {
        rows: Vec<String>,
        drop_row: bool,
    }

    impl Connector for Fake {
        fn spec(&self) -> Result<ConnectorSpec, String> {
            Ok(ConnectorSpec {
                name: "fake".into(),
                connector_version: "0.1.0".into(),
                contract_version: 1,
                config_schema: "{\"type\":\"object\"}".into(),
                roles: vec![ConnectorRole::Source, ConnectorRole::Destination],
                supported_sync_modes: vec![SyncMode::FullRefresh],
            })
        }
        fn check(&self, config: &Config) -> Result<CheckResult, String> {
            let ok = serde_json::from_str::<Value>(&config.json).is_ok();
            Ok(CheckResult {
                success: ok,
                message: (!ok).then(|| "config is not JSON".to_string()),
            })
        }
        fn discover(&self, _config: &Config) -> Result<DiscoverOutcome, String> {
            Ok(DiscoverOutcome::Catalog(Catalog {
                streams: vec![StreamInfo {
                    name: "fake".into(),
                    namespace: None,
                    schema: ArrowSchemaIpc { ipc: Vec::new() },
                    supported_sync_modes: vec![SyncMode::FullRefresh],
                    source_defined_cursor: false,
                    default_cursor_field: None,
                    source_defined_primary_key: None,
                    partitioning: PartitionScheme::Unpartitioned,
                }],
            }))
        }
        fn read(&self, _ctx: &ReadContext) -> Result<ReadIter<'_>, String> {
            let mut rows = self.rows.clone();
            if self.drop_row {
                rows.pop();
            }
            Ok(Box::new(
                vec![
                    Ok(ReadMessage::Records(RecordBatch::Rows(rows))),
                    Ok(ReadMessage::Checkpoint(StreamState {
                        cursor: Some("\"2\"".into()),
                        opaque: Vec::new(),
                    })),
                ]
                .into_iter(),
            ))
        }
        fn write(
            &self,
            _ctx: &WriteContext,
            batches: Vec<RecordBatch>,
        ) -> Result<WriteOutcome, String> {
            let accepted = batches
                .iter()
                .map(|b| b.row_count_hint().unwrap_or(0) as u64)
                .sum();
            Ok(WriteOutcome {
                state: StreamState::default(),
                diagnostics: Vec::new(),
                dead_letters: Vec::new(),
                result: WriteResult::Ok(WriteReceipt { accepted }),
            })
        }
    }

    struct FakeHost {
        drop_row: bool,
    }

    impl Host for FakeHost {
        fn prepare(&self, _crate_dir: &Path) -> Result<String, String> {
            Ok("fake".into())
        }
        fn load(
            &self,
            _package: &str,
            _member: &str,
            config: &Config,
            _grants: Option<&Value>,
            _credential: Option<&Credential>,
        ) -> Result<Box<dyn Connector>, String> {
            let rows = serde_json::from_str::<Value>(&config.json)
                .ok()
                .and_then(|v| v.get("rows").cloned())
                .and_then(|r| serde_json::from_value::<Vec<Value>>(r).ok())
                .unwrap_or_default()
                .iter()
                .map(row_text)
                .collect();
            Ok(Box::new(Fake {
                rows,
                drop_row: self.drop_row,
            }))
        }
    }

    /// Records each step and answers a query step with its `answer`.
    #[derive(Default)]
    struct FakeService {
        ran: RefCell<Vec<Value>>,
    }

    impl Service for FakeService {
        fn run(&self, step: &Map<String, Value>) -> Result<Vec<Value>, String> {
            self.ran.borrow_mut().push(Value::Object(step.clone()));
            Ok(step
                .get("answer")
                .and_then(|a| a.as_array().cloned())
                .unwrap_or_default())
        }
    }

    const FILE: &str = r#"
connector = "fake"

[source]
config = { rows = ['{"id":1}', '{"id":2,"name":"b"}'] }

[destination]

[[setup]]
sql = "CREATE TABLE t (id int)"

[spec]
name = "fake"
roles = ["Destination", "Source"]

[[check]]
case = "good"
success = true

[[check]]
case = "not json"
config_text = "{not json"
success = false
message_contains = "JSON"

[discover]
streams = [{ name = "fake", supported_sync_modes = ["FullRefresh"] }]

[[read]]
case = "fresh"
stream = { stream = "fake" }
records = [{ id = 2 }, { id = 1 }]
checkpoints = [{ cursor = "2" }]
dead_letters = []

[[write]]
case = "two rows"
stream = { stream = "fake", write_mode = { Upsert = { business_keys = ["id"] } } }
batches = [[{ id = 1 }], [{ id = 2 }]]
accepted = 2
verify = [{ sql = "SELECT id FROM t", answer = [{ id = 1 }, { id = 2 }], rows = [{ id = 2 }, { id = 1 }] }]
"#;

    #[test]
    fn a_conforming_connector_passes_every_case() {
        let service = FakeService::default();
        let report = run_text(
            FILE,
            Path::new("."),
            &FakeHost { drop_row: false },
            Some(&service),
        );
        report.assert_passed();
        assert_eq!(report.passed.len(), 6, "{:?}", report.passed);
        assert_eq!(service.ran.borrow().len(), 2);
    }

    #[test]
    fn a_broken_read_fails_naming_the_method_case_and_difference() {
        let service = FakeService::default();
        let report = run_text(
            FILE,
            Path::new("."),
            &FakeHost { drop_row: true },
            Some(&service),
        );
        assert_eq!(report.failures.len(), 1, "{report}");
        let f = &report.failures[0];
        assert_eq!(f.method, "read");
        assert_eq!(f.case, "fresh");
        assert!(f.difference.contains("records differ"), "{f}");
        assert!(f.difference.contains(r#"missing [{"id":2}]"#), "{f}");
        assert!(report.to_string().contains("read `fresh`"), "{report}");
    }

    #[test]
    fn resume_after_reads_to_a_checkpoint_first() {
        let file = r#"
connector = "fake"
[source]
config = { rows = ['{"id":1}'] }
[[read]]
case = "resume"
stream = { stream = "fake" }
resume_after = 1
records = [{ id = 1 }]
[[read]]
case = "too far"
stream = { stream = "fake" }
resume_after = 3
"#;
        let report = run_text(file, Path::new("."), &FakeHost { drop_row: false }, None);
        assert_eq!(report.failures.len(), 1, "{report}");
        assert_eq!(report.failures[0].case, "too far");
        assert!(
            report.failures[0]
                .difference
                .contains("ended after 1 checkpoints"),
            "{report}"
        );
    }

    #[test]
    fn steps_without_a_service_fail_clearly() {
        let report = run_text(FILE, Path::new("."), &FakeHost { drop_row: false }, None);
        assert_eq!(report.failures.len(), 1, "{report}");
        assert_eq!(report.failures[0].method, "setup");
    }

    #[test]
    fn a_bad_file_is_a_file_failure() {
        let report = run_text(
            "connector = 1",
            Path::new("."),
            &FakeHost { drop_row: false },
            None,
        );
        assert_eq!(report.failures[0].method, "file");
    }

    #[test]
    fn matching_is_by_named_fields_and_as_multisets() {
        assert!(matches(&json!({"a": 1}), &json!({"a": 1.0, "b": 2})));
        assert!(!matches(&json!({"a": 1, "c": 3}), &json!({"a": 1})));
        assert!(matches(&json!([1, 2]), &json!([2, 1])));
        assert!(!matches(&json!([1, 2]), &json!([1, 2, 3])));
        assert!(multiset_diff(&[json!(1)], &[json!(1), json!(2)], false).is_none());
        assert!(ordered_diff(&[json!(1), json!(2)], &[json!(2), json!(1)]).is_some());
    }

    /// Hands out the fake, and checks pass only with the `good` token.
    struct CredentialHost;

    impl Host for CredentialHost {
        fn prepare(&self, _crate_dir: &Path) -> Result<String, String> {
            Ok("fake".into())
        }
        fn load(
            &self,
            _package: &str,
            _member: &str,
            _config: &Config,
            _grants: Option<&Value>,
            credential: Option<&Credential>,
        ) -> Result<Box<dyn Connector>, String> {
            let c = credential.ok_or("no credential")?;
            assert_eq!(c.spec["kind"], json!("header"));
            Ok(Box::new(TokenChecked(
                c.secret.get("token").cloned().unwrap_or_default(),
            )))
        }
    }

    struct TokenChecked(String);

    impl Connector for TokenChecked {
        fn spec(&self) -> Result<ConnectorSpec, String> {
            Fake {
                rows: Vec::new(),
                drop_row: false,
            }
            .spec()
        }
        fn check(&self, _config: &Config) -> Result<CheckResult, String> {
            let ok = self.0 == "good";
            Ok(CheckResult {
                success: ok,
                message: (!ok).then(|| "403".to_string()),
            })
        }
        fn discover(&self, _config: &Config) -> Result<DiscoverOutcome, String> {
            Err("unused".into())
        }
        fn read(&self, _ctx: &ReadContext) -> Result<ReadIter<'_>, String> {
            Err("unused".into())
        }
        fn write(
            &self,
            _ctx: &WriteContext,
            _batches: Vec<RecordBatch>,
        ) -> Result<WriteOutcome, String> {
            Err("unused".into())
        }
    }

    const CREDENTIAL_FILE: &str = r#"
connector = "fake"

[source]
credential = { kind = "header", secret = "api", name = "Authorization", field = "token" }

[secrets.api]
token = "good"

[secrets.wrong]
token = "bad"

[[check]]
case = "the named secret"
success = true

[[check]]
case = "another secret"
secret = "wrong"
success = false
message_contains = "403"
"#;

    #[test]
    fn a_credential_reaches_the_host_with_its_secret() {
        let report = run_text(CREDENTIAL_FILE, Path::new("."), &CredentialHost, None);
        report.assert_passed();
        assert_eq!(report.passed.len(), 3, "{:?}", report.passed);
    }

    #[test]
    fn a_missing_secret_is_a_file_failure() {
        let text = CREDENTIAL_FILE.replace("secret = \"wrong\"", "secret = \"absent\"");
        let report = run_text(&text, Path::new("."), &CredentialHost, None);
        assert_eq!(report.failures.len(), 1, "{report}");
        assert_eq!(report.failures[0].method, "file");
        assert!(report.failures[0].difference.contains("absent"), "{report}");
        let shown = format!(
            "{:?}",
            Credential {
                spec: json!({}),
                secret: BTreeMap::from([("token".to_string(), "s3cr3t".to_string())]),
            }
        );
        assert!(!shown.contains("s3cr3t"), "{shown}");
    }

    #[test]
    fn interpolation_reads_the_environment_with_defaults() {
        // Names no test sets, so the defaults apply.
        let out = interpolate("a=${CLOACINA_CONFORMANCE_UNSET_X:-dflt} b=$c").unwrap();
        assert_eq!(out, "a=dflt b=$c");
        assert!(interpolate("${CLOACINA_CONFORMANCE_UNSET_Y}").is_err());
        let vars = BTreeMap::from([("MOCK_PORT".to_string(), "4321".to_string())]);
        assert_eq!(
            interpolate_with("port=${MOCK_PORT:-1}", &vars).unwrap(),
            "port=4321"
        );
        let path = std::env::var("PATH").unwrap();
        assert_eq!(interpolate("${PATH}").unwrap(), path);
    }
}
