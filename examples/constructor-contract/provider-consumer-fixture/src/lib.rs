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

//! CLOACI-T-0836 — a trivial consumer that depends on the `cloacina-provider-fs`
//! provider so the build-side `provider_bundle` can resolve + build + bundle it from
//! a real Cargo dependency graph. The dependency edge is the whole point.

/// Reference the provider so the dependency is a real edge in the resolved graph
/// (not just a manifest entry `cargo` might prune).
pub fn provider_manifest_name() -> String {
    cloacina_provider_fs::__provider_manifest().name
}
