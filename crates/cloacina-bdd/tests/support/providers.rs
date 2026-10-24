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

//! The connectors a scenario can name, built to wasm32-wasip2 and staged once
//! per process on the provider search path.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use cloacina::registry::loader::set_provider_search_path;

/// The provider package of the Postgres connector.
pub const POSTGRES: &str = "cloacina-connector-postgres";

/// Stage the connectors and point the provider search path at them.
pub fn staged() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("cloacina-bdd-providers-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("providers dir");
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        cloacina_testing::connectors::stage_connector(&dir, &root.join("connectors/postgres"));
        set_provider_search_path(dir.clone());
        dir
    })
}
