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

// Enforce exactly one database backend is selected
#[cfg(all(feature = "postgres", feature = "sqlite"))]
compile_error!("Cannot enable both 'postgres' and 'sqlite' features simultaneously");

#[cfg(not(any(feature = "postgres", feature = "sqlite")))]
compile_error!("Must enable exactly one database backend: either 'postgres' or 'sqlite'");

use anyhow::Result;
use clap::Parser;
use cloacina_ctl::*;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging level based on verbose/quiet flags
    init_logging(&cli);

    match cli.command {
        Commands::Compile {
            ref project_path,
            ref output,
            ref cargo_flags,
        } => {
            let _result = compile_workflow(
                project_path.clone(),
                output.clone(),
                cli.target.clone(),
                cli.profile.clone(),
                cargo_flags.clone(),
                &cli,
            )?;
        }
        Commands::Package {
            ref project_path,
            ref output,
            ref cargo_flags,
        } => {
            package_workflow(
                project_path.clone(),
                output.clone(),
                cli.target.clone(),
                cli.profile.clone(),
                cargo_flags.clone(),
                &cli,
            )?;
        }
        Commands::Inspect {
            ref package_path,
            ref format,
        } => {
            inspect_package(package_path.clone(), format.clone(), &cli)?;
        }
        Commands::Visualize {
            ref package_path,
            details,
            ref layout,
            ref format,
        } => {
            visualize_package(
                package_path.clone(),
                details,
                layout.clone(),
                format.clone(),
                &cli,
            )?;
        }
        Commands::Debug {
            ref package_path,
            ref action,
        } => {
            debug_package(package_path.clone(), action, &cli)?;
        }
    }

    Ok(())
}
