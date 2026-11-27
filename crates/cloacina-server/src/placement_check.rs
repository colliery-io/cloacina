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

//! The publish-time check of a package's task placements (CLOACINA-T-1027).
//!
//! A task may name an agent pool and request CPU and memory. A package is
//! refused when one of its tasks can never run:
//!
//! - it names a pool the tenant does not have, or
//! - it requests more CPU or memory than one agent of its pool can hold. What
//!   an agent of a pool can hold is what the pool's settings say: its limit,
//!   or its request when it sets no limit
//!   ([`pool_agent_totals`](crate::actuator::settings::pool_agent_totals)). A
//!   pool that sets neither has no size to compare with, so only the pool's
//!   name is checked.
//!
//! The check runs where the tenant's runner loads the package, which is the
//! first moment its tasks are known (for a Rust package, after the compile):
//! the server registers [`PoolPlacementCheck`] with the registry reconciler
//! ([`cloacina::registry::register_task_placement_check`]). A refused package
//! gets build status `failed` with the message in `build_error`.

use async_trait::async_trait;
use cloacina::database::Database;
use cloacina::registry::{PlacedTask, TaskPlacementCheck};
use cloacina_api_types::{AgentPoolSettings, parse_cpu_millis, parse_memory_bytes};
use tracing::warn;

use crate::actuator::settings::{load_pools, pool_agent_totals};

/// Check the tasks of a package against a tenant's pools. Pure: `pools` is
/// what [`load_pools`] gives (the pool `default` is always in it).
///
/// The error is one sentence for the publisher. It names the task, the pool
/// and the numbers.
pub fn check_tasks(
    tenant_id: &str,
    pools: &[(String, AgentPoolSettings)],
    tasks: &[PlacedTask],
) -> Result<(), String> {
    for task in tasks {
        let pool_name = task.placement.pool_name();
        let Some((_, settings)) = pools.iter().find(|(name, _)| name == pool_name) else {
            let known: Vec<&str> = pools.iter().map(|(name, _)| name.as_str()).collect();
            return Err(format!(
                "task '{}' names the agent pool '{}', which the tenant '{}' does not have (its \
                 pools: {}). Create the pool, or name one of these, and publish again.",
                task.task_id,
                pool_name,
                tenant_id,
                known.join(", ")
            ));
        };
        let (cpu_total, memory_total) = pool_agent_totals(settings);
        // (what, the request as written, the request, the pool's size as
        // written, the pool's size). A size that does not parse is not
        // compared: the pool routes check it when it is set.
        let over = |what: &str,
                    asked_text: &Option<String>,
                    asked: Option<u64>,
                    total_text: &Option<String>,
                    total: Option<u64>| {
            match (asked_text, asked, total_text, total) {
                (Some(asked_text), Some(asked), Some(total_text), Some(total)) if asked > total => {
                    Err(format!(
                        "task '{}' requests {} {}, but one agent of the pool '{}' holds {} at \
                         most. Lower the request, or raise the pool's {} limit, and publish \
                         again.",
                        task.task_id, what, asked_text, pool_name, total_text, what
                    ))
                }
                _ => Ok(()),
            }
        };
        over(
            "cpu",
            &task.placement.cpu,
            task.placement.cpu_millis(),
            &cpu_total,
            cpu_total.as_deref().and_then(|t| parse_cpu_millis(t).ok()),
        )?;
        over(
            "memory",
            &task.placement.memory,
            task.placement.memory_bytes(),
            &memory_total,
            memory_total
                .as_deref()
                .and_then(|t| parse_memory_bytes(t).ok()),
        )?;
    }
    Ok(())
}

/// The check the server registers: a tenant's pools are read from the
/// `agent_pools` table of the admin database.
pub struct PoolPlacementCheck {
    database: Database,
}

