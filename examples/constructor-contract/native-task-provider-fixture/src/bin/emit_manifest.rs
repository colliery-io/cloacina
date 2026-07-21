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

//! Print the `constructor_provider!`-generated provider manifest as JSON — the
//! base `provider.json` the T-0902 e2e test stamps `runtime = "native"` +
//! `component = <cdylib>` into before loading. `__provider_manifest()` is
//! emitted on every target (only the guest glue is wasm-gated).

fn main() {
    let manifest = native_task_provider_fixture::__provider_manifest();
    print!(
        "{}",
        serde_json::to_string_pretty(&manifest).expect("serialize provider manifest")
    );
}
