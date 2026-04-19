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

//! HTTP client wrapper that injects auth, maps HTTP status to CliError, and
//! exposes a `ClientContext` for tenant/path resolution at each call site.

use std::sync::Arc;
use std::time::Duration;

use reqwest::{Method, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;

/// Shared HTTP client used by every verb handler.
pub struct CliClient {
    ctx: ClientContext,
    http: reqwest::Client,
}

/// Prompt the user for destructive-op confirmation unless stdin isn't a TTY
/// (in which case CI scripts are running and should pass --force explicitly).
pub fn confirm_destructive(action: &str) -> Result<(), CliError> {
    use std::io::{self, BufRead, IsTerminal, Write};
    if !io::stdin().is_terminal() {
        return Err(CliError::UserError(format!(
            "refusing to {action} without --force (stdin is not a TTY)"
        )));
    }
    print!("{action}? [y/N] ");
    io::stdout().flush().ok();
    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(CliError::Io)?;
    if line.trim().eq_ignore_ascii_case("y") {
        Ok(())
    } else {
        Err(CliError::UserError("cancelled".into()))
    }
}

impl CliClient {
    pub fn new(ctx: ClientContext) -> Result<Arc<Self>, CliError> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(CliError::from_reqwest)?;
        Ok(Arc::new(Self { ctx, http }))
    }

    pub fn ctx(&self) -> &ClientContext {
        &self.ctx
    }

    fn url(&self, path: &str) -> String {
        let base = self.ctx.server.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{base}/{path}")
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        // Tenant is part of the URL path (`/tenants/{tenant}/...`), not a
        // header — auth is just the bearer token.
        req.bearer_auth(&self.ctx.api_key)
    }

    async fn send(&self, req: reqwest::RequestBuilder) -> Result<Response, CliError> {
        let response = req.send().await.map_err(CliError::from_reqwest)?;
        Ok(response)
    }

    async fn parse_response<T: DeserializeOwned>(response: Response) -> Result<T, CliError> {
        let status = response.status().as_u16();
        if response.status().is_success() {
            return response.json::<T>().await.map_err(CliError::from_reqwest);
        }
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        Err(CliError::from_status(status, body))
    }

    /// Typed GET.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, CliError> {
        let req = self.apply_auth(self.http.request(Method::GET, self.url(path)));
        Self::parse_response(self.send(req).await?).await
    }

    /// Typed POST (JSON body).
    pub async fn post<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, CliError> {
        let req = self
            .apply_auth(self.http.request(Method::POST, self.url(path)))
            .json(body);
        Self::parse_response(self.send(req).await?).await
    }

    /// DELETE without a response body.
    pub async fn delete(&self, path: &str) -> Result<(), CliError> {
        let req = self.apply_auth(self.http.request(Method::DELETE, self.url(path)));
        let response = self.send(req).await?;
        let status = response.status().as_u16();
        if response.status().is_success() {
            return Ok(());
        }
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        Err(CliError::from_status(status, body))
    }
}
