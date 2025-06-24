/*
 *  Copyright 2025 Colliery Software
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

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoredRunnerConfig {
    pub name: String,
    pub db_path: String,
    pub max_concurrent_tasks: i32,
    pub enable_cron_scheduling: bool,
    pub enable_registry_reconciler: bool,
    // Advanced configuration options
    pub cron_poll_interval: Option<i32>,
    pub cron_recovery_interval: Option<i32>,
    pub cron_lost_threshold: Option<i32>,
    pub registry_reconcile_interval: Option<i32>,
    pub executor_poll_interval: Option<i32>,
    pub scheduler_poll_interval: Option<i32>,
    pub task_timeout: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RunnerRecord {
    pub id: String,
    pub config: StoredRunnerConfig,
    pub is_paused: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RunnerConfig {
    pub name: String,
    pub db_path: String,
    pub max_concurrent_tasks: i32,
    pub enable_cron_scheduling: bool,
    pub enable_registry_reconciler: bool,
    // Advanced configuration options
    pub cron_poll_interval: Option<i32>,
    pub cron_recovery_interval: Option<i32>,
    pub cron_lost_threshold: Option<i32>,
    pub registry_reconcile_interval: Option<i32>,
    pub executor_poll_interval: Option<i32>,
    pub scheduler_poll_interval: Option<i32>,
    pub task_timeout: Option<i32>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            name: "Default Runner".to_string(),
            db_path: "./workflows.db".to_string(),
            max_concurrent_tasks: 8,
            enable_cron_scheduling: true,
            enable_registry_reconciler: true,
            // Advanced configuration options with defaults
            cron_poll_interval: Some(30),
            cron_recovery_interval: Some(5),
            cron_lost_threshold: Some(10),
            registry_reconcile_interval: Some(60),
            executor_poll_interval: Some(100),
            scheduler_poll_interval: Some(100),
            task_timeout: Some(5),
        }
    }
}

#[derive(Serialize)]
pub struct LocalRunnerStatus {
    pub id: String,
    pub running: bool,
    pub is_paused: bool,
    pub config: StoredRunnerConfig,
    pub message: String,
}

impl From<RunnerConfig> for StoredRunnerConfig {
    fn from(config: RunnerConfig) -> Self {
        Self {
            name: config.name,
            db_path: config.db_path,
            max_concurrent_tasks: config.max_concurrent_tasks,
            enable_cron_scheduling: config.enable_cron_scheduling,
            enable_registry_reconciler: config.enable_registry_reconciler,
            cron_poll_interval: config.cron_poll_interval,
            cron_recovery_interval: config.cron_recovery_interval,
            cron_lost_threshold: config.cron_lost_threshold,
            registry_reconcile_interval: config.registry_reconcile_interval,
            executor_poll_interval: config.executor_poll_interval,
            scheduler_poll_interval: config.scheduler_poll_interval,
            task_timeout: config.task_timeout,
        }
    }
}
