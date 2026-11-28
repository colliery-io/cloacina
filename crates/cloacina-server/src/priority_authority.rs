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

//! The limit on the priority that the code of a package declares
//! (CLOACINA-T-1045).
//!
//! A package declares priorities in its code: `#[workflow(priority = 2)]`,
//! `#[task(priority = 0)]`. On a server that code was written by whoever
//! published the package, so a declared priority is held to a limit when a
//! run starts. The limit here is the one of the role that may publish: the
//! default of `write`. A package cannot give itself a priority that a
//! `write` principal could not set on a run.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use cloacina::execution_planner::PriorityAuthority;
use cloacina_api_types::PriorityGrant;
use tracing::warn;

use crate::routes::authz::{LimitKind, Role};

/// The limit of the declared priorities of one tenant's packages.
pub struct PublisherLimit {
    tenant: String,
    /// The priority last given to each workflow that was held, so the log
    /// line is written when the given priority changes, not for each run.
    last_given: Mutex<HashMap<String, PriorityGrant>>,
}

impl PublisherLimit {
    pub fn for_tenant(tenant: impl Into<String>) -> Self {
        Self {
            tenant: tenant.into(),
            last_given: Mutex::new(HashMap::new()),
        }
    }

    /// True when `grant` is not what this process last saw for the workflow.
    fn changed(&self, workflow_name: &str, grant: PriorityGrant) -> bool {
        let mut last = self.last_given.lock().unwrap_or_else(|e| e.into_inner());
        last.insert(workflow_name.to_string(), grant) != Some(grant)
    }
}

/// The highest priority the code of a package may set: the default of the
/// role that may publish.
pub fn declared_priority_limit() -> Option<u8> {
    Role::Write.default_limits().get(LimitKind::MaxPriority)
}

#[async_trait]
impl PriorityAuthority for PublisherLimit {
    async fn declared_limit(&self, _workflow_name: &str) -> Option<u8> {
        declared_priority_limit()
    }

    async fn run_held(&self, workflow_name: &str, grant: PriorityGrant) {
        if self.changed(workflow_name, grant) {
            warn!(
                tenant = %self.tenant,
                workflow = %workflow_name,
                priority_asked = grant.asked,
                priority_given = grant.given,
                "priority lowered: the workflow asks for priority {} and its runs get \
                 priority {}",
                grant.asked,
                grant.given
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_package_may_declare_what_a_write_principal_may_set() {
        let limit = PublisherLimit::for_tenant("acme");
        assert_eq!(limit.declared_limit("orders").await, Some(5));
        assert_eq!(
            PriorityGrant::held_to(2, declared_priority_limit()).given,
            5
        );
        assert_eq!(
            PriorityGrant::held_to(8, declared_priority_limit()).given,
            8
        );
    }

    #[test]
    fn the_ageing_interval_is_two_minutes_unless_the_setting_says_otherwise() {
        use std::time::Duration;
        let parse = crate::parse_priority_aging_interval;
        assert_eq!(parse(None), Duration::from_secs(120));
        assert_eq!(parse(Some("")), Duration::from_secs(120));
        assert_eq!(parse(Some("300")), Duration::from_secs(300));
        assert_eq!(parse(Some(" 60 ")), Duration::from_secs(60));
        assert_eq!(parse(Some("0")), Duration::ZERO, "0 switches ageing off");
        assert_eq!(parse(Some("2m")), Duration::from_secs(120), "not a number");
    }

    #[test]
    fn the_log_line_is_for_a_change_of_the_given_priority_not_for_each_run() {
        let limit = PublisherLimit::for_tenant("acme");
        let held = PriorityGrant::held_to(2, Some(5));
        assert!(limit.changed("orders", held), "the first held run");
        assert!(!limit.changed("orders", held), "the same again");
        assert!(!limit.changed("orders", held));
        let other = PriorityGrant::held_to(2, Some(7));
        assert!(limit.changed("orders", other), "the given priority moved");
        assert!(limit.changed("billing", held), "another workflow");
    }
}
