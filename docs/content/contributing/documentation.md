---
title: "Documentation"
description: "How to write and maintain Cloacina documentation"
weight: 61
reviewer: "dstorey"
review_date: "2026-05-18"
---


This guide provides practical information about writing and maintaining documentation for the Cloacina project.

## Documentation Structure

Our documentation follows the [Diátaxis Framework](https://diataxis.fr/) but the structure is **feature-area first, then quadrant** — not the canonical Diataxis "tutorials / how-to / reference / explanation at the top level". Each feature area gets its own Diataxis tree:

- `docs/content/workflows/{tutorials,how-to-guides,reference,explanation}/` — Workflow surface (the DB-backed DAG primitive).
- `docs/content/computation-graphs/{tutorials,how-to-guides,reference,explanation}/` — Computation graph surface (the in-process event-driven DAG primitive).
- `docs/content/platform/{tutorials,how-to-guides,reference,explanation}/` — Operational surface (CLI, server, daemon, multi-tenant, packaging, security).
- `docs/content/python/{workflows,computation-graphs}/{tutorials,how-to-guides,reference,explanation}/` + `docs/content/python/api-reference/` — Python-side mirrors of the same split.

Top-level cross-cutting docs live at:

- `docs/content/_index.md` — Site landing.
- `docs/content/quick-start/` — Navigation hub + `cloacinactl` install.
- `docs/content/glossary.md` — Every term in one place.
- `docs/content/troubleshooting.md` — Common problems, including platform-spanning issues.
- `docs/content/contributing/` — This section.

When adding a new doc, decide first which feature area it belongs to, then which quadrant within that area. If a doc spans feature areas, it lives at the most relevant area and the others cross-link to it.

### Nomenclature compliance

All docs must comply with [`CLOACI-S-0011`](https://github.com/colliery-io/cloacina/blob/main/.metis/specifications/CLOACI-S-0011/specification.md). In particular: never use `reactive scheduler` / `reactive computation graph` / `reactive subsystem`. Use `reactor`, `computation graph`, and `traversal` per spec.

## Writing Guidelines

### General Principles
- Write clear, concise, and accurate documentation
- Use active voice and present tense
- Include practical examples where appropriate
- Keep documentation up-to-date with code changes
- Consider the reader's perspective and experience level

### API Documentation and Cross-Linking

When documenting API features or referring to API components in the documentation, use the `api-link` shortcode. It uses Rust's namespace syntax to create links:

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
  (which wraps `hugo server -D` with the project's configured theme and shortcodes — prefer this over raw `hugo` so theme + shortcode resolution match CI).
- Use `angreal docs build` to validate the site builds without broken cross-links before opening a PR.
- Check the [Hugo documentation](https://gohugo.io/documentation/) for markdown syntax and shortcodes
- Review existing documentation for style and format consistency

## Need Help?

If you need assistance with documentation:
- Check existing documentation for examples
- Ask in the project's communication channels
- Review the [Diátaxis Framework](https://diataxis.fr/) for guidance on documentation types
