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

//! CLOACI-T-0826 — print the macro-generated operator manifest as JSON.
//!
//! Stands in for the packaging step (CLOACI-T-0827) that will write the sidecar
//! `operator.json`: it calls the `#[operator]`-generated `__operator_manifest()`
//! and prints `OperatorManifest::to_json()` to stdout. A host-only target — the
//! manifest fn is emitted on every target, while the wasm guest glue is not.

fn main() {
    let manifest = task_operator_macro_fixture::__operator_manifest();
    print!(
        "{}",
        manifest.to_json().expect("serialize operator manifest")
    );
}
