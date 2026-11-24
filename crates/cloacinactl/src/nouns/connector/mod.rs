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

//! `cloacinactl connector` — Sync's declarative connectors
//! (COLLIERY-I-0634 / CLOACINA-T-0992).
//!
//! `list` shows the catalog: the connectors a server can run, compiled and
//! declarative (CLOACINA-T-0993). `import` turns an Airbyte low-code source
//! into a cloacina connector definition, offline, and reports what the REST
//! runtime supports of it and what it does not. The result is the YAML a
//! server loads from its definitions directory.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use cloacina_declarative::importer::{analyze, import_yaml};

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::render;
use crate::{GlobalOpts, OutputFormat};

#[derive(Args)]
pub struct ConnectorCmd {
    #[command(subcommand)]
    verb: ConnectorVerb,
}

#[derive(Subcommand)]
enum ConnectorVerb {
    /// The connectors the server can run: compiled providers and declarative
    /// definitions, with their roles and streams.
    List,
    /// Import an Airbyte low-code source (`type: DeclarativeSource`) as a
    /// connector definition. The definition goes to stdout (or `--out`); the
    /// report of what the runtime supports goes to stderr.
    Import {
        /// The Airbyte YAML file.
        file: PathBuf,
        /// The definition's name. Default: the file's stem.
        #[arg(long)]
        name: Option<String>,
        /// Write the definition here instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

impl ConnectorCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<()> {
        match self.verb {
            ConnectorVerb::Import { file, name, out } => import(globals, file, name, out),
            ConnectorVerb::List => {
                let config = CloacinaConfig::load(&globals.home.join("config.toml"));
                let ctx = ClientContext::resolve(globals, &config)?;
                let output = ctx.output;
                let client = CliClient::new(ctx).map_err(|e| anyhow::anyhow!("{e}"))?;
                let tenant = client.ctx().tenant_segment().to_string();
                let body: serde_json::Value = client
                    .get(&format!("/v1/tenants/{tenant}/connectors"))
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                render::list(&body, output).map_err(|e| anyhow::anyhow!("{e}"))
            }
        }
    }
}

fn import(
    globals: &GlobalOpts,
    file: PathBuf,
    name: Option<String>,
    out: Option<PathBuf>,
) -> Result<()> {
    let yaml =
        std::fs::read_to_string(&file).with_context(|| format!("read {}", file.display()))?;
    let name = name.unwrap_or_else(|| {
        file.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "connector".to_string())
    });
    let report = analyze(&yaml);
    let definition = import_yaml(&name, &yaml);
    if matches!(globals.effective_output(), OutputFormat::Json) {
        let body = serde_json::json!({
            "name": name,
            "report": report,
            "definition": definition.as_ref().ok(),
            "error": definition.as_ref().err().map(|e| e.to_string()),
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
        if definition.is_err() {
            bail!("the import failed");
        }
        return Ok(());
    }
    eprintln!(
        "{name}: tier {} (confidence {:.2}), streams: {}",
        report.tier,
        report.confidence,
        report.streams.join(", ")
    );
    for gap in &report.unsupported {
        eprintln!("  not supported: {gap}");
    }
    let definition = definition.with_context(|| format!("import {}", file.display()))?;
    let text = serde_yaml::to_string(&definition)?;
    match out {
        Some(path) => {
            std::fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
            eprintln!("wrote {}", path.display());
        }
        None => print!("{text}"),
    }
    Ok(())
}
