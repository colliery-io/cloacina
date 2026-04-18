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
//! inspect / delete.

use clap::{Args, Subcommand};
use std::path::PathBuf;

use crate::shared::error::CliError;
use crate::GlobalOpts;

pub mod build;
pub mod delete;
pub mod inspect;
pub mod list;
pub mod pack;
pub mod publish;
pub mod upload;

#[derive(Args)]
pub struct PackageCmd {
    #[command(subcommand)]
    verb: PackageVerb,
}

#[derive(Subcommand)]
enum PackageVerb {
    /// cargo build the package source directory.
    Build {
        dir: PathBuf,
        /// Build in release profile (default is debug).
        #[arg(long)]
        release: bool,
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
            PackageVerb::Build { dir, release } => build::run(&dir, release),
            PackageVerb::Pack { dir, out, sign } => {
                pack::run(&dir, out.as_deref(), sign.as_deref())
            }
            PackageVerb::Publish { dir, release, sign } => {
                publish::run(globals, &dir, release, sign.as_deref()).await
            }
            PackageVerb::Upload { file } => upload::run(globals, &file).await,
            PackageVerb::List { filter } => list::run(globals, filter.as_deref()).await,
            PackageVerb::Inspect { id } => inspect::run(globals, &id).await,
            PackageVerb::Delete { id, force } => delete::run(globals, &id, force).await,
        }
    }
}
