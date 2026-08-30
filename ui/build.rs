/// Emit the Aurora Dark stylesheet into `style/aurora.css` at build time so
/// index.html can `<link data-trunk rel="css">` it — the no-flash path from
/// the pack's README (vs runtime `<AuroraStyles/>` injection).
fn main() {
    let out = std::path::Path::new("style");
    let path = aurora_leptos::write_css(out).expect("emit aurora.css");
    println!("cargo:rerun-if-changed=build.rs");
    let _ = path;
}
