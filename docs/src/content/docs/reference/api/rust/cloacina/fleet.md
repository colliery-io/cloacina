---
title: "cloacina::fleet"
description: "Execution-agent fleet protocol (CLOACI-I-0114)."
editUrl: false
sidebar:
  order: 127
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Execution-agent fleet protocol (CLOACI-I-0114).

Pure protocol types — no diesel, no engine internals — shared by
`cloacina-server` (the `FleetExecutor` and agent endpoints) and the
`cloacina-agent` binary (T-0632), plus any future SDK. OQ-E (physical
share with the [[CLOACI-I-0113]] SDK crate) can move these without
behavior change.

## Re-exports

- [`protocol::*`](../../../rust/cloacina/fleet/protocol/)
