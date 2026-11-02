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

//! The SQL Server the MSSQL tests run against (CLOACINA-T-0984): the `mssql`
//! service of `.angreal/docker-compose.yaml` on 11433, or
//! `CLOACINA_TEST_MSSQL_HOST` and `CLOACINA_TEST_MSSQL_PORT`. Seeding runs
//! through tiberius on the host, unencrypted.

use super::{Map, Service, Value};
use tokio_util::compat::TokioAsyncWriteCompatExt;

/// The `sa` password of the test service; it is a throwaway test container.
pub const PASSWORD: &str = "cloacinaStrong!Pass1";

pub fn host() -> String {
    std::env::var("CLOACINA_TEST_MSSQL_HOST").unwrap_or_else(|_| "localhost".to_string())
}

pub fn port() -> u16 {
    std::env::var("CLOACINA_TEST_MSSQL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(11433)
}

/// Run one T-SQL batch in `database`. Blocking: call it off an async runtime.
pub fn batch(database: &str, sql: &str) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async {
        let mut cfg = tiberius::Config::new();
        cfg.host(host());
        cfg.port(port());
        cfg.database(database);
        cfg.authentication(tiberius::AuthMethod::sql_server("sa", PASSWORD));
        cfg.encryption(tiberius::EncryptionLevel::NotSupported);
        let tcp = tokio::net::TcpStream::connect(cfg.get_addr())
            .await
            .map_err(|e| format!("connect {}:{}: {e}", host(), port()))?;
        tcp.set_nodelay(true).map_err(|e| e.to_string())?;
        let mut client = tiberius::Client::connect(cfg, tcp.compat_write())
            .await
            .map_err(|e| e.to_string())?;
        client
            .simple_query(sql)
            .await
            .map_err(|e| e.to_string())?
            .into_results()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

/// The conformance kit's steps against SQL Server: `sql` runs a batch in
/// `database` (default `master`).
pub struct Mssql;

impl Service for Mssql {
    fn run(&self, step: &Map<String, Value>) -> Result<Vec<Value>, String> {
        let database = step
            .get("database")
            .and_then(Value::as_str)
            .unwrap_or("master");
        match step.get("sql").and_then(Value::as_str) {
            Some(sql) => batch(database, sql).map(|()| Vec::new()),
            None => Err("an MSSQL step needs `sql`".into()),
        }
    }
}
