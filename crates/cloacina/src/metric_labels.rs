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

//! Label values for metrics that carry a workflow name (CLOACINA-T-1024).
//!
//! A workflow name comes from an uploaded package, so a tenant chooses it.
//! Used as a label value with no limit, it would let one tenant create
//! time series without end. [`workflow_label`] keeps the first
//! [`MAX_WORKFLOW_LABELS_PER_TENANT`] names it sees for a tenant and gives
//! [`OVERFLOW_LABEL`] for each later one.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

/// Distinct `workflow` label values kept per tenant, per process.
pub const MAX_WORKFLOW_LABELS_PER_TENANT: usize = 100;

/// The `workflow` label value for a name over the limit.
pub const OVERFLOW_LABEL: &str = "other";

/// The workflow names seen so far, by tenant.
#[derive(Default)]
struct SeenWorkflows {
    by_tenant: HashMap<String, HashSet<String>>,
}

impl SeenWorkflows {
    fn label(&mut self, tenant: &str, workflow: &str, max_per_tenant: usize) -> String {
        if let Some(seen) = self.by_tenant.get(tenant)
            && seen.contains(workflow)
        {
            return workflow.to_string();
        }
        let seen = self.by_tenant.entry(tenant.to_string()).or_default();
        if seen.len() >= max_per_tenant {
            return OVERFLOW_LABEL.to_string();
        }
        seen.insert(workflow.to_string());
        workflow.to_string()
    }
}

/// The `workflow` label value for `workflow` of `tenant`: the name itself for
/// the first [`MAX_WORKFLOW_LABELS_PER_TENANT`] names of the tenant in this
/// process, [`OVERFLOW_LABEL`] after that. A name keeps its label once it has
/// one. The set is in memory: it starts empty after a restart.
pub fn workflow_label(tenant: &str, workflow: &str) -> String {
    static SEEN: OnceLock<Mutex<SeenWorkflows>> = OnceLock::new();
    SEEN.get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .label(tenant, workflow, MAX_WORKFLOW_LABELS_PER_TENANT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_label_keeps_names_up_to_the_limit_then_gives_other() {
        let mut seen = SeenWorkflows::default();
        assert_eq!(seen.label("acme", "orders", 2), "orders");
        assert_eq!(seen.label("acme", "billing", 2), "billing");
        assert_eq!(seen.label("acme", "reports", 2), OVERFLOW_LABEL);
        // A name that has a label keeps it.
        assert_eq!(seen.label("acme", "orders", 2), "orders");
        assert_eq!(seen.label("acme", "reports", 2), OVERFLOW_LABEL);
    }

    #[test]
    fn workflow_label_limit_is_per_tenant() {
        let mut seen = SeenWorkflows::default();
        assert_eq!(seen.label("acme", "orders", 1), "orders");
        assert_eq!(seen.label("acme", "billing", 1), OVERFLOW_LABEL);
        // Another tenant has its own limit.
        assert_eq!(seen.label("globex", "billing", 1), "billing");
    }
}
