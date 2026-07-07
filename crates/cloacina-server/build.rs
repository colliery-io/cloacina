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

    // CLOACI-I-0130 (T-0847): with `embedded-ui` on, build the SPA so
    // rust-embed has a fresh `ui/dist` to embed — staleness is impossible by
    // construction (every feature-on build rebuilds when UI inputs changed).
    // Feature-off builds never touch Node.
    if std::env::var_os("CARGO_FEATURE_EMBEDDED_UI").is_some() {
        // Containerized builds prebuild ui/dist in a node:20 stage and set
        // this to skip the npm step (the Rust stage then needs no Node).
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
        for input in [
            "src",
            "index.html",
            "package.json",
            "package-lock.json",
            "vite.config.ts",
        ] {
            println!("cargo:rerun-if-changed={}", ui_dir.join(input).display());
        }
        let status = std::process::Command::new("npm")
            .arg("--prefix")
            .arg(&ui_dir)
            .args(["run", "build"])
            .status()
            .expect("embedded-ui: failed to run npm — a Node toolchain is required for feature-on builds");
        assert!(
            status.success(),
            "embedded-ui: `npm run build` failed (see output above)"
        );
    }
}
