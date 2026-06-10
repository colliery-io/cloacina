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

//! CLI-facing wrapper over the published `cloacina-client` crate (T-0646).
//!
//! The HTTP client itself moved to `crates/cloacina-client` so external
//! consumers get the exact surface the CLI exercises; this wrapper keeps
//! the CLI-shaped pieces — `ClientContext` resolution and `CliError`
//! exit-code mapping — and delegates everything else.

use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;

/// Shared HTTP client used by every verb handler.
pub struct CliClient {
    ctx: ClientContext,
    inner: cloacina_client::Client,
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
        let mut builder = cloacina_client::ClientBuilder::new(&ctx.server).api_key(&ctx.api_key);
        if let Some(tenant) = &ctx.tenant {
            builder = builder.tenant(tenant);
        }
        let inner = builder.build().map_err(CliError::from)?;
        Ok(Arc::new(Self { ctx, inner }))
    }

    pub fn ctx(&self) -> &ClientContext {
        &self.ctx
    }

    /// The underlying published client, for typed/WS calls.
    pub fn inner(&self) -> &cloacina_client::Client {
        &self.inner
    }

    /// Typed GET.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, CliError> {
        self.inner.get_json(path).await.map_err(CliError::from)
    }

    /// Typed POST (JSON body).
    pub async fn post<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, CliError> {
        self.inner
            .post_json(path, body)
            .await
            .map_err(CliError::from)
    }

    /// DELETE without a response body.
    pub async fn delete(&self, path: &str) -> Result<(), CliError> {
        self.inner.delete_path(path).await.map_err(CliError::from)
    }
}
