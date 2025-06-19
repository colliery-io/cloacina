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

pub mod config;
pub mod daemon;
pub mod logs;
pub mod process;

use crate::cli::ServerCommands;
use anyhow::Result;

pub async fn handle_server_command(command: ServerCommands) -> Result<()> {
    match command {
        ServerCommands::Config(config_cmd) => config::handle_config_command(config_cmd).await,
        ServerCommands::Start {
            config,
            foreground,
            database_url,
        } => daemon::start_server(config, foreground, database_url).await,
        ServerCommands::Stop { force, timeout } => daemon::stop_server(force, timeout).await,
        ServerCommands::Status { format } => process::show_status(&format).await,
        ServerCommands::Restart {
            config,
            force,
            timeout,
        } => daemon::restart_server(config, force, timeout).await,
        ServerCommands::Logs {
            lines,
            follow,
            level,
        } => logs::show_logs(lines, follow, level).await,
    }
}
