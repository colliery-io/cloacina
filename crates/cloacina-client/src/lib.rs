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

//! Rust client for `cloacina-server` (CLOACI-I-0113 / T-0646).
//!
//! Extracted from `cloacinactl`'s crate-private client so external services
//! consume the same surface the CLI does. DTOs come from
//! [`cloacina-api-types`] — the same crate the server's handlers build their
//! responses from, so request/response shapes cannot drift.
//!
//! ```no_run
//! # async fn demo() -> Result<(), cloacina_client::ClientError> {
//! use cloacina_client::ClientBuilder;
//!
//! let client = ClientBuilder::new("http://localhost:8080")
//!     .api_key("clk_...")
//!     .tenant("public")
//!     .build()?;
//!
//! let accepted = client
//!     .execute_workflow("my_workflow", serde_json::json!({"input": 42}))
//!     .await?;
//!
//! use futures_util::StreamExt;
//! let mut events = std::pin::pin!(client.follow_execution_events(&accepted.execution_id));
//! while let Some(event) = events.next().await {
//!     println!("{:?}", event?);
//! }
//! # Ok(())
//! # }
//! ```

mod error;
mod profile;
mod ws;

pub use cloacina_api_types as types;
pub use error::ClientError;
pub use profile::resolve_api_key_scheme;
pub use ws::{DeliveryPush, SubscribeOptions, DELIVERY_PROTOCOL_VERSION};

use std::sync::Arc;
use std::time::Duration;

use reqwest::{Method, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;

use cloacina_api_types::{
    AccumulatorStatus, AgentInfo, CompilerStatus, CreateKeyRequest, CreateTenantRequest,
    DeclaredSurface, ExecuteRequest, ExecuteResponse, ExecutionDetail, ExecutionEventsResponse,
    ExecutionSummary, ExecutionTasksResponse, FireReactorRequest, FireReactorResponse,
    FireTriggerRequest, FireTriggerResponse, GraphStatus,
    InjectAccumulatorRequest, InjectAccumulatorResponse, KeyCreatedResponse, KeyInfo,
    KeyRevokedResponse, KeyRole, ListResponse, ReactorFire, ReactorFireTimeseries, ReactorStatus,
    TenantCreatedResponse, TenantListResponse, TenantRemovedResponse, TenantSummary,
    TriggerDetailResponse, TriggerPauseResponse, TriggerScheduleSummary, WorkflowDeletedResponse,
    WorkflowDetail, WorkflowPauseResponse, WorkflowSourceResponse, WorkflowSummary,
    WorkflowUploadedResponse, WsTicketResponse,
};

/// Builder for [`Client`].
#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    server: String,
    api_key: Option<String>,
    tenant: Option<String>,
    connect_timeout: Option<Duration>,
    timeout: Option<Duration>,
}

impl ClientBuilder {
    /// Start a builder for the given server base URL
    /// (e.g. `http://localhost:8080`).
    pub fn new(server: impl Into<String>) -> Self {
        Self {
            server: server.into(),
            ..Default::default()
        }
    }

    /// Build from a `cloacinactl` profile in `~/.cloacina/config.toml`
    /// (or `home`/config.toml when `home` is given). Resolves `env:` and
    /// `file:` API-key schemes exactly like the CLI.
    pub fn from_cloacinactl_profile(
        home: Option<&std::path::Path>,
        profile: Option<&str>,
    ) -> Result<Self, ClientError> {
        profile::builder_from_profile(home, profile)
    }

    /// API key, sent as `Authorization: Bearer <key>` on every request.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Default tenant for tenant-scoped calls (defaults to `public` —
    /// the admin-schema tenant the server treats specially).
    pub fn tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = Some(tenant.into());
        self
    }

    /// Connect timeout (default 5s).
    pub fn connect_timeout(mut self, d: Duration) -> Self {
        self.connect_timeout = Some(d);
        self
    }

    /// Overall request timeout (default 30s).
    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout = Some(d);
        self
    }

    pub fn build(self) -> Result<Client, ClientError> {
        let api_key = self
            .api_key
            .ok_or_else(|| ClientError::Config("no API key configured".into()))?;
        let http = reqwest::Client::builder()
            .connect_timeout(self.connect_timeout.unwrap_or(Duration::from_secs(5)))
            .timeout(self.timeout.unwrap_or(Duration::from_secs(30)))
            .build()
            .map_err(ClientError::from_reqwest)?;
        Ok(Client {
            inner: Arc::new(ClientInner {
                server: self.server.trim_end_matches('/').to_string(),
                api_key,
                tenant: self.tenant,
                http,
            }),
        })
    }
}

