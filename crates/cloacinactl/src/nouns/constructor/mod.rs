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

//! `cloacinactl constructor <verb>` — author-side distribution of `#[constructor]`
//! crates as fidius **provider packages** (CLOACI-T-0827).
//!
//! `constructor package <crate-dir>` builds the crate to a `wasm32-wasip2`
//! component, emits its `constructor.json` from the macro-generated
//! `__constructor_manifest()`, assembles a fidius `runtime = "wasm"` provider
//! package, optionally Ed25519-signs it, and packs it into a distributable
//! `<name>-<version>.cloacina` archive — the constructor analogue of
//! `cloacinactl package pack`.

use std::path::PathBuf;

use clap::{Args, Subcommand};

use cloacina::packaging::constructor_provider::{
    package_constructor_provider, ProviderPackageOptions,
};

use crate::shared::error::CliError;
use crate::GlobalOpts;

#[derive(Args)]
pub struct ConstructorCmd {
    #[command(subcommand)]
    verb: ConstructorVerb,
}

#[derive(Subcommand)]
enum ConstructorVerb {
    /// Build + assemble + (optionally sign) + pack a `#[constructor]` crate into a
    /// distributable `.cloacina` provider package.
    Package {
        /// The constructor crate directory (contains Cargo.toml + the
        /// `#[constructor]` lib + a manifest-emitter bin).
        crate_dir: PathBuf,
        /// Output archive path (default: `<name>-<version>.cloacina` in the CWD).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Sign the package with this Ed25519 secret-key file (32 raw bytes).
        #[arg(long = "sign-key")]
        sign_key: Option<PathBuf>,
        /// Host binary in the crate that prints the constructor manifest JSON
        /// (the `__constructor_manifest()` emitter).
        #[arg(long, default_value = "emit_manifest")]
        manifest_bin: String,
        /// Build the wasm component in debug profile (default is release).
        #[arg(long)]
        debug: bool,
    },
}

impl ConstructorCmd {
    pub async fn run(self, _globals: &GlobalOpts) -> Result<(), CliError> {
        match self.verb {
            ConstructorVerb::Package {
                crate_dir,
                out,
                sign_key,
                manifest_bin,
                debug,
            } => {
                let opts = ProviderPackageOptions {
                    crate_dir,
                    output: out,
                    sign_key,
                    manifest_bin,
                    release: !debug,
                };
                let result = package_constructor_provider(&opts)
                    .map_err(|e| CliError::UserError(e.to_string()))?;

                let signed = if result.signed { "signed" } else { "unsigned" };
                eprintln!(
                    "Packaged provider '{}' ({signed}) carrying {} constructor(s): {}",
                    result.provider_name,
                    result.constructors.len(),
                    result.constructors.join(", "),
                );
                println!("{}", result.archive.display());
                Ok(())
            }
        }
    }
}
