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

//! CLOACI-T-0829 — print the macro-generated constructor manifest as JSON.
//!
//! Stands in for the packaging step that writes the sidecar `constructor.json`: it
//! calls the `#[constructor]`-generated `__constructor_manifest()` (which now also
//! records the ordered `#[config]` fields) and prints `ConstructorManifest::to_json()`.

fn main() {
    let manifest = task_constructor_twocfg_fixture::__constructor_manifest();
    print!(
        "{}",
        manifest.to_json().expect("serialize constructor manifest")
    );
}