struct ClientInner {
    server: String,
    api_key: String,
    tenant: Option<String>,
    http: reqwest::Client,
}

/// Typed client for the cloacina-server REST API + delivery WebSocket.
/// Cheap to clone (everything behind one `Arc`).
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

impl Client {
    /// Server base URL this client talks to.
    pub fn server(&self) -> &str {
        &self.inner.server
    }

    /// Default tenant segment for tenant-scoped routes — `--tenant` value
    /// or `public`.
    pub fn tenant_segment(&self) -> &str {
        self.inner.tenant.as_deref().unwrap_or("public")
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.inner.server, path.trim_start_matches('/'))
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        // Tenant rides the URL path (`/tenants/{tenant}/...`), not a
        // header — auth is just the bearer token.
        self.inner
            .http
            .request(method, self.url(path))
            .bearer_auth(&self.inner.api_key)
    }

    async fn parse<T: DeserializeOwned>(response: Response) -> Result<T, ClientError> {
        let status = response.status().as_u16();
        if response.status().is_success() {
            return response
                .json::<T>()
                .await
                .map_err(ClientError::from_reqwest);
        }
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        Err(ClientError::from_status(status, body))
    }

    // ---- generic escape hatches (the surface cloacinactl's verb handlers
    // were built on; kept public so consumers can reach undocumented or
    // bleeding-edge routes) ----

    /// Typed GET of an arbitrary path.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let response = self
            .request(Method::GET, path)
            .send()
            .await
            .map_err(ClientError::from_reqwest)?;
        Self::parse(response).await
    }

    /// Typed POST (JSON body) to an arbitrary path.
    pub async fn post_json<B: serde::Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ClientError> {
        let response = self
            .request(Method::POST, path)
            .json(body)
            .send()
            .await
            .map_err(ClientError::from_reqwest)?;
        Self::parse(response).await
    }

    /// DELETE an arbitrary path, discarding any response body.
    pub async fn delete_path(&self, path: &str) -> Result<(), ClientError> {
        let response = self
            .request(Method::DELETE, path)
            .send()
            .await
            .map_err(ClientError::from_reqwest)?;
        let status = response.status().as_u16();
        if response.status().is_success() {
            return Ok(());
        }
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        Err(ClientError::from_status(status, body))
    }

    fn tenant_of<'a>(&'a self, tenant: Option<&'a str>) -> &'a str {
        tenant.unwrap_or_else(|| self.tenant_segment())
    }

    // ---- operational ----

    pub async fn health(&self) -> Result<Value, ClientError> {
        self.get_json("/health").await
    }

    /// Raw readiness response — 503 is a meaningful state, not an error.
    pub async fn ready(&self) -> Result<(u16, Value), ClientError> {
        let response = self
            .request(Method::GET, "/ready")
            .send()
            .await
            .map_err(ClientError::from_reqwest)?;
        let status = response.status().as_u16();
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        Ok((status, body))
    }

    // ---- keys ----

    pub async fn create_key(
        &self,
        name: &str,
        role: KeyRole,
    ) -> Result<KeyCreatedResponse, ClientError> {
        self.post_json(
            "/v1/auth/keys",
            &CreateKeyRequest {
                name: name.to_string(),
                role,
            },
        )
        .await
    }

    pub async fn list_keys(&self) -> Result<ListResponse<KeyInfo>, ClientError> {
        self.get_json("/v1/auth/keys").await
    }

    pub async fn revoke_key(&self, key_id: &str) -> Result<KeyRevokedResponse, ClientError> {
        let response = self
            .request(Method::DELETE, &format!("/v1/auth/keys/{key_id}"))
            .send()
            .await
            .map_err(ClientError::from_reqwest)?;
        Self::parse(response).await
    }

    pub async fn create_tenant_key(
        &self,
        name: &str,
        role: KeyRole,
        tenant: Option<&str>,
    ) -> Result<KeyCreatedResponse, ClientError> {
        let t = self.tenant_of(tenant);
        self.post_json(
            &format!("/v1/tenants/{t}/keys"),
            &CreateKeyRequest {
                name: name.to_string(),
                role,
            },
        )
        .await
    }

    /// Mint a single-use, short-lived WebSocket ticket.
    pub async fn create_ws_ticket(&self) -> Result<WsTicketResponse, ClientError> {
        self.post_json("/v1/auth/ws-ticket", &Value::Null).await
    }

    // ---- tenants ----

    pub async fn create_tenant(
        &self,
        request: &CreateTenantRequest,
    ) -> Result<TenantCreatedResponse, ClientError> {
        self.post_json("/v1/tenants", request).await
    }

    pub async fn list_tenants(&self) -> Result<ListResponse<TenantSummary>, ClientError> {
        self.get_json("/v1/tenants").await
    }

    pub async fn remove_tenant(
        &self,
        schema_name: &str,
    ) -> Result<TenantRemovedResponse, ClientError> {
        let response = self
            .request(Method::DELETE, &format!("/v1/tenants/{schema_name}"))
            .send()
            .await
            .map_err(ClientError::from_reqwest)?;
        Self::parse(response).await
    }

    // ---- workflows ----

    /// Upload a `.cloacina` package (multipart).
    pub async fn upload_workflow(
        &self,
        package: Vec<u8>,
        tenant: Option<&str>,
    ) -> Result<WorkflowUploadedResponse, ClientError> {
        let t = self.tenant_of(tenant);
        let part = reqwest::multipart::Part::bytes(package)
            .file_name("package.cloacina")
            .mime_str("application/octet-stream")
            .map_err(ClientError::from_reqwest)?;
        let form = reqwest::multipart::Form::new().part("file", part);
        let response = self
            .request(Method::POST, &format!("/v1/tenants/{t}/workflows"))
            .multipart(form)
            .send()
            .await
            .map_err(ClientError::from_reqwest)?;
        Self::parse(response).await
    }

    pub async fn list_workflows(
        &self,
        tenant: Option<&str>,
    ) -> Result<TenantListResponse<WorkflowSummary>, ClientError> {
        let t = self.tenant_of(tenant);
        self.get_json(&format!("/v1/tenants/{t}/workflows")).await
    }

    pub async fn get_workflow(
        &self,
        name: &str,
        tenant: Option<&str>,
    ) -> Result<WorkflowDetail, ClientError> {
        let t = self.tenant_of(tenant);
        self.get_json(&format!("/v1/tenants/{t}/workflows/{name}"))
            .await
    }

    pub async fn delete_workflow(
        &self,
        name: &str,
        version: &str,
        tenant: Option<&str>,
    ) -> Result<WorkflowDeletedResponse, ClientError> {
        let t = self.tenant_of(tenant);
        let response = self
            .request(
                Method::DELETE,
                &format!("/v1/tenants/{t}/workflows/{name}/{version}"),
            )
            .send()
            .await
            .map_err(ClientError::from_reqwest)?;
        Self::parse(response).await
    }

    // ---- triggers ----

    pub async fn list_triggers(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        tenant: Option<&str>,
    ) -> Result<TenantListResponse<TriggerScheduleSummary>, ClientError> {
        let t = self.tenant_of(tenant);
        let mut path = format!("/v1/tenants/{t}/triggers");
        let mut sep = '?';
        if let Some(l) = limit {
            path.push_str(&format!("{sep}limit={l}"));
            sep = '&';
        }
        if let Some(o) = offset {
            path.push_str(&format!("{sep}offset={o}"));
        }
        self.get_json(&path).await
    }

    pub async fn get_trigger(
        &self,
        name: &str,
        tenant: Option<&str>,
    ) -> Result<TriggerDetailResponse, ClientError> {
        let t = self.tenant_of(tenant);
        self.get_json(&format!("/v1/tenants/{t}/triggers/{name}"))
            .await
    }

    // ---- executions ----

    pub async fn execute_workflow(
        &self,
        name: &str,
        context: Value,
    ) -> Result<ExecuteResponse, ClientError> {
        let t = self.tenant_segment();
        self.post_json(
            &format!("/v1/tenants/{t}/workflows/{name}/execute"),
            &ExecuteRequest {
                context: Some(context),
            },
        )
        .await
    }

    pub async fn list_executions(
        &self,
        query: &cloacina_api_types::ListExecutionsQuery,
        tenant: Option<&str>,
    ) -> Result<TenantListResponse<ExecutionSummary>, ClientError> {
        let t = self.tenant_of(tenant);
        let mut path = format!("/v1/tenants/{t}/executions");
        let mut sep = '?';
        let mut push = |k: &str, v: String| {
            path.push_str(&format!("{sep}{k}={v}"));
            sep = '&';
        };
        if let Some(s) = &query.status {
            push("status", urlencoding::encode(s).into_owned());
        }
        if let Some(w) = &query.workflow {
            push("workflow", urlencoding::encode(w).into_owned());
        }
        if let Some(l) = query.limit {
            push("limit", l.to_string());
        }
        if let Some(o) = query.offset {
            push("offset", o.to_string());
        }
        self.get_json(&path).await
    }

    pub async fn get_execution(
        &self,
        exec_id: &str,
        tenant: Option<&str>,
    ) -> Result<ExecutionDetail, ClientError> {
        let t = self.tenant_of(tenant);
        self.get_json(&format!("/v1/tenants/{t}/executions/{exec_id}"))
            .await
    }

    pub async fn get_execution_events(
        &self,
        exec_id: &str,
        tenant: Option<&str>,
    ) -> Result<ExecutionEventsResponse, ClientError> {
        let t = self.tenant_of(tenant);
        self.get_json(&format!("/v1/tenants/{t}/executions/{exec_id}/events"))
            .await
    }

    pub async fn get_execution_tasks(
        &self,
        tenant_id: &str,
        exec_id: &str,
    ) -> Result<ExecutionTasksResponse, ClientError> {
        self.get_json(&format!(
            "/v1/tenants/{tenant_id}/executions/{exec_id}/tasks"
        ))
        .await
    }

    // ---- computation-graph health ----

    pub async fn list_accumulators(&self) -> Result<ListResponse<AccumulatorStatus>, ClientError> {
        self.get_json("/v1/health/accumulators").await
    }

    pub async fn list_graphs(&self) -> Result<ListResponse<GraphStatus>, ClientError> {
        self.get_json("/v1/health/graphs").await
    }

    pub async fn get_graph(&self, name: &str) -> Result<GraphStatus, ClientError> {
        self.get_json(&format!("/v1/health/graphs/{name}")).await
    }

    pub async fn list_reactors(&self) -> Result<ListResponse<ReactorStatus>, ClientError> {
        self.get_json("/v1/health/reactors").await
    }

    // ---- reactor operator controls (CLOACI-T-0772) ----

    pub async fn fire_reactor(
        &self,
        name: &str,
        request: &FireReactorRequest,
    ) -> Result<FireReactorResponse, ClientError> {
        self.post_json(&format!("/v1/health/reactors/{name}/fire"), request)
            .await
    }

    pub async fn list_reactor_fires(
        &self,
        name: &str,
    ) -> Result<ListResponse<ReactorFire>, ClientError> {
        self.get_json(&format!("/v1/health/reactors/{name}/fires"))
            .await
    }

    pub async fn reactor_fire_timeseries(
        &self,
        name: &str,
    ) -> Result<ReactorFireTimeseries, ClientError> {
        self.get_json(&format!("/v1/health/reactors/{name}/fires/timeseries"))
            .await
    }

    pub async fn reactor_interface(&self, name: &str) -> Result<DeclaredSurface, ClientError> {
        self.get_json(&format!("/v1/health/reactors/{name}/interface"))
            .await
    }

    pub async fn accumulator_interface(&self, name: &str) -> Result<DeclaredSurface, ClientError> {
        self.get_json(&format!("/v1/health/accumulators/{name}/interface"))
            .await
    }

    pub async fn inject_accumulator(
        &self,
        name: &str,
        request: &InjectAccumulatorRequest,
    ) -> Result<InjectAccumulatorResponse, ClientError> {
        self.post_json(&format!("/v1/health/accumulators/{name}/inject"), request)
            .await
    }

    // ---- workflow & trigger pause/resume + source (CLOACI-T-0772) ----

    pub async fn pause_workflow(
        &self,
        name: &str,
        tenant: Option<&str>,
    ) -> Result<WorkflowPauseResponse, ClientError> {
        let t = self.tenant_of(tenant);
        self.post_json(
            &format!("/v1/tenants/{t}/workflows/{name}/pause"),
            &Value::Null,
        )
        .await
    }

    pub async fn resume_workflow(
        &self,
        name: &str,
        tenant: Option<&str>,
    ) -> Result<WorkflowPauseResponse, ClientError> {
        let t = self.tenant_of(tenant);
        self.post_json(
            &format!("/v1/tenants/{t}/workflows/{name}/resume"),
            &Value::Null,
        )
        .await
    }

    pub async fn get_workflow_source(
        &self,
        name: &str,
        tenant: Option<&str>,
    ) -> Result<WorkflowSourceResponse, ClientError> {
        let t = self.tenant_of(tenant);
        self.get_json(&format!("/v1/tenants/{t}/workflows/{name}/source"))
            .await
    }

    pub async fn pause_trigger(
        &self,
        name: &str,
        tenant: Option<&str>,
    ) -> Result<TriggerPauseResponse, ClientError> {
        let t = self.tenant_of(tenant);
        self.post_json(
            &format!("/v1/tenants/{t}/triggers/{name}/pause"),
            &Value::Null,
        )
        .await
    }

    /// Manually fire a trigger — fans out to every subscribed workflow
    /// (CLOACI-T-0777).
    pub async fn fire_trigger(
        &self,
        name: &str,
        request: &FireTriggerRequest,
        tenant: Option<&str>,
    ) -> Result<FireTriggerResponse, ClientError> {
        let t = self.tenant_of(tenant);
        self.post_json(&format!("/v1/tenants/{t}/triggers/{name}/fire"), request)
            .await
    }

    pub async fn resume_trigger(
        &self,
        name: &str,
        tenant: Option<&str>,
    ) -> Result<TriggerPauseResponse, ClientError> {
        let t = self.tenant_of(tenant);
        self.post_json(
            &format!("/v1/tenants/{t}/triggers/{name}/resume"),
            &Value::Null,
        )
        .await
    }

    // ---- fleet / compiler ----

    pub async fn list_agents(&self) -> Result<ListResponse<AgentInfo>, ClientError> {
        self.get_json("/v1/agents").await
    }

    pub async fn compiler_status(&self) -> Result<CompilerStatus, ClientError> {
        self.get_json("/v1/compiler/status").await
    }

    // ---- WebSocket (substrate delivery) ----

    /// Subscribe to the substrate delivery stream for a recipient. Yields
    /// each push exactly once (dedup on row id), acking after yield;
    /// reconnects with exponential backoff. See [`ws::SubscribeOptions`].
    pub fn subscribe_delivery(
        &self,
        recipient: &str,
        options: SubscribeOptions,
    ) -> impl futures_util::Stream<Item = Result<DeliveryPush, ClientError>> + '_ {
        ws::subscribe_delivery(self.clone(), recipient.to_string(), options)
    }

    /// Stream the JSON events of one workflow execution — recipient
    /// convention `exec_events:<execution_id>`, the same stream
    /// `cloacinactl execution follow` renders.
    pub fn follow_execution_events(
        &self,
        execution_id: &str,
    ) -> impl futures_util::Stream<Item = Result<Value, ClientError>> + '_ {
        self.follow_execution_events_with(execution_id, SubscribeOptions::default())
    }

    /// [`follow_execution_events`](Self::follow_execution_events) with
    /// explicit subscription options (reconnect policy, backoff).
    pub fn follow_execution_events_with(
        &self,
        execution_id: &str,
        options: SubscribeOptions,
    ) -> impl futures_util::Stream<Item = Result<Value, ClientError>> + '_ {
        ws::follow_execution_events(self.clone(), execution_id.to_string(), options)
    }
}
