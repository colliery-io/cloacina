# cloacina::workflow::registry <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Workflow registry types.

The process-global workflow registry was removed in CLOACI-T-0509. Workflow
constructors are now owned by [`crate::Runtime`], which is seeded from the
`inventory` entries emitted by the `#[workflow]` macro and then mutated
dynamically by the reconciler when packages are loaded/unloaded.
