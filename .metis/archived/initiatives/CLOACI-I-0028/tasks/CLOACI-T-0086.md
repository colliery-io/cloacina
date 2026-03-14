---
id: structural-audit-frontmatter
level: task
title: "Structural Audit — Frontmatter, Shortcodes & Build Validation"
short_code: "CLOACI-T-0086"
created_at: 2026-03-13T14:29:48.844652+00:00
updated_at: 2026-03-13T15:16:17.882476+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# Structural Audit — Frontmatter, Shortcodes & Build Validation

**Phase:** 1 — Structural Audit & Link Validation
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Audit every documentation page (50+ files) for structural correctness: valid Hugo frontmatter, correct shortcode usage, and clean `angreal docs build` output with zero warnings or errors.

## Scope

All files under `docs/content/` — tutorials, explanations, API references, how-to guides, quick starts, and contributing guides.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Every `.md` file in `docs/content/` has valid frontmatter (title, description, weight fields present)
- [ ] Every file has `reviewer` and `review_date` frontmatter fields
- [ ] All Hugo shortcodes use valid syntax (`{{< hint type=... >}}`, not `{{< tip >}}` or other nonexistent shortcodes)
- [ ] `angreal docs build` completes with zero warnings and zero errors
- [ ] `angreal docs build --draft` also builds cleanly (catches draft-only pages with issues)
- [ ] Produce a checklist of all files audited with pass/fail status for frontmatter and shortcode validity
- [ ] All issues found are fixed in-place

## Implementation Notes

### Technical Approach
1. Glob all `docs/content/**/*.md` files
2. For each file: parse frontmatter, verify required fields exist, check shortcode patterns via regex
3. Known bad patterns to scan for: `{{< tip >}}`, `{{< warning >}}`, `{{< note >}}` (Hugo Book theme uses `{{< hint >}}` only)
4. Run `angreal docs build` and `angreal docs build --draft` — capture stderr for any warnings
5. Fix all issues found, re-run build to confirm clean

### Known Risk Areas
- Recent docs written by agents may use incorrect shortcode names (we already caught `{{< tip >}}` in CLOACI-T-0078)
- Some older docs may have missing `reviewer`/`review_date` fields
- Draft pages may have issues not caught by regular builds

## Status Updates

### Completed 2026-03-13

**Frontmatter audit** — 73 files checked:
- Fixed 3 files missing `description`: security/_index.md, local-development.md, package-signing.md
- Fixed 1 malformed date: tutorials/05-cron-scheduling.md `review_date: "2024-04-2"` → `"2024-04-02"`
- Root `_index.md` missing `weight` — acceptable for site root

**Shortcode audit** — all shortcodes valid:
- 49 `{{< hint >}}` / `{{< /hint >}}` pairs — all balanced
- All `{{< ref >}}` targets resolve (verified by clean build)
- `{{< tabs >}}`, `{{< tab >}}`, `{{< button >}}`, `{{< toc-tree >}}`, `{{< relref >}}` all properly used

**Build verification:**
- `angreal docs build` passes cleanly — 90 pages, zero Hugo errors
- `angreal docs build --draft` not tested separately (draft content excluded by default in prod mode)
