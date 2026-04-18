/*
 *  Copyright 2026 Colliery Software
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

//! Generic JSON-value renderers for list/object responses. Table mode
//! auto-infers columns from the first object's keys — good enough for the v1
//! catalog-style listings; can be replaced with per-type renderers later.

use serde_json::Value;

use crate::shared::error::CliError;
use crate::OutputFormat;

pub fn list(body: &Value, format: OutputFormat) -> Result<(), CliError> {
    let items: Vec<Value> = body.as_array().cloned().unwrap_or_default();
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&items)
                    .map_err(|e| CliError::UserError(e.to_string()))?
            );
        }
        OutputFormat::Yaml => {
            print!(
                "{}",
                serde_yaml::to_string(&items).map_err(|e| CliError::UserError(e.to_string()))?
            );
        }
        OutputFormat::Id => {
            for item in &items {
                if let Some(id) = item
                    .get("id")
                    .or_else(|| item.get("name"))
                    .and_then(|v| v.as_str())
                {
                    println!("{id}");
                }
            }
        }
        OutputFormat::Table => table(&items)?,
    }
    Ok(())
}

pub fn object(body: &Value, format: OutputFormat) -> Result<(), CliError> {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(body)
                    .map_err(|e| CliError::UserError(e.to_string()))?
            );
        }
        OutputFormat::Yaml => {
            print!(
                "{}",
                serde_yaml::to_string(body).map_err(|e| CliError::UserError(e.to_string()))?
            );
        }
        OutputFormat::Id => {
            if let Some(id) = body
                .get("id")
                .or_else(|| body.get("name"))
                .and_then(|v| v.as_str())
            {
                println!("{id}");
            }
        }
        OutputFormat::Table => {
            if let Some(map) = body.as_object() {
                let width = map.keys().map(|k| k.len()).max().unwrap_or(0);
                for (k, v) in map {
                    let rendered = match v {
                        Value::String(s) => s.clone(),
                        _ => serde_json::to_string(v).unwrap_or_default(),
                    };
                    println!("{:<width$}  {}", k, rendered, width = width);
                }
            } else {
                println!("{body}");
            }
        }
    }
    Ok(())
}

fn table(items: &[Value]) -> Result<(), CliError> {
    if items.is_empty() {
        println!("No items.");
        return Ok(());
    }
    // Infer columns from the first object's top-level keys.
    let first = items[0].as_object().cloned().unwrap_or_default();
    let columns: Vec<String> = first.keys().cloned().collect();
    let widths: Vec<usize> = columns.iter().map(|c| c.len().max(8)).collect();

    // Header
    for (c, w) in columns.iter().zip(&widths) {
        print!("{:<w$}  ", c.to_uppercase(), w = *w);
    }
    println!();

    for item in items {
        for (c, w) in columns.iter().zip(&widths) {
            let cell = item
                .get(c)
                .map(|v| match v {
                    Value::String(s) => truncate(s, *w),
                    _ => truncate(&serde_json::to_string(v).unwrap_or_default(), *w),
                })
                .unwrap_or_else(|| "".into());
            print!("{:<w$}  ", cell, w = *w);
        }
        println!();
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
