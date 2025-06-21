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
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

pub async fn show_logs(lines: usize, follow: bool, level: Option<String>) -> Result<()> {
    // TODO: Read actual log file location from config
    let log_file = PathBuf::from("/var/log/cloacina/cloacina.log");

    if !log_file.exists() {
        println!(
            "{} Log file not found: {}",
            "⚠".yellow().bold(),
            log_file.display()
        );
        return Ok(());
    }

    if follow {
        tail_logs(&log_file, lines, level).await
    } else {
        show_recent_logs(&log_file, lines, level).await
    }
}

async fn show_recent_logs(log_file: &PathBuf, lines: usize, level: Option<String>) -> Result<()> {
    let file = File::open(log_file)
        .with_context(|| format!("Failed to open log file: {}", log_file.display()))?;

    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to read log file")?;

    let filtered_lines = filter_by_level(&all_lines, level.as_deref());
    let recent_lines = filtered_lines
        .iter()
        .rev()
        .take(lines)
        .rev()
        .collect::<Vec<_>>();

    for line in recent_lines {
        println!("{}", format_log_line(line));
    }

    Ok(())
}

async fn tail_logs(log_file: &PathBuf, initial_lines: usize, level: Option<String>) -> Result<()> {
    println!(
        "{} Following logs from {} (Ctrl+C to stop)",
        "→".cyan().bold(),
        log_file.display()
    );

    // Show initial lines
    show_recent_logs(log_file, initial_lines, level.clone()).await?;

    // TODO: Implement proper file watching for new lines
    // This is a simplified implementation that just polls the file
    let mut last_position = {
        let file = File::open(log_file)?;
        let metadata = file.metadata()?;
        metadata.len()
    };

    loop {
        sleep(Duration::from_millis(500)).await;

        let mut file = File::open(log_file)?;
        let metadata = file.metadata()?;
        let current_size = metadata.len();

        if current_size > last_position {
            file.seek(SeekFrom::Start(last_position))?;
            let reader = BufReader::new(file);

            for line in reader.lines() {
                let line = line?;
                if should_include_line(&line, level.as_deref()) {
                    println!("{}", format_log_line(&line));
                }
            }

            last_position = current_size;
        }
    }
}

fn filter_by_level<'a>(lines: &'a [String], level_filter: Option<&str>) -> Vec<&'a String> {
    if let Some(filter) = level_filter {
        lines
            .iter()
            .filter(|line| should_include_line(line, Some(filter)))
            .collect()
    } else {
        lines.iter().collect()
    }
}

fn should_include_line(line: &str, level_filter: Option<&str>) -> bool {
    if let Some(filter) = level_filter {
        let filter_upper = filter.to_uppercase();
        line.to_uppercase().contains(&filter_upper)
    } else {
        true
    }
}

fn format_log_line(line: &str) -> String {
    // Basic log line formatting with color coding
    if line.contains("ERROR") {
        line.red().to_string()
    } else if line.contains("WARN") {
        line.yellow().to_string()
    } else if line.contains("INFO") {
        line.normal().to_string()
    } else if line.contains("DEBUG") {
        line.blue().to_string()
    } else if line.contains("TRACE") {
        line.bright_black().to_string()
    } else {
        line.to_string()
    }
}
