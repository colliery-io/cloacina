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
pub mod sign;
pub mod upload;
pub mod validate;
pub mod vendor;
pub mod versions;

/// The unpack limit of `package validate`, `pack` and `publish`
/// (CLOACINA-T-1048). Give the value the server and the agents run with.
#[derive(Debug, Clone, Args)]
pub struct UnpackLimitArgs {
    /// The largest size, in bytes, that a package archive may unpack to. A
    /// larger package is refused. Use the value of the server and the
    /// agents. Default: 524288000 (500 MiB).
    #[arg(
        long,
        env = "CLOACINA_MAX_UNPACKED_BYTES",
        default_value_t = cloacina_workflow_plugin::unpack_limit::DEFAULT_MAX_UNPACKED_BYTES
    )]
    pub max_unpacked_bytes: u64,
}

impl UnpackLimitArgs {
    /// Make the limit the limit of this process.
    fn apply(&self) -> Result<(), CliError> {
        cloacina_workflow_plugin::unpack_limit::set_max_unpacked_bytes(self.max_unpacked_bytes)
            .map_err(CliError::UserError)
    }
}

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
        /// Package shape: workflow (default), graph, or cron.
        #[arg(long, value_enum, default_value_t = new::ScaffoldKind::Workflow)]
        kind: new::ScaffoldKind,
        /// Directory to create (default: ./<name>).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Build the package source directory. Rust: cargo build. Python: lock
    /// the dependencies of pyproject.toml into requirements.lock, and with
    /// --vendor install them into vendor/<platform>/.
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
        #[command(flatten)]
        unpack_limit: UnpackLimitArgs,
    },
    /// fidius-pack the source directory into a .cloacina archive.
    Pack {
        dir: PathBuf,
        /// Output path (default: <dir>/<name>.cloacina).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Sign the archive with this key file and write the signature
        /// beside it, as <archive>.sig. The key file is an unencrypted
        /// Ed25519 private key in PKCS#8 PEM (`openssl genpkey -algorithm
        /// ed25519`) that only its owner can read. The key stays on this
        /// machine.
        #[arg(long, value_name = "KEY_FILE")]
        sign: Option<PathBuf>,
        #[command(flatten)]
        unpack_limit: UnpackLimitArgs,
    },
    /// build + pack + upload in one shot.
    Publish {
        dir: PathBuf,
        #[arg(long)]
        release: bool,
        /// Sign the archive with this key file and send the signature with
        /// the upload. The key file is an unencrypted Ed25519 private key in
        /// PKCS#8 PEM that only its owner can read. Only the signature goes
        /// to the server.
        #[arg(long, value_name = "KEY_FILE")]
        sign: Option<PathBuf>,
        #[command(flatten)]
        vendor: vendor::VendorArgs,
        #[command(flatten)]
        unpack_limit: UnpackLimitArgs,
    },
    /// Upload a pre-packed .cloacina archive. The signature file beside it
    /// (<FILE>.sig, written by `package pack --sign`) is sent with it when
    /// it is there.
    Upload {
        file: PathBuf,
        /// Send this signature file with the archive, in place of <FILE>.sig.
        #[arg(long, value_name = "SIG_FILE")]
        signature: Option<PathBuf>,
    },
    /// List installed packages.
    List {
        #[arg(long)]
        filter: Option<String>,
        /// Only the packages that need a new publish: built for another
        /// plugin interface than the server's release, and not carried over
        /// by a rebuild from their kept source.
        #[arg(long)]
        needs_new_build: bool,
        /// With `--needs-new-build`: the packages of every tenant (platform
        /// admin key).
        #[arg(long, requires = "needs_new_build")]
        all_tenants: bool,
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
                unpack_limit,
            } => {
                unpack_limit.apply()?;
                validate::run(&path, python_version.as_deref())
            }
            PackageVerb::Build {
                dir,
                release,
                vendor,
            } => build::run(&dir, release, &vendor),
            PackageVerb::Pack {
                dir,
                out,
                sign,
                unpack_limit,
            } => {
                unpack_limit.apply()?;
                pack::run(&dir, out.as_deref(), sign.as_deref())
            }
            PackageVerb::Publish {
                dir,
                release,
                sign,
                vendor,
                unpack_limit,
            } => {
                unpack_limit.apply()?;
                publish::run(globals, &dir, release, sign.as_deref(), &vendor).await
            }
            PackageVerb::Upload { file, signature } => {
                let signature = match signature {
                    Some(path) => sign::UploadSignature::File(path),
                    None => sign::UploadSignature::Sidecar,
                };
                upload::run(globals, &file, Some(signature)).await
            }
            PackageVerb::List {
                filter,
                needs_new_build,
                all_tenants,
            } => {
                let only = if all_tenants {
                    list::Only::NeedsNewBuildAllTenants
                } else if needs_new_build {
                    list::Only::NeedsNewBuild
                } else {
                    list::Only::All
                };
                list::run(globals, filter.as_deref(), only).await
            }
            PackageVerb::Inspect { id } => inspect::run(globals, &id).await,
            PackageVerb::Versions { name } => versions::list(globals, &name).await,
            PackageVerb::Activate { name, version } => {
                versions::activate(globals, &name, &version).await
            }
            PackageVerb::Delete { id, force } => delete::run(globals, &id, force).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(flatten)]
        package: PackageCmd,
    }

    fn limit_of(args: &[&str]) -> u64 {
        let cli = TestCli::try_parse_from(args).unwrap();
        match cli.package.verb {
            PackageVerb::Validate { unpack_limit, .. }
            | PackageVerb::Pack { unpack_limit, .. }
            | PackageVerb::Publish { unpack_limit, .. } => unpack_limit.max_unpacked_bytes,
            _ => panic!("the verb has no unpack limit"),
        }
    }

    /// CLOACINA-T-1062: `--sign` takes the key file on `pack` and `publish`;
    /// `upload` takes a signature file.
    #[test]
    fn sign_and_signature_flags_are_read() {
        let cli = TestCli::try_parse_from(["t", "pack", "pkg", "--sign", "key.pem"]).unwrap();
        match cli.package.verb {
            PackageVerb::Pack { sign, .. } => assert_eq!(sign, Some(PathBuf::from("key.pem"))),
            _ => panic!("not pack"),
        }
        let cli = TestCli::try_parse_from(["t", "publish", "pkg", "--sign", "key.pem"]).unwrap();
        match cli.package.verb {
            PackageVerb::Publish { sign, .. } => assert_eq!(sign, Some(PathBuf::from("key.pem"))),
            _ => panic!("not publish"),
        }
        let cli =
            TestCli::try_parse_from(["t", "upload", "a.cloacina", "--signature", "a.sig"]).unwrap();
        match cli.package.verb {
            PackageVerb::Upload { signature, .. } => {
                assert_eq!(signature, Some(PathBuf::from("a.sig")))
            }
            _ => panic!("not upload"),
        }
        let cli = TestCli::try_parse_from(["t", "upload", "a.cloacina"]).unwrap();
        match cli.package.verb {
            PackageVerb::Upload { signature, .. } => assert_eq!(signature, None),
            _ => panic!("not upload"),
        }
    }

    #[test]
    fn max_unpacked_bytes_flag_is_read_by_validate_pack_and_publish() {
        for verb in ["validate", "pack", "publish"] {
            assert_eq!(
                limit_of(&["t", verb, "pkg", "--max-unpacked-bytes", "1073741824"]),
                1_073_741_824,
                "{verb}"
            );
        }
    }
}
