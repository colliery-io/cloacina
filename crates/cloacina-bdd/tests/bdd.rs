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

//! The BDD runner (CLOACINA-A-0015): every feature under `features/`, one
//! scenario at a time, each with a fresh World.
//!
//! - `CLOACINA_BDD_RUNNER`: `sqlite` (default) or `postgres`, the database of
//!   the scenario's runner.
//! - `CLOACINA_BDD_PG_URL`: the Postgres that holds the scenario's tables (and
//!   the runner's schema). Default: the dev Postgres on 15432.
//! - `CLOACINA_BDD_JUNIT`: a path; when set, a JUnit report is written there.
//!
//! Filter with cucumber's own flags after `--`, e.g. `--name "cursor"` or
//! `--tags @wip`.

mod steps;
mod support;

use cucumber::{World as _, WriterExt as _, writer};
use futures::FutureExt as _;

use support::world::SyncWorld;

#[tokio::main]
async fn main() {
    let features = concat!(env!("CARGO_MANIFEST_DIR"), "/features");
    let cucumber = SyncWorld::cucumber()
        // The provider search path is process-wide and each runner polls its
        // database: scenarios run one at a time.
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .after(|_, _, _, _, world| {
            async move {
                if let Some(world) = world {
                    world.teardown().await;
                }
            }
            .boxed_local()
        });
    match std::env::var_os("CLOACINA_BDD_JUNIT") {
        Some(path) => {
            let file = std::fs::File::create(&path).expect("create the JUnit report");
            cucumber
                .with_writer(
                    writer::Basic::stdout()
                        .summarized()
                        .tee::<SyncWorld, _>(writer::JUnit::for_tee(file, 0))
                        .normalized(),
                )
                .run_and_exit(features)
                .await
        }
        None => cucumber.run_and_exit(features).await,
    }
}
