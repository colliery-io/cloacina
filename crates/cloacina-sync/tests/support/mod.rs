/*
 *  Copyright 2025-2026 Colliery Software
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

//! Shared by the Sync connector tests: the conformance kit's host over the
//! real connector host, and SQL Server seeding for the MSSQL tests.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use cloacina::registry::loader::connector_host::{ConnectorHandle, load_connector};
use cloacina::registry::loader::grants::{ResolvedGrants, translate};
use cloacina_constructor_contract::sync::{
    CheckResult, Config, ConnectorSpec, DiscoverOutcome, ReadContext, RecordBatch, WriteContext,
    WriteOutcome,
};
use cloacina_sync::credential::{BuildCredential, CredentialSpec};
use cloacina_sync::workflow::grant_spec_from_json;
use cloacina_testing::connectors::conformance::{Connector, Credential, Host, ReadIter, Service};
use cloacina_testing::connectors::{crate_name, stage_connector};
use serde_json::{Map, Value};

pub mod minio;
pub mod mssql;

/// A path in the repository.
pub fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// The kit's host over cloacina's connector host: stages each crate under one
/// directory per test process and loads members from there.
pub struct ConnectorHost {
    pub root: PathBuf,
}

impl ConnectorHost {
    pub fn new(tag: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("cloacina-conformance-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }
}

impl Host for ConnectorHost {
    fn prepare(&self, crate_dir: &Path) -> Result<String, String> {
        stage_connector(&self.root, crate_dir);
        Ok(crate_name(crate_dir))
    }

    fn load(
        &self,
        package: &str,
        member: &str,
        config: &Config,
        grants: Option<&Value>,
        credential: Option<&Credential>,
    ) -> Result<Box<dyn Connector>, String> {
        let mut grants = match grants {
            None => ResolvedGrants::deny_all(),
            Some(v) => translate(&grant_spec_from_json(v)?).map_err(|e| e.to_string())?,
        };
        // The credential rides on the egress, as the Sync tasks put it there.
        if let Some(c) = credential {
            let side = if member == "destination" {
                "destination"
            } else {
                "source"
            };
            let spec: CredentialSpec = serde_json::from_value(c.spec.clone())
                .map_err(|e| format!("credential spec: {e}"))?;
            grants =
                grants.with_credential(spec.build(side, &c.secret).map_err(|e| e.to_string())?);
        }
        load_connector(&self.root, package, member, config, &grants)
            .map(|h| Box::new(Handle(h)) as Box<dyn Connector>)
            .map_err(|e| e.to_string())
    }
}

pub struct Handle(pub ConnectorHandle);

impl Connector for Handle {
    fn spec(&self) -> Result<ConnectorSpec, String> {
        self.0.spec().map_err(|e| e.to_string())
    }

    fn check(&self, config: &Config) -> Result<CheckResult, String> {
        self.0.check(config).map_err(|e| e.to_string())
    }

    fn discover(&self, config: &Config) -> Result<DiscoverOutcome, String> {
        self.0.discover(config).map_err(|e| e.to_string())
    }

    fn read(&self, ctx: &ReadContext) -> Result<ReadIter<'_>, String> {
        let iter = self.0.read_blocking(ctx).map_err(|e| e.to_string())?;
        Ok(Box::new(iter.map(|m| m.map_err(|e| e.to_string()))))
    }

    fn write(&self, ctx: &WriteContext, batches: Vec<RecordBatch>) -> Result<WriteOutcome, String> {
        self.0.write(ctx, batches).map_err(|e| e.to_string())
    }
}
