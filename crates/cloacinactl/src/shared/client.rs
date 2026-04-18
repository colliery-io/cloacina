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

//! HTTP client wrapper that injects auth + tenant headers, maps HTTP status to
//! CliError, and caches `/v1/keys/self` (whoami) for the tenant-resolution
//! rule from ADR-0003 §4.

use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use reqwest::{Method, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;

/// Scope of the caller's API key as reported by `GET /v1/keys/self`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum KeyScope {
    /// Admin key — can act on any tenant when one is named.
    Admin,
    /// Tenant-scoped key bound to a single tenant.
    Tenant { tenant: String },
}

/// What `whoami` returns.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WhoAmI {
    #[serde(flatten)]
    pub scope: KeyScope,
    #[serde(default)]
    pub role: Option<String>,
}

/// Shared HTTP client used by every verb handler.
pub struct CliClient {
    ctx: ClientContext,
    http: reqwest::Client,
    whoami_cache: OnceLock<WhoAmI>,
}

impl CliClient {
    pub fn new(ctx: ClientContext) -> Result<Arc<Self>, CliError> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(CliError::from_reqwest)?;
        Ok(Arc::new(Self {
            ctx,
            http,
            whoami_cache: OnceLock::new(),
        }))
    }

    pub fn ctx(&self) -> &ClientContext {
        &self.ctx
    }

    fn url(&self, path: &str) -> String {
        let base = self.ctx.server.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{base}/{path}")
    }

    fn apply_auth(
        &self,
        req: reqwest::RequestBuilder,
        tenant: Option<&str>,
    ) -> reqwest::RequestBuilder {
        let mut req = req.bearer_auth(&self.ctx.api_key);
        if let Some(t) = tenant {
            req = req.header("X-Tenant", t);
        }
        req
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
        let tenant = self.ctx.tenant.clone();
        let req = self.apply_auth(
            self.http.request(Method::GET, self.url(path)),
            tenant.as_deref(),
        );
        Self::parse_response(self.send(req).await?).await
    }

    /// Typed POST (JSON body).
    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, CliError> {
        let tenant = self.ctx.tenant.clone();
        let req = self
            .apply_auth(
                self.http.request(Method::POST, self.url(path)),
                tenant.as_deref(),
            )
            .json(body);
        Self::parse_response(self.send(req).await?).await
    }

    /// DELETE without a response body.
    pub async fn delete(&self, path: &str) -> Result<(), CliError> {
        let tenant = self.ctx.tenant.clone();
        let req = self.apply_auth(
            self.http.request(Method::DELETE, self.url(path)),
            tenant.as_deref(),
        );
        let response = self.send(req).await?;
        let status = response.status().as_u16();
        if response.status().is_success() {
            return Ok(());
        }
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        Err(CliError::from_status(status, body))
    }

    /// Cache-aware `GET /v1/keys/self`.
    pub async fn whoami(&self) -> Result<&WhoAmI, CliError> {
        if let Some(w) = self.whoami_cache.get() {
            return Ok(w);
        }
        let w: WhoAmI = self.get("/v1/keys/self").await?;
        // First writer wins — OnceLock is fine for `&` borrow.
        let _ = self.whoami_cache.set(w);
        Ok(self.whoami_cache.get().unwrap())
    }

    /// Resolve the tenant to use for the current command per ADR §4. Returns
    /// the tenant name a caller should thread through, or errors with an exit
    /// code-appropriate CliError.
    ///
    /// `tenant_scoped_command` indicates whether the command operates on a
    /// tenant-scoped resource.
    pub async fn require_tenant(
        &self,
        tenant_scoped_command: bool,
    ) -> Result<Option<String>, CliError> {
        if !tenant_scoped_command {
            return Ok(None);
        }
        let who = self.whoami().await?;
        match (&who.scope, self.ctx.tenant.as_deref()) {
            (KeyScope::Tenant { tenant }, None) => Ok(Some(tenant.clone())),
            (KeyScope::Tenant { tenant }, Some(requested)) if tenant != requested => {
                Err(CliError::UserError(format!(
                    "key is scoped to tenant '{tenant}', cannot target '{requested}'"
                )))
            }
            (KeyScope::Tenant { tenant }, Some(_)) => Ok(Some(tenant.clone())),
            (KeyScope::Admin, None) => Err(CliError::UserError(
                "admin key requires --tenant for this command".into(),
            )),
            (KeyScope::Admin, Some(t)) => Ok(Some(t.to_string())),
        }
    }
}
