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

//! Fleet placement of task executions (CLOACINA-T-1027).
//!
//! A task row carries the agent pool the task runs in and its CPU and memory
//! requests (written by the planner), and, while a fleet dispatch holds the
//! row's claim, the agent the task was placed on (`fleet_agent_id`).
//!
//! The requests in flight on an agent are not kept in a counter. They are the
//! sum over the task rows placed on it: `fleet_agent_id` set, a claim held,
//! status `Ready` or `Running`. A row leaves that set by itself when its
//! dispatch ends (the claim is released), when the task reaches an end state,
//! or when the stale claim sweep takes the claim of a dead server replica. So
//! every replica reads the same figure from the database and nothing can leak.

use super::{TaskExecutionDAL, WaitingTasks};
use crate::database::schema::unified::task_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use diesel::prelude::*;

/// The pool a task row with no pool is counted in.
const DEFAULT_POOL: &str = cloacina_api_types::DEFAULT_AGENT_POOL;

/// One task in flight on a fleet agent (CLOACINA-T-1027).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetPlacement {
    /// The task execution.
    pub task_id: UniversalUuid,
    /// The agent it is placed on.
    pub agent_id: String,
    /// The task's CPU request in millicores. `None` = it declared none.
    pub cpu_request_millis: Option<u64>,
    /// The task's memory request in bytes. `None` = it declared none.
    pub memory_request_bytes: Option<u64>,
}

/// The waiting tasks of one pool (CLOACINA-T-1027).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolWaitingTasks {
    /// The pool. A task with no pool is in `default`.
    pub pool: String,
    /// Its waiting tasks.
    pub waiting: WaitingTasks,
}

/// The waiting tasks of one pool at one workflow priority (CLOACINA-T-1123).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolPriorityWaitingTasks {
    /// The pool. A task with no pool is in `default`.
    pub pool: String,
    /// The workflow priority stored on the task rows, 0 to 10.
    pub priority: u8,
    /// Its waiting tasks.
    pub waiting: WaitingTasks,
}

/// Waiting tasks of one pool with the same requests (CLOACINA-T-1029).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingDemand {
    /// The pool. A task with no pool is in `default`.
    pub pool: String,
    /// The CPU request of each of these tasks in millicores. `None`: none.
    pub cpu_request_millis: Option<u64>,
    /// The memory request of each of these tasks in bytes. `None`: none.
    pub memory_request_bytes: Option<u64>,
    /// How many tasks wait with these requests.
    pub count: u64,
}

impl<'a> TaskExecutionDAL<'a> {
    /// The waiting work of each pool, grouped by requests (CLOACINA-T-1029):
    /// the tasks that are `Ready`, waiting (the rows of
    /// [`waiting_task_stats_by_pool`](Self::waiting_task_stats_by_pool)) and
    /// on no agent. A task in the middle of a dispatch (claimed, not placed)
    /// is still waiting; a placed one is in flight.
    ///
    /// One grouped read: task sizes are few distinct values, so the autoscaler
    /// packs groups, not rows. Sorted by pool, then by memory and CPU.
    pub async fn waiting_demand_by_pool(&self) -> Result<Vec<WaitingDemand>, ValidationError> {
        let now = UniversalTimestamp::now();
        type Row = (Option<String>, Option<i64>, Option<i64>, i64);
        let rows: Vec<Row> = crate::interact_on_backend!(self.dal, |conn| {
            task_executions::table
                .filter(task_executions::status.eq("Ready"))
                .filter(task_executions::fleet_agent_id.is_null())
                .filter(
                    task_executions::waiting_since
                        .le(now)
                        .or(task_executions::waiting_since.is_null().and(
                            task_executions::retry_at
                                .is_null()
                                .or(task_executions::retry_at.le(now)),
                        )),
                )
                .group_by((
                    task_executions::pool,
                    task_executions::cpu_request_millis,
                    task_executions::memory_request_bytes,
                ))
                .select((
                    task_executions::pool,
                    task_executions::cpu_request_millis,
                    task_executions::memory_request_bytes,
                    diesel::dsl::count_star(),
                ))
                .load::<Row>(conn)
        })?;
        // A row with no pool and a row that names `default` are one pool.
        let mut merged: std::collections::BTreeMap<(String, Option<u64>, Option<u64>), u64> =
            std::collections::BTreeMap::new();
        for (pool, cpu, memory, count) in rows {
            let key = (
                pool.unwrap_or_else(|| DEFAULT_POOL.to_string()),
                cpu.and_then(|v| u64::try_from(v).ok()),
                memory.and_then(|v| u64::try_from(v).ok()),
            );
            *merged.entry(key).or_insert(0) += u64::try_from(count).unwrap_or(0);
        }
        Ok(merged
            .into_iter()
            .map(|((pool, cpu, memory), count)| WaitingDemand {
                pool,
                cpu_request_millis: cpu,
                memory_request_bytes: memory,
                count,
            })
            .collect())
    }

