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

//! Cloacina control-plane UI — Leptos CSR (CLOACI-I-0141).
//!
//! The Rust/WASM successor to the React SPA (I-0117/I-0129/I-0130): same
//! routes, same auth/session semantics, same-origin API via the
//! contract-tested `cloacina-client` (wasm transport, T-0932), styled by the
//! Aurora design system (`colliery-io-aurora` 0.4: light and dark themes,
//! following the OS by default, with a toggle in the top bar).

mod app;
mod auth;
mod brand;
mod charts;
mod components;
mod config;
mod data;
mod ops;
mod routes;
mod shell;
mod util;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}

#[cfg(test)]
mod tests {
    //! Theme guards (COLLIERY-T-1837): the page must follow the Aurora
    //! tokens in both themes.

    const INDEX_HTML: &str = include_str!("../index.html");
    const APP_CSS: &str = include_str!("../style/app.css");

    /// A raw colour: `#abc`, `#aabbcc`, `#aabbccdd`, or `rgb(`/`rgba(`/`hsl(`.
    fn raw_colours(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let bytes = text.as_bytes();
        for (i, _) in text.match_indices('#') {
            let hex: String = text[i + 1..]
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            let next = bytes.get(i + 1 + hex.len()).copied().unwrap_or(b' ');
            let ends_word = !(next.is_ascii_alphanumeric() || next == b'_' || next == b'-');
            if matches!(hex.len(), 3 | 4 | 6 | 8) && ends_word {
                found.push(format!("#{hex}"));
            }
        }
        for f in ["rgb(", "rgba(", "hsl(", "hsla("] {
            if text.contains(f) {
                found.push(f.to_string());
            }
        }
        found
    }

    #[test]
    fn index_html_runs_the_aurora_theme_init_script() {
        assert!(
            INDEX_HTML.contains(aurora_leptos::THEME_INIT_SCRIPT),
            "index.html must carry aurora_leptos::THEME_INIT_SCRIPT verbatim"
        );
        let script = INDEX_HTML.find("<script>").expect("inline script");
        let css = INDEX_HTML
            .find("<link data-trunk rel=\"css\"")
            .expect("stylesheet link");
        assert!(
            script < css,
            "the init script must come before the stylesheet"
        );
        assert!(INDEX_HTML.contains(r#"<meta name="color-scheme" content="light dark" />"#));
    }

    #[test]
    fn no_raw_colours_in_the_page_shell_or_the_local_stylesheet() {
        assert_eq!(raw_colours(INDEX_HTML), Vec::<String>::new(), "index.html");
        assert_eq!(raw_colours(APP_CSS), Vec::<String>::new(), "style/app.css");
    }

    #[test]
    fn no_raw_colours_in_the_rust_views() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            for entry in std::fs::read_dir(&d).expect("read src") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("main.rs")
                {
                    let text = std::fs::read_to_string(&path).expect("read file");
                    assert_eq!(
                        raw_colours(&text),
                        Vec::<String>::new(),
                        "{}",
                        path.display()
                    );
                }
            }
        }
    }

    #[test]
    fn the_raw_colour_scan_finds_colours() {
        assert_eq!(raw_colours("background: #0e1116;"), vec!["#0e1116"]);
        assert_eq!(raw_colours("color: rgba(0,0,0,.5)"), vec!["rgba("]);
        assert!(raw_colours("see issue #12 and var(--bg)").is_empty());
    }
}
