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

//! Reactor-subscription API for the DefaultRunner (CLOACI-I-0100 / T-0600).
//!
//! Surfaces a thin user-facing wrapper over the
//! `ReactorSubscriptionsDAL::subscribe / unsubscribe / list` operations.
//! The unfiltered registration path — "fire workflow on every reactor
//! firing for this tenant" — lives here. The optional Python filter
//! callback (`@trigger(reactor=...)`) is a follow-up surface.

use uuid::Uuid;

use crate::dal::unified::ReactorSubscription;
use crate::dal::DAL;
use crate::executor::workflow_executor::WorkflowExecutionError;

use super::DefaultRunner;

/// Default tenant used when the caller passes `None`. Matches the
/// hobbyist single-tenant default used elsewhere in the runner stack.
const DEFAULT_TENANT: &str = "public";

impl DefaultRunner {
    /// Subscribe a workflow to a reactor's firings.
    ///
    /// Each future fire of `reactor` (for `tenant`) will dispatch
    /// `workflow` via the unified scheduler's reactor poll tick
    /// (CLOACI-T-0599). The dispatched workflow receives the boundary
    /// cache from the firing as its input context.
    ///
    /// Idempotent: calling twice with the same `(reactor, workflow,
    /// tenant)` returns the existing subscription id without error.
    ///
    /// # Arguments
    /// * `reactor` - The reactor name (matches `reactor_name` on
    ///   loaded CG declarations).
    /// * `workflow` - The workflow to dispatch.
    /// * `tenant` - Tenant scope. `None` ⇒ `"public"`.
    pub async fn subscribe_workflow_to_reactor(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: Option<&str>,
    ) -> Result<Uuid, WorkflowExecutionError> {
        let tenant = tenant.unwrap_or(DEFAULT_TENANT);
        let dal = DAL::new(self.database.clone());
        dal.reactor_subscriptions()
            .subscribe(reactor, workflow, tenant)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!(
                    "failed to subscribe workflow '{}' to reactor '{}' (tenant={}): {}",
                    workflow, reactor, tenant, e
                ),
            })
    }

    /// Remove a workflow-to-reactor subscription. Returns true if a
    /// row was deleted, false if no subscription matched.
    pub async fn unsubscribe_workflow_from_reactor(
        &self,
        reactor: &str,
        workflow: &str,
        tenant: Option<&str>,
    ) -> Result<bool, WorkflowExecutionError> {
        let tenant = tenant.unwrap_or(DEFAULT_TENANT);
        let dal = DAL::new(self.database.clone());
        dal.reactor_subscriptions()
            .unsubscribe(reactor, workflow, tenant)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!(
                    "failed to unsubscribe workflow '{}' from reactor '{}' (tenant={}): {}",
                    workflow, reactor, tenant, e
                ),
            })
    }

    /// List enabled reactor subscriptions for a tenant.
    pub async fn list_reactor_subscriptions(
        &self,
        tenant: Option<&str>,
    ) -> Result<Vec<ReactorSubscription>, WorkflowExecutionError> {
        let tenant = tenant.unwrap_or(DEFAULT_TENANT);
        let dal = DAL::new(self.database.clone());
        dal.reactor_subscriptions()
            .list_subscriptions(tenant)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!(
                    "failed to list reactor subscriptions for tenant '{}': {}",
                    tenant, e
                ),
            })
    }
}
