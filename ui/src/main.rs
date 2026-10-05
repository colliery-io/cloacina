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
    //! Theme guards (CLOACINA-T-0943): the page must follow the Aurora
    //! tokens in both themes.

    const INDEX_HTML: &str = include_str!("../index.html");
    const APP_CSS: &str = include_str!("../style/app.css");

    /// The CSS colour functions, matched without regard to case.
    const COLOUR_FUNCTIONS: [&str; 9] = [
        "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch",
    ];

    /// The CSS named colours (CSS Color Level 4), matched without regard to
    /// case. `transparent` and `currentcolor` are not here: they are not a
    /// fixed colour, so they work in both themes.
    const NAMED_COLOURS: [&str; 148] = [
        "aliceblue",
        "antiquewhite",
        "aqua",
        "aquamarine",
        "azure",
        "beige",
        "bisque",
        "black",
        "blanchedalmond",
        "blue",
        "blueviolet",
        "brown",
        "burlywood",
        "cadetblue",
        "chartreuse",
        "chocolate",
        "coral",
        "cornflowerblue",
        "cornsilk",
        "crimson",
        "cyan",
        "darkblue",
        "darkcyan",
        "darkgoldenrod",
        "darkgray",
        "darkgreen",
        "darkgrey",
        "darkkhaki",
        "darkmagenta",
        "darkolivegreen",
        "darkorange",
        "darkorchid",
        "darkred",
        "darksalmon",
        "darkseagreen",
        "darkslateblue",
        "darkslategray",
        "darkslategrey",
        "darkturquoise",
        "darkviolet",
        "deeppink",
        "deepskyblue",
        "dimgray",
        "dimgrey",
        "dodgerblue",
        "firebrick",
        "floralwhite",
        "forestgreen",
        "fuchsia",
        "gainsboro",
        "ghostwhite",
        "gold",
        "goldenrod",
        "gray",
        "green",
        "greenyellow",
        "grey",
        "honeydew",
        "hotpink",
        "indianred",
        "indigo",
        "ivory",
        "khaki",
        "lavender",
        "lavenderblush",
        "lawngreen",
        "lemonchiffon",
        "lightblue",
        "lightcoral",
        "lightcyan",
        "lightgoldenrodyellow",
        "lightgray",
        "lightgreen",
        "lightgrey",
        "lightpink",
        "lightsalmon",
        "lightseagreen",
        "lightskyblue",
        "lightslategray",
        "lightslategrey",
        "lightsteelblue",
        "lightyellow",
        "lime",
        "limegreen",
        "linen",
        "magenta",
        "maroon",
        "mediumaquamarine",
        "mediumblue",
        "mediumorchid",
        "mediumpurple",
        "mediumseagreen",
        "mediumslateblue",
        "mediumspringgreen",
        "mediumturquoise",
        "mediumvioletred",
        "midnightblue",
        "mintcream",
        "mistyrose",
        "moccasin",
        "navajowhite",
        "navy",
        "oldlace",
        "olive",
        "olivedrab",
        "orange",
        "orangered",
        "orchid",
        "palegoldenrod",
        "palegreen",
        "paleturquoise",
        "palevioletred",
        "papayawhip",
        "peachpuff",
        "peru",
        "pink",
        "plum",
        "powderblue",
        "purple",
        "rebeccapurple",
        "red",
        "rosybrown",
        "royalblue",
        "saddlebrown",
        "salmon",
        "sandybrown",
        "seagreen",
        "seashell",
        "sienna",
        "silver",
        "skyblue",
        "slateblue",
        "slategray",
        "slategrey",
        "snow",
        "springgreen",
        "steelblue",
        "tan",
        "teal",
        "thistle",
        "tomato",
        "turquoise",
        "violet",
        "wheat",
        "white",
        "whitesmoke",
        "yellow",
        "yellowgreen",
    ];

    /// A character that continues a word, so that `white-space`, `app-gold`
    /// and `to_rgb(` are not colours.
    fn is_word(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
    }

    /// A raw colour in CSS, HTML or a Rust view:
    /// - a hex colour: `#abc`, `#abcd`, `#aabbcc`, `#aabbccdd`;
    /// - a colour function in any case: `rgb(`, `RGBA(`, `hsl(`, `hwb(`,
    ///   `lab(`, `lch(`, `oklab(`, `oklch(`;
    /// - a named colour (`red`, `White`, `BLACK`, ...) as a whole value: after
    ///   `:` or `=`, with optional spaces and quotes, and nothing else after
    ///   it. `token::GOLD` is a token constant, not a value, so a `::` before
    ///   the word does not count.
    fn raw_colours(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let bytes = text.as_bytes();
        for (i, _) in text.match_indices('#') {
            let hex: String = text[i + 1..]
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            let next = bytes.get(i + 1 + hex.len()).copied().unwrap_or(b' ');
            if matches!(hex.len(), 3 | 4 | 6 | 8) && !is_word(next) {
                found.push(format!("#{hex}"));
            }
        }
        let lower = text.to_ascii_lowercase();
        for f in COLOUR_FUNCTIONS {
            for (i, _) in lower.match_indices(&format!("{f}(")) {
                let before = i.checked_sub(1).map(|j| bytes[j]);
                if !before.is_some_and(is_word) {
                    found.push(text[i..i + f.len() + 1].to_string());
                }
            }
        }
        for name in NAMED_COLOURS {
            for (i, _) in lower.match_indices(name) {
                let end = i + name.len();
                let before = i.checked_sub(1).map(|j| bytes[j]);
                if before.is_some_and(is_word) || bytes.get(end).copied().is_some_and(is_word) {
                    continue;
                }
                // Walk back over spaces and quotes to the `:` or `=`.
                let mut j = i;
                while j > 0 && matches!(bytes[j - 1], b' ' | b'\t' | b'"' | b'\'') {
                    j -= 1;
                }
                let is_value = match (
                    j.checked_sub(1).map(|k| bytes[k]),
                    j.checked_sub(2).map(|k| bytes[k]),
                ) {
                    (Some(b'='), _) => true,
                    (Some(b':'), Some(b':')) => false,
                    (Some(b':'), _) => true,
                    _ => false,
                };
                // The word is the whole value: after it come spaces and then the
                // end of the value (`;`, a quote, `}`, `)`, `,`, `!important` or
                // a line end), not more words (`caption="gold band"`).
                let mut k = end;
                while matches!(bytes.get(k), Some(b' ' | b'\t')) {
                    k += 1;
                }
                let ends_value = matches!(
                    bytes.get(k),
                    None | Some(b';' | b'"' | b'\'' | b'}' | b')' | b',' | b'!' | b'\n' | b'\r')
                );
                if is_value && ends_value {
                    found.push(text[i..end].to_string());
                }
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
        assert_eq!(raw_colours("color: RGB(0, 0, 0)"), vec!["RGB("]);
        assert_eq!(
            raw_colours("background: oklch(70% 0.1 200)"),
            vec!["oklch("]
        );
        assert_eq!(raw_colours("border-color: hwb(200 10% 20%)"), vec!["hwb("]);
        assert_eq!(raw_colours("color: red;"), vec!["red"]);
        assert_eq!(raw_colours("background:white"), vec!["white"]);
        assert_eq!(raw_colours("color=\"Black\""), vec!["Black"]);
        assert_eq!(raw_colours("style:background=\"teal\""), vec!["teal"]);
        assert!(raw_colours("see issue #12 and var(--bg)").is_empty());
        assert!(raw_colours("white-space: nowrap; class=\"app-gold\"").is_empty());
        assert!(raw_colours("color=token::GOLD size=6").is_empty());
        assert!(raw_colours("let c = to_rgb(x); caption=\"gold band\"").is_empty());
        assert!(raw_colours("background: transparent; color: currentColor").is_empty());
    }
}
