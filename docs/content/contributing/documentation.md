---
title: "Documentation"
description: "How to write and maintain Cloacina documentation"
weight: 61
reviewer: "dstorey"
review_date: "2026-05-18"
---


This guide provides practical information about writing and maintaining documentation for the Cloacina project.

## Documentation Structure

The docs are a [Hugo](https://gohugo.io/) site (`docs/`, theme `hugo-geekdoc`) organized around the [Diátaxis Framework](https://diataxis.fr/), but the top level is **reader-path first, then quadrant** — not the canonical "tutorials / how-to / reference / explanation at the top level". The top-level sections under `docs/content/` are:

- `start/` — Orientation: what Cloacina is, whether it fits your problem, core concepts, features, and installation. Read-first pages, no deep quadrant split.
- `embed/` — The **embedded-library path** (Rust or Python application embedding the engine). Contains `quick-start.md` plus its own `tutorials/`, `how-to/`, and `explanation/` trees.
- `service/` — The **service path** (running `cloacina-server` as a multi-tenant control plane with HTTP/WebSocket API and web UI). Same shape: `quick-start.md`, `tutorials/`, `how-to/`, `explanation/`.
- `engine/` — The shared **engine primitives**, described once and referenced from both paths: `workflows/`, `computation-graphs/`, `constructors/`, `packaging/`, `scheduling/`, and `explanation/`.
- `reference/` — Hand-written **lookup reference**: CLI, HTTP API, WebSocket protocol, configuration, environment variables, macros, errors, metrics catalog, glossary, troubleshooting, plus `python-api/` and `sdks/` subtrees.
- `api-reference/` — **Generated** API reference (Rust crates + the `cloaca` Python module). Never hand-edit these pages; see [Generated documentation pipelines](#generated-documentation-pipelines) below.
- `contributing/` — This section.

When adding a new doc, decide first which section it belongs to (which reader path, or the shared engine/reference material), then which quadrant within that section. Tutorials teach one path, how-tos solve one task, reference looks things up, explanation explains. If a doc spans sections, it lives in the most relevant one and the others cross-link to it.

### Nomenclature compliance

All docs must comply with [`CLOACI-S-0011`](https://github.com/colliery-io/cloacina/blob/main/.metis/specifications/CLOACI-S-0011/specification.md). In particular: never use `reactive scheduler` / `reactive computation graph` / `reactive subsystem`. Use `reactor`, `computation graph`, and `traversal` per spec.

## Generated documentation pipelines

Three documentation surfaces are generated from source rather than hand-written. Know which one you are touching before you edit anything.

### 1. plissken → `docs/content/api-reference/`

The [plissken](https://github.com/colliery-io/plissken) tool generates the entire `api-reference/` section (except `_index.md`) from source:

- **Config**: `plissken.toml` at the repository root. It lists the Rust crates to document (`crates/cloacina`, `crates/cloacina-computation-graph`, `crates/cloacina-workflow`) and the Python package (`cloaca`, generated from the PyO3 binding source in `crates/cloacina-python/src`).
- **Command**: `plissken render` from the project root. Output lands in `docs/content/api-reference/` and is committed like any other content.
- **Wiring**: this pipeline is **not** run by any angreal task or CI workflow — regeneration is a manual step. If you add, move, or remove public modules in the covered crates, rerun `plissken render` and commit the result; otherwise the generated tree drifts (orphaned pages for removed modules, missing pages for new ones).

### 2. rustdoc → `docs/static/api/` (the `api-link` shortcode target)

`angreal docs serve` and `angreal docs build` (defined in `.angreal/task_docs.py`) first run `cargo doc --no-deps` and copy `target/doc` into `docs/static/api/` before invoking Hugo. This rustdoc tree is what the `api-link` shortcode links into (URLs under `/api/...`). It is rebuilt on every docs build and is never committed.

### 3. utoipa → `docs/static/openapi.json` (CI drift-gated)

The REST API spec is emitted from the server's utoipa annotations:

```bash
cargo run -p cloacina-server --bin cloacina-server -- emit-openapi > docs/static/openapi.json
```

The committed spec is the public API contract. CI runs `angreal docs spec-check`, which re-emits the spec and diffs it against the committed `docs/static/openapi.json`; any change to server routes or `cloacina-api-types` DTOs must ship with a regenerated spec or the check fails. Note the gate only covers what is utoipa-annotated — annotate new routes so they appear in the spec.

## Writing Guidelines

### General Principles
- Write clear, concise, and accurate documentation
- Use active voice and present tense
- Include practical examples where appropriate
- Keep documentation up-to-date with code changes
- Consider the reader's perspective and experience level

### API Documentation and Cross-Linking

When documenting API features or referring to API components in the documentation, use the `api-link` shortcode. It uses Rust's namespace syntax to create links into the rustdoc tree (pipeline 2 above):

`{{</* api-link path="path::to::component" */>}}`
Renders as: {{< api-link path="cloacina::models" type="module" >}}

You can also customize the display text:
`{{</* api-link path="path::to::component" display="Custom Text" */>}}`
Example: {{< api-link path="cloacina::models" type="module" display="Data Models" >}}

#### Item Types

The shortcode supports different Rust item types through the optional `type` parameter:

```markdown
{{< api-link path="cloacina::context::Context" type="struct" >}}    <!-- For structs -->
{{< api-link path="cloacina::task::Task" type="trait" >}}           <!-- For traits -->
{{< api-link path="cloacina::models" type="module" >}}              <!-- For modules -->
{{< api-link path="cloacina::error::Error" type="enum" >}}          <!-- For enums -->
{{< api-link path="cloacina::types::Result" type="type" >}}         <!-- For type aliases -->
{{< api-link path="cloacina::utils::format_error" type="fn" >}}     <!-- For functions -->
```

Available types:
- `struct` - For structs (default if not specified)
- `enum` - For enums
- `trait` - For traits
- `type` - For type aliases
- `fn` - For functions
- `module` - For modules (uses index.html)

These links will automatically stay up-to-date with the API documentation.

## Documentation Review Process

1. **Self-Review**
   - Check for technical accuracy
   - Verify all links work
   - Ensure examples are up-to-date
   - Review for clarity and completeness

2. **Peer Review**
   - Documentation changes should be reviewed by at least one other contributor
   - Focus on both technical accuracy and clarity
   - Consider the perspective of new users

## Tools and Resources

- Use the angreal task to preview documentation changes:
  ```bash
  angreal docs serve
  ```
  (which integrates rustdoc into `docs/static/api` and wraps `hugo server -D` with the project's configured theme and shortcodes — prefer this over raw `hugo` so theme + shortcode resolution match CI).
- Use `angreal docs build` to validate the site builds without broken cross-links before opening a PR.
- Use `angreal docs spec-check` before opening a PR that touches server routes or API DTOs — CI runs the same gate.
- Check the [Hugo documentation](https://gohugo.io/documentation/) for markdown syntax and shortcodes
- Review existing documentation for style and format consistency

## Need Help?

If you need assistance with documentation:
- Check existing documentation for examples
- Ask in the project's communication channels
- Review the [Diátaxis Framework](https://diataxis.fr/) for guidance on documentation types
