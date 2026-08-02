# cloacina::input_interface <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Injectable input interface (CLOACI-I-0128) — re-export of the canonical helpers.

The implementation lives in `cloacina-workflow` (not here) because the
`#[workflow(params(...))]` macro emits calls to `schema_for` into **packaged
cdylibs**, which depend on `cloacina-workflow`, not core `cloacina`. Core +
server code reach the same helpers through this re-export.
Spec: [CLOACI-S-0013]; descriptor decision: [CLOACI-A-0007].
