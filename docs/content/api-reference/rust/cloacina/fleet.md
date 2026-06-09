# cloacina::fleet <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Execution-agent fleet protocol (CLOACI-I-0114).

Pure protocol types — no diesel, no engine internals — shared by
`cloacina-server` (the `FleetExecutor` and agent endpoints) and the
`cloacina-agent` binary (T-0632), plus any future SDK. OQ-E (physical
share with the [[CLOACI-I-0113]] SDK crate) can move these without
behavior change.