impl PoolPlacementCheck {
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl TaskPlacementCheck for PoolPlacementCheck {
    async fn check(&self, tenant_id: &str, tasks: &[PlacedTask]) -> Result<(), String> {
        match load_pools(&self.database, tenant_id).await {
            Ok(pools) => check_tasks(tenant_id, &pools, tasks),
            // The pools cannot be read now. That says nothing about the
            // package, so it is not refused: a task that fits no agent waits.
            Err(e) => {
                warn!(
                    tenant = %tenant_id,
                    error = %e,
                    "could not read the tenant's agent pools; the package is loaded with no \
                     placement check"
                );
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_api_types::{DEFAULT_AGENT_POOL, TaskPlacement};

    fn task(id: &str, pool: Option<&str>, cpu: Option<&str>, memory: Option<&str>) -> PlacedTask {
        PlacedTask {
            task_id: id.to_string(),
            placement: TaskPlacement {
                pool: pool.map(str::to_string),
                cpu: cpu.map(str::to_string),
                memory: memory.map(str::to_string),
                inline: false,
            },
        }
    }

    /// The tenant's pools: `default` with no settings, and `large` with a
    /// 4 CPU / 32 Gi limit.
    fn pools() -> Vec<(String, AgentPoolSettings)> {
        vec![
            (DEFAULT_AGENT_POOL.to_string(), AgentPoolSettings::default()),
            (
                "large".to_string(),
                AgentPoolSettings {
                    cpu_request: Some("1".into()),
                    memory_request: Some("8Gi".into()),
                    cpu_limit: Some("4".into()),
                    memory_limit: Some("32Gi".into()),
                    max_agents: Some(2),
                    ..Default::default()
                },
            ),
        ]
    }

    #[test]
    fn tasks_that_fit_their_pool_pass() {
        let tasks = vec![
            task("plain", None, None, None),
            task("small", None, Some("500m"), Some("512Mi")),
            task("big", Some("large"), Some("4"), Some("32Gi")),
            task("big-no-request", Some("large"), None, None),
        ];
        assert_eq!(check_tasks("acme", &pools(), &tasks), Ok(()));
    }

    #[test]
    fn a_pool_the_tenant_does_not_have_is_refused_and_named() {
        let tasks = vec![
            task("fine", None, None, None),
            task("train_model", Some("gpu"), None, Some("8Gi")),
        ];
        let err = check_tasks("acme", &pools(), &tasks).unwrap_err();
        assert!(err.contains("task 'train_model'"), "{err}");
        assert!(err.contains("pool 'gpu'"), "{err}");
        assert!(err.contains("tenant 'acme'"), "{err}");
        assert!(err.contains("default, large"), "{err}");
    }

    #[test]
    fn a_request_above_one_agent_of_the_pool_is_refused_with_the_numbers() {
        let err = check_tasks(
            "acme",
            &pools(),
            &[task("load_history", Some("large"), Some("2"), Some("64Gi"))],
        )
        .unwrap_err();
        assert!(err.contains("task 'load_history'"), "{err}");
        assert!(err.contains("memory 64Gi"), "{err}");
        assert!(err.contains("pool 'large'"), "{err}");
        assert!(err.contains("holds 32Gi at most"), "{err}");

        let err = check_tasks(
            "acme",
            &pools(),
            &[task("crunch", Some("large"), Some("8"), Some("1Gi"))],
        )
        .unwrap_err();
        assert!(err.contains("task 'crunch'"), "{err}");
        assert!(err.contains("cpu 8"), "{err}");
        assert!(err.contains("holds 4 at most"), "{err}");
    }

    /// The comparison is on amounts, not on text: `32768Mi` is `32Gi`.
    #[test]
    fn a_request_equal_to_the_pool_size_in_another_notation_passes() {
        let tasks = vec![task("exact", Some("large"), Some("4000m"), Some("32768Mi"))];
        assert_eq!(check_tasks("acme", &pools(), &tasks), Ok(()));
    }

    /// With no limit the pool's request is the size of its agents.
    #[test]
    fn a_pool_with_a_request_and_no_limit_is_sized_by_the_request() {
        let pools = vec![
            (DEFAULT_AGENT_POOL.to_string(), AgentPoolSettings::default()),
            (
                "medium".to_string(),
                AgentPoolSettings {
                    memory_request: Some("8Gi".into()),
                    max_agents: Some(1),
                    ..Default::default()
                },
            ),
        ];
        let fits = [task("ok", Some("medium"), Some("64"), Some("8Gi"))];
        assert_eq!(
            check_tasks("acme", &pools, &fits),
            Ok(()),
            "no cpu size: not compared"
        );
        let err = check_tasks(
            "acme",
            &pools,
            &[task("no", Some("medium"), None, Some("9Gi"))],
        )
        .unwrap_err();
        assert!(err.contains("holds 8Gi at most"), "{err}");
    }

    /// A pool that sets no size has nothing to compare a request with: its
    /// agents report no totals and are packed by their slots.
    #[test]
    fn a_pool_with_no_size_checks_the_name_only() {
        let tasks = vec![task("huge", None, Some("512"), Some("4Ti"))];
        assert_eq!(check_tasks("acme", &pools(), &tasks), Ok(()));
    }
}
