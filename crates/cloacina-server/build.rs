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

fn main() {
    cloacina_build::configure();

    // CLOACI-I-0130 (T-0847), reshaped by I-0141 (T-0932): with `embedded-ui`
    // on, build the Leptos UI (trunk) so rust-embed has a fresh `ui/dist` to
    // embed — staleness is impossible by construction (every feature-on build
    // rebuilds when UI inputs changed). Feature-off builds never touch trunk.
    if std::env::var_os("CARGO_FEATURE_EMBEDDED_UI").is_some() {
        // Containerized builds prebuild ui/dist in a trunk stage and set this
        // to skip the step (the server stage then needs no trunk/wasm target).
        // (Env var name kept from the npm era so existing build wiring works.)
        if std::env::var_os("CLOACINA_EMBEDDED_UI_SKIP_NPM").is_some() {
            println!(
                "cargo:warning=embedded-ui: using prebuilt ui/dist (CLOACINA_EMBEDDED_UI_SKIP_NPM)"
            );
            return;
        }
        let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui")
            .canonicalize()
            .expect("ui/ directory not found — embedded-ui requires the UI sources");
        for input in ["src", "index.html", "Cargo.toml", "Trunk.toml", "build.rs"] {
            println!("cargo:rerun-if-changed={}", ui_dir.join(input).display());
        }
        let status = std::process::Command::new("trunk")
            .arg("build")
            .arg("--release")
            .current_dir(&ui_dir)
            .status()
            .expect(
                "embedded-ui: failed to run trunk — install it (`cargo install trunk`) plus \
                 the wasm32-unknown-unknown target for feature-on builds",
            );
        assert!(
            status.success(),
            "embedded-ui: `trunk build --release` failed (see output above)"
        );
    }
}
