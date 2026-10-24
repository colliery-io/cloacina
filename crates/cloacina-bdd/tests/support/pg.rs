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

//! The scenario's Postgres: tables a step creates, reads and drops.

use tokio_postgres::{Client, NoTls};

/// `CLOACINA_BDD_PG_URL`, or the dev Postgres on 15432.
pub fn url() -> String {
    std::env::var("CLOACINA_BDD_PG_URL")
        .unwrap_or_else(|_| "postgres://cloacina:cloacina@localhost:15432/cloacina".into())
}

/// The URL a connector is given: TLS off, as the dev and CI Postgres have none.
pub fn connector_url() -> String {
    format!("{}?sslmode=disable", url())
}

/// The port of [`url`], for the connector's network grant.
pub fn port() -> u16 {
    let url = url();
    url.rsplit_once(':')
        .and_then(|(_, t)| t.split('/').next())
        .and_then(|p| p.parse().ok())
        .unwrap_or(5432)
}

pub async fn client() -> Client {
    let (client, connection) = tokio_postgres::connect(&url(), NoTls)
        .await
        .unwrap_or_else(|e| panic!("connect to the scenario Postgres at {}: {e}", url()));
    tokio::spawn(connection);
    client
}

fn ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// A column holds integers when every value in it parses as one.
fn is_integer(rows: &[Vec<String>], col: usize) -> bool {
    rows.iter().all(|r| r[col].parse::<i64>().is_ok())
}

/// Create `table` from a Gherkin table: the header names the columns, the first
/// is the primary key, a column of integers is BIGINT and any other TEXT.
pub async fn create_table(table: &str, data: &[Vec<String>]) {
    let (header, rows) = data.split_first().expect("a table with a header row");
    let columns: Vec<String> = header
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let ty = if is_integer(rows, i) {
                "BIGINT"
            } else {
                "TEXT"
            };
            format!("{} {ty}", ident(name))
        })
        .collect();
    let sql = format!(
        "DROP TABLE IF EXISTS {t}; CREATE TABLE {t} ({}, PRIMARY KEY ({}))",
        columns.join(", "),
        ident(&header[0]),
        t = ident(table),
    );
    client()
        .await
        .batch_execute(&sql)
        .await
        .expect("create the table");
    insert(table, data).await;
}

/// Insert the rows of a Gherkin table (header first) into `table`.
pub async fn insert(table: &str, data: &[Vec<String>]) {
    let (header, rows) = data.split_first().expect("a table with a header row");
    if rows.is_empty() {
        return;
    }
    let columns: Vec<String> = header.iter().map(|c| ident(c)).collect();
    let values: Vec<String> = rows
        .iter()
        .map(|r| {
            format!(
                "({})",
                r.iter().map(|v| literal(v)).collect::<Vec<_>>().join(", ")
            )
        })
        .collect();
    let sql = format!(
        "INSERT INTO {} ({}) VALUES {}",
        ident(table),
        columns.join(", "),
        values.join(", ")
    );
    client()
        .await
        .batch_execute(&sql)
        .await
        .expect("insert the rows");
}

/// The given columns of every row of `table` as text, ordered by the first.
pub async fn rows(table: &str, columns: &[String]) -> Vec<Vec<String>> {
    let select: Vec<String> = columns
        .iter()
        .map(|c| format!("COALESCE({}::text, '')", ident(c)))
        .collect();
    let sql = format!(
        "SELECT {} FROM {} ORDER BY {}",
        select.join(", "),
        ident(table),
        ident(&columns[0])
    );
    client()
        .await
        .query(&sql, &[])
        .await
        .unwrap_or_else(|e| panic!("read {table}: {e}"))
        .iter()
        .map(|row| {
            (0..columns.len())
                .map(|i| row.get::<_, String>(i))
                .collect()
        })
        .collect()
}

pub async fn drop_tables(tables: &[String]) {
    if tables.is_empty() {
        return;
    }
    let list: Vec<String> = tables.iter().map(|t| ident(t)).collect();
    let sql = format!("DROP TABLE IF EXISTS {}", list.join(", "));
    let _ = client().await.batch_execute(&sql).await;
}

pub async fn drop_schema(schema: &str) {
    let sql = format!("DROP SCHEMA IF EXISTS {} CASCADE", ident(schema));
    let _ = client().await.batch_execute(&sql).await;
}

/// The number of rows of `table`.
pub async fn count(table: &str) -> i64 {
    client()
        .await
        .query_one(&format!("SELECT count(*) FROM {}", ident(table)), &[])
        .await
        .unwrap_or_else(|e| panic!("count {table}: {e}"))
        .get(0)
}

/// The number of rows of `table`, or 0 while it does not exist yet.
pub async fn count_if_exists(table: &str) -> i64 {
    let client = client().await;
    let exists: bool = client
        .query_one("SELECT to_regclass($1) IS NOT NULL", &[&ident(table)])
        .await
        .expect("look up the table")
        .get(0);
    if !exists {
        return 0;
    }
    client
        .query_one(&format!("SELECT count(*) FROM {}", ident(table)), &[])
        .await
        .map(|r| r.get(0))
        .unwrap_or(0)
}
