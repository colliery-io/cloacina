---
title: "Packaging for Multiple Platforms"
description: "Build Python workflow packages targeting multiple operating systems and architectures"
weight: 40
reviewer: "dstorey"
review_date: "2025-03-13"
---

# Packaging for Multiple Platforms

Build and distribute Python workflow packages that run across different operating systems and CPU architectures. This guide covers target selection, vendoring behavior, and CI/CD patterns for multi-platform builds.

## When You Need Multi-Platform Packages

- **Mixed infrastructure** — deploying workflows to both Linux and macOS servers
- **CI/CD pipelines** — building packages for target environments that differ from the build host
- **Distribution** — shipping workflow packages to users on different platforms

## Supported Platforms

| Target | Platform String | UV Platform |
|--------|----------------|-------------|
| Linux x86_64 | `linux-x86_64` | `x86_64-unknown-linux-gnu` |
| Linux ARM64 | `linux-arm64` | `aarch64-unknown-linux-gnu` |
| macOS x86_64 | `macos-x86_64` | `x86_64-apple-darwin` |
| macOS ARM64 | `macos-arm64` | `aarch64-apple-darwin` |

## Prerequisites

- `uv` installed for dependency resolution and vendoring:

  ```bash
  curl -LsSf https://astral.sh/uv/install.sh | sh
  ```

- A Python project with `pyproject.toml` containing a `[tool.cloaca]` section

## Building for Multiple Targets

Pass one or more `--target` flags to `cloaca build` to specify which platforms the package should support:

```bash
# Build for specific targets
cloaca build -o my-workflow.cloacina --target linux-x86_64 --target macos-arm64

# Default: current platform only
cloaca build -o my-workflow.cloacina
```

{{< hint type=info title="Default Target" >}}
If you omit `--target`, the package is built for your current platform only. Always specify targets explicitly when building packages intended for deployment elsewhere.
{{< /hint >}}

## How Vendoring Works for Multiple Targets

When you build for multiple targets, the vendoring process resolves and bundles your Python dependencies:

- Currently, vendoring resolves dependencies for the **first target** in the list
- **Pure-Python packages** are platform-agnostic and work on all targets without issue
- **Platform-specific binary wheels** (e.g., `numpy`, `pandas`) are resolved per-target
- The `requirements.lock` file records which target was used for resolution
- Future versions will support per-target vendor directories for full binary compatibility

{{< hint type=info title="Pure Python" >}}
If all your dependencies are pure Python, a single build works for every platform. You only need per-platform builds when binary wheels are involved.
{{< /hint >}}

## Dealing with Platform-Specific Dependencies

Binary dependencies require extra care in multi-platform builds:

- **Prefer pure-Python packages** when possible to avoid platform-specific issues
- If you need binary packages (`numpy`, `pandas`, `cryptography`, etc.), **build separate packages per target**
- Vendoring enforces `--only-binary :all:` — packages that only provide source distributions (sdists) will cause an error

```bash
# Build separate packages when binary dependencies are involved
cloaca build -o workflow-linux-x86_64.cloacina --target linux-x86_64
cloaca build -o workflow-macos-arm64.cloacina --target macos-arm64
```

## Manifest Target Field

The `targets` field in `manifest.json` lists the platforms a package is compatible with:

```json
{
  "package": {
    "targets": ["linux-x86_64", "macos-arm64"]
  }
}
```

The server validates platform compatibility at load time using `is_compatible_with_platform()`. A package will be rejected if the current server platform is not in the target list.

## CI/CD Patterns

### Building Per-Platform in CI

Use a build matrix to produce one package per target platform:

```yaml
# Example GitHub Actions matrix
strategy:
  matrix:
    target: [linux-x86_64, linux-arm64, macos-arm64]
steps:
  - run: cloaca build -o workflow-${{ matrix.target }}.cloacina --target ${{ matrix.target }}
```

### Single-Platform Pure-Python Workflow

If your workflow has no binary dependencies, a single build covers all platforms:

```bash
cloaca build -o workflow.cloacina \
  --target linux-x86_64 \
  --target linux-arm64 \
  --target macos-x86_64 \
  --target macos-arm64
```

{{< hint type=info title="CI Tip" >}}
For pure-Python workflows in CI, building once with all targets is simpler and faster than a full build matrix.
{{< /hint >}}

## Troubleshooting

### "No matching distribution"

The package has no pre-built wheel for your target platform. Options:
- Use an alternative package that provides wheels for your target
- Build the wheel from source separately and include it in your project

### "Unsupported target platform"

The target string does not match any supported platform. Check the [supported platforms table](#supported-platforms) above and verify your `--target` value.

### Pure-Python packages with C extensions

Some packages appear to be pure Python but include optional C extensions. These need binary wheels for each target platform. Build separate packages per target if you encounter build failures.

## See Also

- [Quick Start Guide]({{< ref "/python-bindings/quick-start/" >}}) — Getting started with Cloaca
- [Testing Workflows]({{< ref "/python-bindings/how-to-guides/testing-workflows/" >}}) — Testing your workflow packages
- [Performance Optimization]({{< ref "/python-bindings/how-to-guides/performance-optimization/" >}}) — Optimizing workflow execution
