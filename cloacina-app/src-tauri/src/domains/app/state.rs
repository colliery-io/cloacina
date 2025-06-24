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

use cloacina::runner::DefaultRunner;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::domains::runners::RunnerService;

pub struct AppState {
    pub runners: HashMap<String, Arc<DefaultRunner>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            runners: HashMap::new(),
        }
    }
}

// Global application state
pub static APP_STATE: Lazy<Mutex<AppState>> = Lazy::new(|| Mutex::new(AppState::new()));

// Global runner service - initialized once and never changed
pub static RUNNER_SERVICE: Lazy<Mutex<Option<Arc<RunnerService>>>> = Lazy::new(|| Mutex::new(None));
