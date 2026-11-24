// Regenerate wit/ and the author-to-component conversions from src/lib.rs (the
// #[derive(WitType)] typed-method types). A proc-macro cannot see type
// definitions, so this build step produces the WIT the #[plugin_impl] adapter
// consumes.
fn main() {
    fidius_build::emit_wit();
}
