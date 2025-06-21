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

pub mod archive;
pub mod cli;
pub mod commands;
pub mod config;
pub mod database;
pub mod library;
pub mod manifest;
pub mod server;
pub mod utils;
pub mod validation;
pub mod visualization;

// Re-export commonly used types and functions
pub use cli::{Cli, Commands, DebugAction};
pub use commands::*;
pub use manifest::{CompileResult, PackageManifest, TaskInfo};
pub use utils::{init_logging, should_print, LogLevel};
