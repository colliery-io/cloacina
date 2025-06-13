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

use crate::cli::Cli;

#[derive(Debug)]
pub enum LogLevel {
    #[allow(dead_code)]
    Error,
    Info,
    Debug,
}

pub fn init_logging(cli: &Cli) {
    let level = if cli.quiet {
        "error"
    } else if cli.verbose {
        "debug"
    } else {
        "info"
    };

    std::env::set_var("RUST_LOG", level);
    // Simple print-based logging for now
}

pub fn should_print(cli: &Cli, level: LogLevel) -> bool {
    match level {
        LogLevel::Error => true, // Always print errors
        LogLevel::Info => !cli.quiet,
        LogLevel::Debug => cli.verbose && !cli.quiet,
    }
}
