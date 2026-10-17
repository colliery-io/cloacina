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

//! The MinIO the S3 tests run against (CLOACINA-T-0985): the `minio` service
//! of `.angreal/docker-compose.yaml` on 19000, or `CLOACINA_TEST_MINIO_HOST`
//! and `CLOACINA_TEST_MINIO_PORT`. Seeding signs its PUTs with cloacina's
//! SigV4 signer over a plain HTTP/1.1 socket.

use std::io::{Read, Write};
use std::net::TcpStream;

use cloacina::registry::loader::credentials::sigv4::{SigV4, sha256_hex};

use super::{Map, Service, Value};

/// The test service's root user; a throwaway test container.
pub const ACCESS_KEY: &str = "minioadmin";
pub const SECRET_KEY: &str = "minioadmin";
pub const REGION: &str = "us-east-1";

pub fn host() -> String {
    std::env::var("CLOACINA_TEST_MINIO_HOST").unwrap_or_else(|_| "localhost".to_string())
}

pub fn port() -> u16 {
    std::env::var("CLOACINA_TEST_MINIO_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(19000)
}

pub fn endpoint() -> String {
    format!("http://{}:{}", host(), port())
}

/// A signed PUT of `body` to `path` (`/bucket` or `/bucket/key`). Returns the
/// status code.
fn put(path: &str, body: &[u8]) -> Result<u16, String> {
    let now = chrono::Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let datestamp = now.format("%Y%m%d").to_string();
    let authority = format!("{}:{}", host(), port());
    let payload_hash = sha256_hex(body);
    let signed = vec![
        ("host".to_string(), authority.clone()),
        ("x-amz-content-sha256".to_string(), payload_hash.clone()),
        ("x-amz-date".to_string(), amz_date.clone()),
    ];
    let auth = SigV4 {
        access_key: ACCESS_KEY,
        secret_key: SECRET_KEY,
        region: REGION,
        service: "s3",
    }
    .authorization("PUT", path, "", &signed, &payload_hash, &amz_date, &datestamp);

    let mut stream = TcpStream::connect(&authority).map_err(|e| format!("connect {authority}: {e}"))?;
    let head = format!(
        "PUT {path} HTTP/1.1\r\nHost: {authority}\r\nx-amz-date: {amz_date}\r\n\
         x-amz-content-sha256: {payload_hash}\r\nAuthorization: {auth}\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(body))
        .map_err(|e| e.to_string())?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| e.to_string())?;
    let status = response
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| format!("no HTTP status in {response:?}"))?;
    if (200..300).contains(&status) {
        Ok(status)
    } else {
        Err(format!("PUT {path}: HTTP {status}: {response}"))
    }
}

/// Create `bucket`; an existing bucket is fine.
pub fn create_bucket(bucket: &str) -> Result<(), String> {
    match put(&format!("/{bucket}"), b"") {
        Ok(_) => Ok(()),
        Err(e) if e.contains("BucketAlreadyOwnedByYou") => Ok(()),
        Err(e) => Err(e),
    }
}

/// Put an object.
pub fn put_object(bucket: &str, key: &str, body: &str) -> Result<(), String> {
    put(&format!("/{bucket}/{key}"), body.as_bytes()).map(|_| ())
}

/// The conformance kit's steps against MinIO: `bucket` creates it; with `key`
/// and `body`, the step puts that object.
pub struct Minio;

impl Service for Minio {
    fn run(&self, step: &Map<String, Value>) -> Result<Vec<Value>, String> {
        let bucket = step
            .get("bucket")
            .and_then(Value::as_str)
            .ok_or("a MinIO step needs `bucket`")?;
        match (
            step.get("key").and_then(Value::as_str),
            step.get("body").and_then(Value::as_str),
        ) {
            (None, _) => create_bucket(bucket)?,
            (Some(key), Some(body)) => put_object(bucket, key, body)?,
            (Some(_), None) => return Err("a MinIO object step needs `body`".into()),
        }
        Ok(Vec::new())
    }
}
