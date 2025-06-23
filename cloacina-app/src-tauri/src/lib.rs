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

mod domains;

use domains::app::commands::*;
use domains::app::logging::*;
use domains::app::settings::*;
use domains::packages::commands::*;
use domains::runners::commands::*;

// Legacy commands for debugging
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn test_cloacina() -> Result<String, String> {
    use cloacina::runner::DefaultRunnerConfig;
    let config = DefaultRunnerConfig::default();
    Ok(format!(
        "Cloacina integration working! Default config: max_concurrent_tasks = {}",
        config.max_concurrent_tasks
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging system
    if let Err(e) = initialize_logging() {
        eprintln!("Failed to initialize logging: {}", e);
        // Fall back to env_logger
        env_logger::init();
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            test_cloacina,
            initialize_app,
            create_runner,
            get_local_runners,
            start_local_runner,
            stop_local_runner,
            delete_runner,
            get_settings,
            save_settings,
            get_data_directory,
            get_runner_db_path,
            get_full_path,
            select_database_folder,
            change_database_location,
            get_log_files,
            get_runner_log_files,
            get_runner_log_files_for_runner,
            read_runner_log_file,
            get_active_runner_loggers,
            get_runner_logs_from_main_log,
            get_all_runner_activity_from_logs,
            open_log_directory,
            reload_logging_config,
            generate_reset_confirmation,
            full_system_reset,
            // Package management commands
            build_package,
            inspect_package,
            debug_package,
            visualize_package,
            // Dialog commands
            select_directory_dialog,
            save_file_dialog,
            select_file_dialog,
            open_file_location,
            get_desktop_path,
            get_system_paths
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
