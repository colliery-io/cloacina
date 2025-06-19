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

use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use sysinfo::{Pid, Process, System};

#[derive(Debug)]
pub struct ServerStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub uptime: Option<u64>,
    pub memory_usage: Option<u64>,
    pub cpu_usage: Option<f32>,
}

pub async fn show_status(format: &str) -> Result<()> {
    let status = get_server_status().await?;

    match format {
        "json" => {
            let json_output = json!({
                "running": status.running,
                "pid": status.pid,
                "uptime": status.uptime,
                "memory_usage": status.memory_usage,
                "cpu_usage": status.cpu_usage
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        "human" | _ => {
            print_human_status(&status);
        }
    }

    Ok(())
}

async fn get_server_status() -> Result<ServerStatus> {
    // TODO: Read actual PID file location from config
    let pid_file = PathBuf::from("/var/run/cloacina.pid");

    let pid = if pid_file.exists() {
        let pid_content = fs::read_to_string(&pid_file).context("Failed to read PID file")?;

        let pid: u32 = pid_content
            .trim()
            .parse()
            .context("Invalid PID in PID file")?;

        Some(pid)
    } else {
        None
    };

    let mut status = ServerStatus {
        running: false,
        pid,
        uptime: None,
        memory_usage: None,
        cpu_usage: None,
    };

    if let Some(pid) = pid {
        // Check if process is actually running
        let mut system = System::new_all();
        system.refresh_all();

        if let Some(process) = system.process(Pid::from(pid as usize)) {
            status.running = true;
            status.memory_usage = Some(process.memory());
            status.cpu_usage = Some(process.cpu_usage());

            // Calculate uptime (this is simplified)
            // TODO: Implement proper uptime calculation
            status.uptime = Some(60); // Placeholder
        } else {
            // PID file exists but process is not running
            // TODO: Clean up stale PID file
        }
    }

    Ok(status)
}

fn print_human_status(status: &ServerStatus) {
    if status.running {
        println!(
            "{} Cloacina server is {}",
            "✓".green().bold(),
            "running".green().bold()
        );

        if let Some(pid) = status.pid {
            println!("  PID: {}", pid.to_string().cyan());
        }

        if let Some(uptime) = status.uptime {
            println!("  Uptime: {} seconds", uptime.to_string().cyan());
        }

        if let Some(memory) = status.memory_usage {
            println!("  Memory: {} KB", memory.to_string().cyan());
        }

        if let Some(cpu) = status.cpu_usage {
            println!("  CPU: {:.1}%", cpu.to_string().cyan());
        }
    } else {
        println!(
            "{} Cloacina server is {}",
            "✗".red().bold(),
            "not running".red().bold()
        );

        if status.pid.is_some() {
            println!("  {} Stale PID file detected", "⚠".yellow().bold());
        }
    }
}