    /// Record that the fleet placed a task on an agent. Only the runner that
    /// holds the claim can place the task. Returns `false` when it does not
    /// hold the claim (the row is not changed).
    ///
    /// The caller then reads [`fleet_placements`](Self::fleet_placements)
    /// again and takes the placement back with
    /// [`clear_fleet_placement`](Self::clear_fleet_placement) when the agent
    /// is over its totals: that is how two server replicas that pick the same
    /// agent at the same moment do not both keep it.
    ///
    /// `cpu_millis` and `memory_bytes` are the requests the task was selected
    /// with. They are written with the placement, so the row holds what the
    /// selection used (also for a row made before the columns existed).
    pub async fn place_on_fleet_agent(
        &self,
        task_id: UniversalUuid,
        runner_id: UniversalUuid,
        agent_id: &str,
        cpu_millis: Option<u64>,
        memory_bytes: Option<u64>,
    ) -> Result<bool, ValidationError> {
        let agent_id = agent_id.to_string();
        let cpu = cpu_millis.and_then(|v| i64::try_from(v).ok());
        let memory = memory_bytes.and_then(|v| i64::try_from(v).ok());
        let now = UniversalTimestamp::now();
        let rows: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                task_executions::table
                    .find(task_id)
                    .filter(task_executions::claimed_by.eq(Some(runner_id))),
            )
            .set((
                task_executions::fleet_agent_id.eq(Some(agent_id)),
                task_executions::cpu_request_millis.eq(cpu),
                task_executions::memory_request_bytes.eq(memory),
                task_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;
        Ok(rows > 0)
    }

    /// Take a task off its agent: the placement lost a race. Idempotent.
    /// (The end of a dispatch needs no call: releasing the claim takes the
    /// task off its agent, see `release_runner_claim`.)
    pub async fn clear_fleet_placement(
        &self,
        task_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                task_executions::table
                    .find(task_id)
                    .filter(task_executions::fleet_agent_id.is_not_null()),
            )
            .set(task_executions::fleet_agent_id.eq(None::<String>))
            .execute(conn)
        })?;
        Ok(())
    }

    /// The tasks in flight on fleet agents: placed on an agent, with a claim
    /// held, in status `Ready` (placed, packet not yet sent) or `Running`.
    ///
    /// A row whose claim is gone or whose task has ended is not in flight,
    /// whatever its `fleet_agent_id` says.
    pub async fn fleet_placements(&self) -> Result<Vec<FleetPlacement>, ValidationError> {
        type Row = (UniversalUuid, Option<String>, Option<i64>, Option<i64>);
        let rows: Vec<Row> = crate::interact_on_backend!(self.dal, |conn| {
            task_executions::table
                .filter(task_executions::fleet_agent_id.is_not_null())
                .filter(task_executions::claimed_by.is_not_null())
                .filter(task_executions::status.eq_any(vec!["Ready", "Running"]))
                .select((
                    task_executions::id,
                    task_executions::fleet_agent_id,
                    task_executions::cpu_request_millis,
                    task_executions::memory_request_bytes,
                ))
                .load::<Row>(conn)
        })?;
        Ok(rows
            .into_iter()
            .filter_map(|(task_id, agent_id, cpu, memory)| {
                Some(FleetPlacement {
                    task_id,
                    agent_id: agent_id?,
                    cpu_request_millis: cpu.and_then(|v| u64::try_from(v).ok()),
                    memory_request_bytes: memory.and_then(|v| u64::try_from(v).ok()),
                })
            })
            .collect())
    }

    /// The waiting tasks of each pool that has any (CLOACINA-T-1027): the
    /// figures of [`waiting_task_stats`](Self::waiting_task_stats), grouped by
    /// the task's pool. A task with no pool is counted in `default`. A pool
    /// with no waiting task is not in the result. Sorted by pool name.
    ///
    /// Reads no task rows: one grouped count, and for each pool in it a
    /// one-column `ORDER BY … LIMIT 1` read for its earliest `waiting_since`.
    pub async fn waiting_task_stats_by_pool(
        &self,
    ) -> Result<Vec<PoolWaitingTasks>, ValidationError> {
        let mut by_pool: std::collections::BTreeMap<String, WaitingTasks> =
            std::collections::BTreeMap::new();
        for group in self.waiting_task_stats_by_pool_and_priority().await? {
            merge_waiting(
                by_pool.entry(group.pool).or_insert(WaitingTasks {
                    count: 0,
                    oldest_waiting_since: None,
                }),
                &group.waiting,
            );
        }
        Ok(by_pool
            .into_iter()
            .map(|(pool, waiting)| PoolWaitingTasks { pool, waiting })
            .collect())
    }

    /// The waiting tasks of each pool and workflow priority that has any
    /// (CLOACINA-T-1123): the figures of
    /// [`waiting_task_stats_by_pool`](Self::waiting_task_stats_by_pool),
    /// grouped also by the workflow priority stored on the task row (0 to
    /// 10, so at most 11 groups per pool). Sorted by pool, then priority.
    ///
    /// Reads no task rows: one grouped count, and for each group in it a
    /// one-column `ORDER BY … LIMIT 1` read for its earliest `waiting_since`.
    pub async fn waiting_task_stats_by_pool_and_priority(
        &self,
    ) -> Result<Vec<PoolPriorityWaitingTasks>, ValidationError> {
        let now = UniversalTimestamp::now();
        type Group = (Option<String>, i32, i64, Option<UniversalTimestamp>);
        let per_group: Vec<Group> = crate::interact_on_backend!(self.dal, |conn| {
            let counts: Vec<(Option<String>, i32, i64)> = task_executions::table
                .filter(task_executions::status.eq("Ready"))
                .filter(
                    task_executions::waiting_since
                        .le(now)
                        .or(task_executions::waiting_since.is_null().and(
                            task_executions::retry_at
                                .is_null()
                                .or(task_executions::retry_at.le(now)),
                        )),
                )
                .group_by((task_executions::pool, task_executions::workflow_priority))
                .select((
                    task_executions::pool,
                    task_executions::workflow_priority,
                    diesel::dsl::count_star(),
                ))
                .load(conn)?;
            let mut out = Vec::with_capacity(counts.len());
            for (pool, priority, count) in counts {
                let mut oldest = task_executions::table
                    .filter(task_executions::status.eq("Ready"))
                    .filter(task_executions::waiting_since.le(now))
                    .filter(task_executions::workflow_priority.eq(priority))
                    .order(task_executions::waiting_since.asc())
                    .select(task_executions::waiting_since)
                    .into_boxed();
                oldest = match &pool {
                    Some(name) => oldest.filter(task_executions::pool.eq(name.clone())),
                    None => oldest.filter(task_executions::pool.is_null()),
                };
                let oldest: Option<UniversalTimestamp> = oldest
                    .first::<Option<UniversalTimestamp>>(conn)
                    .optional()?
                    .flatten();
                out.push((pool, priority, count, oldest));
            }
            Ok::<_, diesel::result::Error>(out)
        })?;

        // A row with no pool and a row that names `default` are one pool; a
        // stored priority outside the scale counts at its nearest end.
        let mut groups: std::collections::BTreeMap<(String, u8), WaitingTasks> =
            std::collections::BTreeMap::new();
        for (pool, priority, count, oldest) in per_group {
            let key = (
                pool.unwrap_or_else(|| DEFAULT_POOL.to_string()),
                cloacina_api_types::clamp_priority(priority),
            );
            merge_waiting(
                groups.entry(key).or_insert(WaitingTasks {
                    count: 0,
                    oldest_waiting_since: None,
                }),
                &WaitingTasks {
                    count,
                    oldest_waiting_since: oldest.map(|t| t.0),
                },
            );
        }
        Ok(groups
            .into_iter()
            .map(|((pool, priority), waiting)| PoolPriorityWaitingTasks {
                pool,
                priority,
                waiting,
            })
            .collect())
    }
}

