---
title: "Package Manifest"
description: "The package.toml manifest — moved to the Package Format reference"
weight: 45
---

# Package Manifest Reference

{{< hint type="warning" title="Moved — `manifest.json` has been removed" >}}
The old `manifest.json` schema this page documented is **no longer used**.
Packages are described by a top-level **`package.toml`** (`[package]` identity +
a closed `[metadata]` table), and the server reads `package.toml` directly. The
`manifest.json` reader/writer machinery has been removed.
{{< /hint >}}

The canonical manifest schema now lives with the archive format:

- **[Package Format]({{< ref "/engine/explanation/package-format" >}})** — the
  `.cloacina` archive layout (Rust + Python) and the full `package.toml`
  `[package]` / `[metadata]` schema, including which keys are accepted and which
  are rejected (`package_type`, `[[metadata.triggers]]`).
- **[Packaging Python Workflows]({{< ref "/python/workflows/how-to-guides/packaging-python-workflows" >}})**
  — the step-by-step Python packaging procedure.

## Triggers

Triggers are **not** declared in the manifest. They are declared in code —
`#[trigger]` (Rust) or `@cloaca.trigger` (Python) — and registered when the
package is compiled/imported at load time. See
[Packaged Triggers]({{< ref "/python/workflows/tutorials/08-packaged-triggers" >}}).
