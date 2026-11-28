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

//! `key create --tenant <name>`: a key of that tenant, made through
//! `POST /v1/tenants/{tenant_id}/keys` (CLOACINA-T-1140).

use crate::OutputFormat;
use crate::shared::client::CliClient;
use crate::shared::error::CliError;
use crate::shared::render;

/// The route that makes a key of `tenant`.
pub(super) fn tenant_keys_path(tenant: &str) -> String {
    format!("/v1/tenants/{tenant}/keys")
}

/// Make the key `name` with `role` in `tenant`, and show it once.
pub(super) async fn create(
    client: &CliClient,
    tenant: &str,
    name: &str,
    role: &str,
    output: OutputFormat,
) -> Result<(), CliError> {
    let body = serde_json::json!({ "name": name, "role": role });
    let resp: serde_json::Value = client.post(&tenant_keys_path(tenant), &body).await?;
    if matches!(output, OutputFormat::Table) {
        let id = resp.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let plaintext = resp.get("key").and_then(|v| v.as_str()).unwrap_or("<none>");
        let role = resp
            .get("permissions")
            .and_then(|v| v.as_str())
            .unwrap_or(role);
        println!("created key: {plaintext}");
        println!("ID:          {id}");
        println!("tenant:      {tenant}");
        println!("role:        {role}");
        println!("NOTE: this is the only time the secret will be shown. Save it now.");
        Ok(())
    } else {
        render::object(&resp, output)
    }
}

#[cfg(test)]
mod tests {
    use super::super::KeyCmd;
    use crate::GlobalOpts;
    use clap::Parser;
    use std::io::{BufRead, BufReader, Read, Write};

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        key: KeyCmd,
    }

    /// A server that answers one request with a created key and gives back
    /// the request line and body.
    fn one_request_server() -> (String, std::thread::JoinHandle<(String, String)>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            let mut length = 0usize;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header == "\r\n" || header.is_empty() {
                    break;
                }
                if let Some((key, value)) = header.split_once(':')
                    && key.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut body = vec![0u8; length];
            reader.read_exact(&mut body).unwrap();
            let answer = r#"{"id":"00000000-0000-0000-0000-000000000001","key":"clk_test","name":"acme-ci","permissions":"write"}"#;
            let mut stream = stream;
            write!(
                stream,
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                answer.len(),
                answer
            )
            .unwrap();
            (
                request_line.trim_end().to_string(),
                String::from_utf8(body).unwrap(),
            )
        });
        (url, handle)
    }

    fn globals(server: &str, tenant: Option<&str>) -> GlobalOpts {
        GlobalOpts {
            verbose: false,
            home: std::path::PathBuf::from("/nonexistent-cloacinactl-home"),
            profile: None,
            server: Some(server.to_string()),
            api_key: Some("clk_admin".to_string()),
            tenant: tenant.map(str::to_string),
            json: true,
            output: None,
            no_color: false,
        }
    }

    async fn request_of(args: &[&str], tenant: Option<&str>) -> (String, serde_json::Value) {
        let (url, server) = one_request_server();
        let cli = Cli::try_parse_from(args).unwrap();
        cli.key.run(&globals(&url, tenant)).await.unwrap();
        let (line, body) = server.join().unwrap();
        (line, serde_json::from_str(&body).unwrap())
    }

    /// CLOACINA-T-1140: the command of step 3 of the multi-tenant how-to
    /// makes a key of the tenant it names, through the tenant's route.
    #[tokio::test]
    async fn key_create_with_tenant_uses_the_tenant_route() {
        let (line, body) = request_of(
            &["key", "create", "acme-ci", "--role", "write"],
            Some("acme"),
        )
        .await;
        assert_eq!(line, "POST /v1/tenants/acme/keys HTTP/1.1");
        assert_eq!(
            body,
            serde_json::json!({ "name": "acme-ci", "role": "write" })
        );
    }

    /// Without `--tenant` the command is as before.
    #[tokio::test]
    async fn key_create_without_tenant_uses_the_global_route() {
        let (line, body) = request_of(&["key", "create", "ci-bot"], None).await;
        assert_eq!(line, "POST /v1/auth/keys HTTP/1.1");
        assert_eq!(
            body,
            serde_json::json!({ "name": "ci-bot", "role": "read" })
        );
    }
}