/// Add the figures of `more` to `into`: the counts add up, the earliest
/// `waiting_since` stays.
fn merge_waiting(into: &mut WaitingTasks, more: &WaitingTasks) {
    into.count += more.count;
    into.oldest_waiting_since = match (into.oldest_waiting_since, more.oldest_waiting_since) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::dal::DAL;
    use crate::database::Database;
    use crate::models::task_execution::NewTaskExecution;
    use crate::models::workflow_execution::NewWorkflowExecution;

    async fn unique_dal() -> DAL {
        let url = format!(
            "file:fleet_placement_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    async fn create_workflow(dal: &DAL) -> UniversalUuid {
        dal.workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "placement_workflow".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap()
            .id
    }

    /// A `Ready` task with the pool and requests the planner would write.
    async fn create_task(
        dal: &DAL,
        workflow_id: UniversalUuid,
        name: &str,
        pool: Option<&str>,
        memory: Option<i64>,
    ) -> UniversalUuid {
        let id = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: workflow_id,
                task_name: name.into(),
                status: "Ready".into(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap()
            .id;
        let pool = pool.map(str::to_string);
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(task_executions::table.find(id))
                .set((
                    task_executions::pool.eq(pool),
                    task_executions::memory_request_bytes.eq(memory),
                    task_executions::cpu_request_millis.eq(memory.map(|_| 1000i64)),
                ))
                .execute(conn)
        })
        .unwrap();
        id
    }

    const GI: i64 = 1 << 30;

    #[tokio::test]
    async fn a_new_task_row_is_on_no_agent_and_has_the_columns() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let id = create_task(&dal, wf, "t", Some("large"), Some(24 * GI)).await;
        let row = dal.task_execution().get_by_id(id).await.unwrap();
        assert_eq!(row.pool.as_deref(), Some("large"));
        assert_eq!(row.memory_request_bytes, Some(24 * GI));
        assert_eq!(row.cpu_request_millis, Some(1000));
        assert_eq!(row.fleet_agent_id, None);
        assert!(
            dal.task_execution()
                .fleet_placements()
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn only_the_claim_holder_places_a_task() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let id = create_task(&dal, wf, "t", None, Some(2 * GI)).await;
        let me = UniversalUuid::new_v4();
        let other = UniversalUuid::new_v4();

        // No claim: nobody places it.
        assert!(
            !dal.task_execution()
                .place_on_fleet_agent(id, me, "a1", Some(1000), Some(2 * GI as u64))
                .await
                .unwrap()
        );

        dal.task_execution().claim_for_runner(id, me).await.unwrap();
        assert!(
            !dal.task_execution()
                .place_on_fleet_agent(id, other, "a1", Some(1000), Some(2 * GI as u64))
                .await
                .unwrap()
        );
        assert!(
            dal.task_execution()
                .place_on_fleet_agent(id, me, "a1", Some(1000), Some(2 * GI as u64))
                .await
                .unwrap()
        );

        let placed = dal.task_execution().fleet_placements().await.unwrap();
        assert_eq!(
            placed,
            vec![FleetPlacement {
                task_id: id,
                agent_id: "a1".to_string(),
                cpu_request_millis: Some(1000),
                memory_request_bytes: Some(2 * GI as u64),
            }]
        );
    }

    /// A placement counts only while the dispatch holds its claim and the
    /// task has not ended. Nothing has to be cleared for it to stop counting.
    #[tokio::test]
    async fn a_placement_stops_counting_when_the_claim_or_the_task_ends() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let me = UniversalUuid::new_v4();
        let mut ids = Vec::new();
        for name in ["released", "completed", "running", "cleared"] {
            let id = create_task(&dal, wf, name, None, Some(GI)).await;
            dal.task_execution().claim_for_runner(id, me).await.unwrap();
            assert!(
                dal.task_execution()
                    .place_on_fleet_agent(id, me, "a1", Some(1000), Some(2 * GI as u64))
                    .await
                    .unwrap()
            );
            ids.push(id);
        }
        assert_eq!(
            dal.task_execution().fleet_placements().await.unwrap().len(),
            4
        );

        // The dispatch of the first ended without a result: claim released.
        dal.task_execution()
            .release_runner_claim(ids[0])
            .await
            .unwrap();
        // The second completed (its claim is still on the row).
        dal.task_execution()
            .mark_completed(ids[1], None)
            .await
            .unwrap();
        // The third is running on the agent.
        dal.task_execution().mark_started(ids[2]).await.unwrap();
        // The fourth lost a placement race and was taken back.
        dal.task_execution()
            .clear_fleet_placement(ids[3])
            .await
            .unwrap();
        dal.task_execution()
            .clear_fleet_placement(ids[3])
            .await
            .unwrap();

        let placed = dal.task_execution().fleet_placements().await.unwrap();
        assert_eq!(placed.len(), 1, "{placed:?}");
        assert_eq!(placed[0].task_id, ids[2]);
    }

    /// Two replicas (two DAL handles on one database) see each other's
    /// placements: the sum for an agent is the same wherever it is read.
    #[tokio::test]
    async fn placements_are_visible_across_dal_handles() {
        let url = format!(
            "file:fleet_placement_shared_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations().await.unwrap();
        let replica_a = DAL::new(db);
        let replica_b = DAL::new(Database::new(&url, "", 5));
        let wf = create_workflow(&replica_a).await;
        let (runner_a, runner_b) = (UniversalUuid::new_v4(), UniversalUuid::new_v4());

        let first = create_task(&replica_a, wf, "first", Some("large"), Some(24 * GI)).await;
        let second = create_task(&replica_a, wf, "second", Some("large"), Some(24 * GI)).await;
        replica_a
            .task_execution()
            .claim_for_runner(first, runner_a)
            .await
            .unwrap();
        replica_b
            .task_execution()
            .claim_for_runner(second, runner_b)
            .await
            .unwrap();
        assert!(
            replica_a
                .task_execution()
                .place_on_fleet_agent(first, runner_a, "big-1", Some(1000), Some(24 * GI as u64))
                .await
                .unwrap()
        );
        assert!(
            replica_b
                .task_execution()
                .place_on_fleet_agent(second, runner_b, "big-1", Some(1000), Some(24 * GI as u64))
                .await
                .unwrap()
        );

        for replica in [&replica_a, &replica_b] {
            let sum: u64 = replica
                .task_execution()
                .fleet_placements()
                .await
                .unwrap()
                .iter()
                .filter(|p| p.agent_id == "big-1")
                .filter_map(|p| p.memory_request_bytes)
                .sum();
            assert_eq!(sum, 48 * GI as u64);
        }
    }

    #[tokio::test]
    async fn waiting_tasks_are_grouped_by_pool() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        assert!(
            dal.task_execution()
                .waiting_task_stats_by_pool()
                .await
                .unwrap()
                .is_empty()
        );

        create_task(&dal, wf, "plain-1", None, None).await;
        create_task(&dal, wf, "plain-2", Some("default"), None).await;
        let big = create_task(&dal, wf, "big-1", Some("large"), Some(24 * GI)).await;
        create_task(&dal, wf, "big-2", Some("large"), Some(24 * GI)).await;
        create_task(&dal, wf, "big-3", Some("large"), Some(24 * GI)).await;

        // The oldest `large` task has waited ten minutes.
        let since = UniversalTimestamp(chrono::Utc::now() - chrono::Duration::seconds(600));
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(task_executions::table.find(big))
                .set(task_executions::waiting_since.eq(Some(since)))
                .execute(conn)
        })
        .unwrap();

        let stats = dal
            .task_execution()
            .waiting_task_stats_by_pool()
            .await
            .unwrap();
        let pools: Vec<(&str, i64)> = stats
            .iter()
            .map(|p| (p.pool.as_str(), p.waiting.count))
            .collect();
        assert_eq!(pools, vec![("default", 2), ("large", 3)]);
        let now = chrono::Utc::now();
        let age = |pool: &str| {
            stats
                .iter()
                .find(|p| p.pool == pool)
                .unwrap()
                .waiting
                .oldest_age_seconds(now)
        };
        assert!((600.0..660.0).contains(&age("large")), "{}", age("large"));
        assert!((0.0..60.0).contains(&age("default")), "{}", age("default"));

        // The sum over the pools is the figure with no grouping.
        let total = dal.task_execution().waiting_task_stats().await.unwrap();
        assert_eq!(total.count, 5);
    }

    /// CLOACINA-T-1123: the waiting tasks of a pool are grouped by the
    /// workflow priority on their rows; the sum over the priorities is the
    /// pool's figure.
    #[tokio::test]
    async fn waiting_tasks_are_grouped_by_pool_and_workflow_priority() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let urgent = create_task(&dal, wf, "urgent", None, None).await;
        create_task(&dal, wf, "plain-1", None, None).await;
        create_task(&dal, wf, "plain-2", Some("default"), None).await;
        let low = create_task(&dal, wf, "low", Some("large"), None).await;
        async fn set_priority(dal: &DAL, id: UniversalUuid, priority: i32) {
            crate::interact_on_backend!(dal, |conn| {
                diesel::update(task_executions::table.find(id))
                    .set(task_executions::workflow_priority.eq(priority))
                    .execute(conn)
            })
            .unwrap();
        }
        set_priority(&dal, urgent, 0).await;
        set_priority(&dal, low, 9).await;
        let since = UniversalTimestamp(chrono::Utc::now() - chrono::Duration::seconds(600));
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(task_executions::table.find(urgent))
                .set(task_executions::waiting_since.eq(Some(since)))
                .execute(conn)
        })
        .unwrap();

        let stats = dal
            .task_execution()
            .waiting_task_stats_by_pool_and_priority()
            .await
            .unwrap();
        let groups: Vec<(&str, u8, i64)> = stats
            .iter()
            .map(|g| (g.pool.as_str(), g.priority, g.waiting.count))
            .collect();
        assert_eq!(
            groups,
            vec![("default", 0, 1), ("default", 5, 2), ("large", 9, 1)]
        );
        let now = chrono::Utc::now();
        assert!((600.0..660.0).contains(&stats[0].waiting.oldest_age_seconds(now)));
        assert!((0.0..60.0).contains(&stats[1].waiting.oldest_age_seconds(now)));

        let by_pool = dal
            .task_execution()
            .waiting_task_stats_by_pool()
            .await
            .unwrap();
        let pools: Vec<(&str, i64)> = by_pool
            .iter()
            .map(|p| (p.pool.as_str(), p.waiting.count))
            .collect();
        assert_eq!(pools, vec![("default", 3), ("large", 1)]);
        assert!((600.0..660.0).contains(&by_pool[0].waiting.oldest_age_seconds(now)));
    }

    /// CLOACINA-T-1029: the waiting work of each pool, grouped by requests.
    /// A placed task and a started one are not waiting; a task with no pool
    /// is in `default`; a task put off for capacity is still waiting.
    #[tokio::test]
    async fn waiting_demand_is_grouped_by_pool_and_requests() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        assert!(
            dal.task_execution()
                .waiting_demand_by_pool()
                .await
                .unwrap()
                .is_empty()
        );
        create_task(&dal, wf, "plain-1", None, None).await;
        create_task(&dal, wf, "plain-2", Some("default"), None).await;
        for i in 0..3 {
            create_task(&dal, wf, &format!("small-{i}"), Some("large"), Some(2 * GI)).await;
        }
        let deferred = create_task(&dal, wf, "deferred", Some("large"), Some(2 * GI)).await;
        let later = UniversalTimestamp(chrono::Utc::now() + chrono::Duration::seconds(2));
        assert!(
            dal.task_execution()
                .defer_dispatch(deferred, later)
                .await
                .unwrap()
        );
        create_task(&dal, wf, "big", Some("large"), Some(24 * GI)).await;
        let placed = create_task(&dal, wf, "placed", Some("large"), Some(24 * GI)).await;
        let me = UniversalUuid::new_v4();
        dal.task_execution()
            .claim_for_runner(placed, me)
            .await
            .unwrap();
        assert!(
            dal.task_execution()
                .place_on_fleet_agent(placed, me, "a1", Some(1000), Some(24 * GI as u64))
                .await
                .unwrap()
        );
        let started = create_task(&dal, wf, "started", Some("large"), Some(2 * GI)).await;
        dal.task_execution().mark_started(started).await.unwrap();

        let demand = dal.task_execution().waiting_demand_by_pool().await.unwrap();
        let got: Vec<(&str, Option<u64>, u64)> = demand
            .iter()
            .map(|d| (d.pool.as_str(), d.memory_request_bytes, d.count))
            .collect();
        assert_eq!(
            got,
            vec![
                ("default", None, 2),
                ("large", Some(2 * GI as u64), 4),
                ("large", Some(24 * GI as u64), 1),
            ]
        );
    }

    /// A task put off for capacity stays in its pool's count; a started task
    /// leaves it.
    #[tokio::test]
    async fn a_deferred_task_stays_in_its_pool_count_and_a_started_one_leaves() {
        let dal = unique_dal().await;
        let wf = create_workflow(&dal).await;
        let waiting = create_task(&dal, wf, "waits", Some("large"), Some(24 * GI)).await;
        let runs = create_task(&dal, wf, "runs", Some("large"), Some(24 * GI)).await;

        let later = UniversalTimestamp(chrono::Utc::now() + chrono::Duration::seconds(2));
        assert!(
            dal.task_execution()
                .defer_dispatch(waiting, later)
                .await
                .unwrap()
        );
        dal.task_execution().mark_started(runs).await.unwrap();

        let stats = dal
            .task_execution()
            .waiting_task_stats_by_pool()
            .await
            .unwrap();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].pool, "large");
        assert_eq!(stats[0].waiting.count, 1);
    }
}
