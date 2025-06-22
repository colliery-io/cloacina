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
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            name: "Default Runner".to_string(),
            db_path: "./workflows.db".to_string(),
            max_concurrent_tasks: 4,
            enable_cron_scheduling: false,
            enable_registry_reconciler: true,
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
        }
    }
}
