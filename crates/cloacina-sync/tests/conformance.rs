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

//! The connector conformance kit through the real connector host
//! (CLOACINA-A-0014, CLOACINA-T-0983): each connector's conformance file, run
//! against the built component.
//!
//! The echo runs need only the `wasm32-wasip2` target. The Postgres run is
//! ignored by default: run it with `-- --ignored` once `angreal services up`
//! has the database (or set `CLOACINA_TEST_PG_URL`). A `CARGO_TARGET_DIR` in
//! the environment moves the guests' artifacts away from the path the kit
//! reads; the build runs with it unset.

use std::path::Path;

use cloacina_constructor_contract::sync::{
    CheckResult, Config, ConnectorSpec, DiscoverOutcome, ReadContext, ReadMessage, RecordBatch,
    WriteContext, WriteOutcome,
};
use cloacina_testing::connectors::conformance::{self, Connector, Host, ReadIter, Report, Service};
use serde_json::{Map, Value};

mod support;
use support::{ConnectorHost, repo};

#[test]
fn echo_conforms() {
    let report = conformance::run(
        &repo("examples/constructor-contract/connector-echo-fixture/conformance.toml"),
        &ConnectorHost::new("echo"),
        None,
    );
    report.assert_passed();
    println!("{report}");
}

/// A broken echo: the real component through the real host, with the last
/// row of every batch it reads dropped.
struct BrokenHost(ConnectorHost);

struct Broken(Box<dyn Connector>);

impl Host for BrokenHost {
    fn prepare(&self, crate_dir: &Path) -> Result<String, String> {
        self.0.prepare(crate_dir)
    }

    fn load(
        &self,
        package: &str,
        member: &str,
        config: &Config,
        grants: Option<&Value>,
    ) -> Result<Box<dyn Connector>, String> {
        Ok(Box::new(Broken(
            self.0.load(package, member, config, grants)?,
        )))
    }
}

impl Connector for Broken {
    fn spec(&self) -> Result<ConnectorSpec, String> {
        self.0.spec()
    }

    fn check(&self, config: &Config) -> Result<CheckResult, String> {
        self.0.check(config)
    }

    fn discover(&self, config: &Config) -> Result<DiscoverOutcome, String> {
        self.0.discover(config)
    }

    fn read(&self, ctx: &ReadContext) -> Result<ReadIter<'_>, String> {
        Ok(Box::new(self.0.read(ctx)?.map(|m| match m {
            Ok(ReadMessage::Records(RecordBatch::Rows(mut rows))) => {
                rows.pop();
                Ok(ReadMessage::Records(RecordBatch::Rows(rows)))
            }
            other => other,
        })))
    }

    fn write(&self, ctx: &WriteContext, batches: Vec<RecordBatch>) -> Result<WriteOutcome, String> {
        self.0.write(ctx, batches)
    }
}

#[test]
fn a_broken_echo_fails_naming_the_method_and_the_difference() {
    let report: Report = conformance::run(
        &repo("examples/constructor-contract/connector-echo-fixture/conformance.toml"),
        &BrokenHost(ConnectorHost::new("broken")),
        None,
    );
    assert!(!report.is_pass(), "the broken echo passed: {report}");
    assert!(
        report.failures.iter().all(|f| f.method == "read"),
        "only the reads are broken: {report}"
    );
    let fresh = report
        .failures
        .iter()
        .find(|f| f.case == "fresh read")
        .unwrap_or_else(|| panic!("the fresh read did not fail: {report}"));
    assert!(
        fresh
            .difference
            .contains(r#"records differ: missing [{"id":2,"name":"b"}]"#),
        "{fresh}"
    );
    let shown = report.to_string();
    assert!(
        shown.contains("read `fresh read`: records differ"),
        "{shown}"
    );
    assert!(report.passed.iter().any(|p| p == "write two batches"));
}

/// Setup and verify steps against Postgres: `sql` runs statements, `query`
/// returns rows as JSON objects.
struct Postgres {
    url: String,
}

impl Service for Postgres {
    fn run(&self, step: &Map<String, Value>) -> Result<Vec<Value>, String> {
        let mut client =
            postgres::Client::connect(&self.url, postgres::NoTls).map_err(|e| e.to_string())?;
        if let Some(sql) = step.get("sql").and_then(Value::as_str) {
            client.batch_execute(sql).map_err(|e| e.to_string())?;
            return Ok(Vec::new());
        }
        if let Some(query) = step.get("query").and_then(Value::as_str) {
            let rows = client
                .query(
                    &format!("SELECT row_to_json(q)::text FROM ({query}) q"),
                    &[],
                )
                .map_err(|e| e.to_string())?;
            return rows
                .iter()
                .map(|r| serde_json::from_str(r.get::<_, &str>(0)).map_err(|e| e.to_string()))
                .collect();
        }
        Err("a Postgres step needs `sql` or `query`".into())
    }
}

#[test]
#[ignore = "needs the dev Postgres on 15432 (angreal services up) and the wasm32-wasip2 target"]
fn postgres_conforms() {
    let url = std::env::var("CLOACINA_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://cloacina:cloacina@localhost:15432/cloacina".to_string());
    let report = conformance::run(
        &repo("connectors/postgres/conformance.toml"),
        &ConnectorHost::new("postgres"),
        Some(&Postgres { url }),
    );
    report.assert_passed();
    println!("{report}");
}
