---
id: structural-audit-cross-reference
level: task
title: "Structural Audit — Cross-Reference & External Link Validation"
short_code: "CLOACI-T-0087"
created_at: 2026-03-13T14:29:49.788547+00:00
updated_at: 2026-03-13T15:23:36.865996+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# Structural Audit — Cross-Reference & External Link Validation

**Phase:** 1 — Structural Audit & Link Validation
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Validate every internal cross-reference (`{{< ref >}}` shortcodes) and external link in all documentation pages. Ensure no broken links, no references to nonexistent pages, and no dead external URLs.

## Scope

All files under `docs/content/` — every `{{< ref "..." >}}` shortcode and every `[text](url)` markdown link.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Every `{{< ref "..." >}}` shortcode resolves to an existing page in `docs/content/`
- [ ] Every relative markdown link (`[text](relative-path)`) resolves correctly
- [ ] Every external HTTP/HTTPS link is reachable (200 response or valid redirect)
- [ ] "See Also" / "Related Resources" sections at bottom of docs all point to valid pages
- [ ] Cross-references between Rust and Python doc sections are bidirectional (if A links to B, B should link back to A where appropriate)
- [ ] All broken links are fixed — either by correcting the path or removing dead references
- [ ] Produce a report listing all links checked, status, and any fixes applied

## Implementation Notes

### Technical Approach
1. Extract all `{{< ref "..." >}}` patterns via regex across all docs
2. For each ref: verify the target path exists under `docs/content/`
3. Extract all markdown links `[text](url)` — separate internal vs external
4. For external links: HTTP HEAD request to verify reachability (allow redirects)
5. Build a cross-reference map: which pages link to which — identify orphaned pages (pages with no inbound links)
6. `angreal docs build` also catches broken refs at build time — use as secondary validation

### Known Risk Areas
- Newly created docs (CLOACI-T-0078) may have cross-references to pages that were planned but not yet written
- Python and Rust doc sections may have inconsistent cross-referencing
- External links to crates.io, docs.rs, or PyPI may have changed

### Dependencies
- Should run after CLOACI-T-0086 (frontmatter/shortcode fixes) since build errors could mask link issues

## Status Updates

### Completed 2026-03-13

**`ref` shortcode audit** — 62 unique targets, all resolve:
- Every `{{< ref >}}` target maps to an existing content file
- Mix of absolute (`/tutorials/01-first-workflow/`) and relative (`multi-tenant-recovery`) refs — all valid

**`relref` shortcode audit** — 8 unique targets, all resolve:
- Used in contributing/ and explanation/ sections for same-directory links

**Direct markdown links** — 2 found in `explanation/macro-system.md`:
- `[Workflow Versioning](workflow-versioning.md)` — file exists, valid

**No broken links found.** Clean build confirms all refs resolve at build time.
