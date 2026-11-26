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

//! `cloacinactl package <verb>` — build / pack / publish / upload / list /
//! inspect / versions / activate / delete.

use clap::{Args, Subcommand};
use std::path::PathBuf;

use crate::GlobalOpts;
use crate::shared::error::CliError;

pub mod build;
pub mod delete;
pub mod inspect;
pub mod list;
pub mod manifest;
pub mod new;
pub mod pack;
pub mod publish;
pub mod upload;
pub mod validate;
pub mod vendor;
pub mod versions;

#[derive(Args)]
pub struct PackageCmd {
    #[command(subcommand)]
    verb: PackageVerb,
}

#[derive(Subcommand)]
enum PackageVerb {
    /// Scaffold a new canonical package source tree (Rust or Python).
    New {
        /// Package name (e.g. data-pipeline).
        name: String,
        /// Source language to scaffold.
        #[arg(long, value_enum, default_value_t = new::ScaffoldLang::Python)]
        lang: new::ScaffoldLang,
        /// Package shape: workflow (default), graph, or cron (cron is Rust-only).
        #[arg(long, value_enum, default_value_t = new::ScaffoldKind::Workflow)]
        kind: new::ScaffoldKind,
        /// Directory to create (default: ./<name>).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Build the package source directory. Rust: cargo build. Python: vendor
    /// the dependencies of pyproject.toml into vendor/<platform>/.
    Build {
        dir: PathBuf,
        /// Rust packages: build in release profile (default is debug).
        #[arg(long)]
        release: bool,
        #[command(flatten)]
        vendor: vendor::VendorArgs,
    },
    /// Validate a package (source dir or .cloacina archive) against the
    /// canonical format without uploading.
    Validate {
        /// Package source directory or .cloacina archive.
        path: PathBuf,
        /// Python packages: the Python version of the agents that run the
        /// package. `requires_python` and the vendored dependencies are
        /// checked against it. Default: the version the dependencies were
        /// vendored for, or 3.11.
        #[arg(long, value_name = "MAJOR.MINOR")]
        python_version: Option<String>,
    },
    /// fidius-pack the source directory into a .cloacina archive.
    Pack {
        dir: PathBuf,
        /// Output path (default: <dir>/<name>.cloacina).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Sign the archive with this Ed25519 key file.
        #[arg(long)]
        sign: Option<PathBuf>,
    },
    /// build + pack + upload in one shot.
    Publish {
        dir: PathBuf,
        #[arg(long)]
        release: bool,
        #[arg(long)]
        sign: Option<PathBuf>,
        #[command(flatten)]
        vendor: vendor::VendorArgs,
    },
    /// Upload a pre-packed .cloacina archive.
    Upload { file: PathBuf },
    /// List installed packages.
    List {
        #[arg(long)]
        filter: Option<String>,
    },
    /// Fetch metadata for a single package.
    Inspect { id: String },
    /// List every stored version of a package: the active one, versions that
    /// wait for a build or failed, and superseded ones.
    Versions {
        /// Package name.
        name: String,
    },
    /// Make an earlier built version of a package the active one. No rebuild.
    /// Executions that already run stay on the version they started with.
    Activate {
        /// Package name.
        name: String,
        /// Version to make active (see `package versions`).
        version: String,
    },
    /// Uninstall a package.
    Delete {
        id: String,
        #[arg(long)]
        force: bool,
    },
}

impl PackageCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        match self.verb {
            PackageVerb::New {
                name,
                lang,
                kind,
                path,
            } => new::run(&name, lang, kind, path.as_deref()),
            PackageVerb::Validate {
                path,
                python_version,
            } => validate::run(&path, python_version.as_deref()),
            PackageVerb::Build {
                dir,
                release,
                vendor,
            } => build::run(&dir, release, &vendor),
            PackageVerb::Pack { dir, out, sign } => {
                pack::run(&dir, out.as_deref(), sign.as_deref())
            }
            PackageVerb::Publish {
                dir,
                release,
                sign,
                vendor,
            } => publish::run(globals, &dir, release, sign.as_deref(), &vendor).await,
            PackageVerb::Upload { file } => upload::run(globals, &file).await,
            PackageVerb::List { filter } => list::run(globals, filter.as_deref()).await,
            PackageVerb::Inspect { id } => inspect::run(globals, &id).await,
            PackageVerb::Versions { name } => versions::list(globals, &name).await,
            PackageVerb::Activate { name, version } => {
                versions::activate(globals, &name, &version).await
            }
            PackageVerb::Delete { id, force } => delete::run(globals, &id, force).await,
        }
    }
}
