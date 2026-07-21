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
//! `constructor package <crate-dir>` builds the provider crate (a *suite* of
//! members, CLOACI-A-0011) to a `wasm32-wasip2` component, emits its `provider.json`
//! from the macro-generated `__provider_manifest()`, assembles a fidius
//! `runtime = "wasm"` provider package, optionally Ed25519-signs it, and packs it
//! into a distributable `<name>-<version>.cloacina` archive — the constructor
//! analogue of `cloacinactl package pack`.
//!
//! `--native` (CLOACI-T-0903) packages a NATIVE host cdylib provider instead: the
//! crate is built to a host `cdylib` (no wasm target, `--features native`), the
//! fidius `package.toml` is stamped `runtime = "rust"` (fidius's cdylib runtime —
//! there is no `"native"` there) with no `[wasm]` section, and cloacina's
//! `provider.json` carries `runtime = "native"` so the loader `dlopen`s it
//! in-process (trusted; grants advisory). Same signing + pack path.

use std::path::PathBuf;

use clap::{Args, Subcommand};

use cloacina::packaging::constructor_provider::{
    package_constructor_provider, ProviderPackageOptions, ProviderRuntime,
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
        /// Host binary in the crate that prints the provider manifest JSON
        /// (the `__provider_manifest()` emitter).
        #[arg(long, default_value = "emit_manifest")]
        manifest_bin: String,
        /// Build the component in debug profile (default is release).
        #[arg(long)]
        debug: bool,
        /// Package a NATIVE host cdylib provider (loaded in-process, trusted)
        /// instead of a sandboxed `wasm32-wasip2` component (CLOACI-T-0903). The
        /// crate must declare a `native` cargo feature + `fidius-core`/`fidius-macro`
        /// host deps.
        #[arg(long)]
        native: bool,
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
                native,
            } => {
                let opts = ProviderPackageOptions {
                    crate_dir,
                    output: out,
                    sign_key,
                    manifest_bin,
                    release: !debug,
                    runtime: if native {
                        ProviderRuntime::Native
                    } else {
                        ProviderRuntime::Wasm
                    },
                };
                let result = package_constructor_provider(&opts)
                    .map_err(|e| CliError::UserError(e.to_string()))?;

                let signed = if result.signed { "signed" } else { "unsigned" };
                // CLOACI-T-0907: surface the TRUST TIER — a native provider runs
                // in-process with full host trust (grants advisory); wasm runs
                // sandboxed with grants enforced. Operators should see which
                // tier they just packaged.
                let tier = if native {
                    "native — TRUSTED, runs unsandboxed in-process"
                } else {
                    "wasm — sandboxed, capability grants enforced"
                };
                eprintln!(
                    "Packaged provider '{}' ({signed}, {tier}) carrying {} constructor(s): {}",
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
