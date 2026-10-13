---
title: "Documentation"
description: "How to write and maintain Cloacina documentation"
sidebar:
  order: 61
reviewer: "dstorey"
review_date: "2026-05-18"
---

This guide provides practical information about writing and maintaining documentation for the Cloacina project.

## Documentation Structure

The docs are an [Astro Starlight](https://starlight.astro.build/) site (`docs/`, theme [starlight-aurora](https://github.com/colliery-io/starlight-aurora)) organized by the [Diátaxis Framework](https://diataxis.fr/). The pages are in `docs/src/content/docs/`:

- `tutorials/`, `how-to/`, `explanation/` — one folder for each reader path inside each: `embed/` (the **embedded library**) and `service/` (**`cloacina-server`**). How-to guides for the engine primitives are in `how-to/workflows/`, `how-to/computation-graphs/` and `how-to/constructors/`; the primitives themselves are described in `explanation/primitives/`, and the engine internals in `explanation/engine/`.
- `start/`, `embed/`, `service/`, `engine/` — the orientation pages of each path ("Start Here" in the sidebar).
- `reference/` — Hand-written **lookup reference**: CLI, HTTP API, WebSocket protocol, configuration, environment variables, macros, errors, metrics catalog, glossary, troubleshooting, plus `python-api/` and `sdks/` subtrees.
- `reference/api/` — **Generated** API reference (Rust crates + the `cloaca` Python bindings). Never hand-edit these pages; see [Generated documentation pipelines](#generated-documentation-pipelines) below.
- `contributing/` — This section.

When adding a new doc, decide first which quadrant it belongs to, then which reader path. Tutorials teach one path, how-tos solve one task, reference looks things up, explanation explains. If a doc spans paths, it lives in the most relevant one and the others cross-link to it.

The sidebar (`docs/src/sidebar.mjs`) lists each folder as a group, labelled with the title of its `index.md`, in the order of `sidebar.order`. Link between pages with relative links to the route (`../workflows/task/`), so that the links work under the base path of the published site. The old Hugo addresses redirect to the new pages (`docs/src/redirects.json`).

### Nomenclature compliance

All docs must comply with [`CLOACI-S-0011`](https://github.com/colliery-io/cloacina/blob/main/.metis/specifications/CLOACI-S-0011/specification.md). In particular: never use `reactive scheduler` / `reactive computation graph` / `reactive subsystem`. Use `reactor`, `computation graph`, and `traversal` per spec.

## Generated documentation pipelines

Two documentation surfaces are generated from source rather than hand-written. Know which one you are touching before you edit anything.

### 1. plissken → `docs/src/content/docs/reference/api/`

The [plissken](https://github.com/colliery-io/plissken) tool generates the entire `reference/api/` folder from source, its overview page (`reference/api/index.md`) included:

- **Config**: `plissken.toml` at the repository root, with `template = "starlight"`. It lists the Rust crates to document (`crates/cloacina`, `crates/cloacina-computation-graph`, `crates/cloacina-workflow`, `crates/cloacina-python`) and the Python package (`cloaca`, generated from the PyO3 binding source in `crates/cloacina-python/src`).
- **Command**: `plissken render` (0.2.0) from the project root. Output lands in `docs/src/content/docs/reference/api/` (`index.md`, one page for each module, and `_sidebar.mjs`, the sidebar group) and is committed like any other content. Private items are left out. The Python API follows the `#[pymodule]` of the bindings and is one page, `cloaca`.
- **Removed pages**: plissken deletes the page of a module that went away. Add its old address to `docs/src/redirects.json`, pointing at the nearest page, so the address keeps working.
- **Wiring**: this pipeline is **not** run by any angreal task or CI workflow — regeneration is a manual step. If you add, move, or remove public modules in the covered crates, rerun `plissken render` and commit the result; otherwise the generated tree drifts (pages of removed modules, missing pages for new ones).
- **Doc links**: write rustdoc links to other crates in the intra-doc form, `` [`cloacina_workflow::Context::secret`] ``. plissken copies a link written as `[text](cloacina_workflow::Context::secret)` into the page as is, and it is a broken link there.

### 2. utoipa → `docs/public/openapi.json` (CI drift-gated)

The REST API spec is emitted from the server's utoipa annotations:

```bash
cargo run -p cloacina-server --bin cloacina-server -- emit-openapi > docs/public/openapi.json
```

The committed spec is the public API contract. CI runs `angreal docs spec-check`, which re-emits the spec and diffs it against the committed `docs/public/openapi.json`; any change to server routes or `cloacina-api-types` DTOs must ship with a regenerated spec or the check fails. Note the gate only covers what is utoipa-annotated — annotate new routes so they appear in the spec.

## Writing Guidelines

### General Principles
- Write clear, concise, and accurate documentation
- Use active voice and present tense
- Include practical examples where appropriate
- Keep documentation up-to-date with code changes
- Consider the reader's perspective and experience level

### API Documentation and Cross-Linking

When documenting API features or referring to API components in the documentation, link to the page of the module in `reference/api/`, with a relative link. Each item has a fixed anchor:

| Item | Anchor |
|------|--------|
| Rust struct | `#struct.Name` |
| Rust enum | `#enum.Name` |
| Function | `#fn.name` |
| Method | `#Type.method` |
| Python class | `#class.Name` |

For example, from a page in `explanation/engine/`:

```markdown
[`cloacina::models`](../../../reference/api/rust/cloacina/models/)
[`Context`](../../../reference/api/rust/cloacina-workflow/context/#struct.Context)
```

### Pages: Markdown and MDX

A page is Markdown (`.md`). A page that uses a Starlight component, such as `<Tabs>`, is MDX (`.mdx`): in MDX, write `{`, `}` and `<` in text as `\{`, `\}` and `&lt;`. Use Starlight's asides (`:::note`, `:::tip`, `:::caution`, `:::danger`) for hints, and a `mermaid` code block for a diagram.

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
  (which runs the Starlight dev server at http://localhost:4321, with live reload; it installs the npm packages of `docs/` on the first run).
- Use `angreal docs build` to validate the site builds before opening a PR. The site is in `docs/dist`.
- Use `angreal docs spec-check` before opening a PR that touches server routes or API DTOs — CI runs the same gate.
- Check the [Starlight documentation](https://starlight.astro.build/) for Markdown, MDX and components
- Review existing documentation for style and format consistency

## Need Help?

If you need assistance with documentation:
- Check existing documentation for examples
- Ask in the project's communication channels
- Review the [Diátaxis Framework](https://diataxis.fr/) for guidance on documentation types
