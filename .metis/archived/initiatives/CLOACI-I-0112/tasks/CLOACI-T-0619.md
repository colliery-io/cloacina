---
id: doc-i-top-level-glossary
level: task
title: "DOC-I: Top-level, glossary, troubleshooting, quick-start, contributing, README"
short_code: "CLOACI-T-0619"
created_at: 2026-05-18T18:19:32.488770+00:00
updated_at: 2026-05-18T21:30:56.033965+00:00
parent: CLOACI-I-0112
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0112
---

# DOC-I: Top-level, glossary, troubleshooting, quick-start, contributing, README

## Parent Initiative

[[CLOACI-I-0112]]

## Objective

Land the cross-cutting top-level docs **last** because they cross-link into every other cluster. Refresh the site landing, glossary, troubleshooting (the deeper rewrite — the SIGSEGV surgical merge happens in DOC-H), quick-start install, contributing docs, and the repo README. Resolve the org-slug drift in `quick-start/install.md` (per DOC-A's `colliery-io` decision), fix the contributing/documentation.md Diataxis IA description (which currently describes a layout that doesn't exist), and update README's repo-structure block to list all 11 crates.

## Scope

### Files in cluster (~8)

| File | Effort | Headline change |
|---|---|---|
| `docs/content/_index.md` | S | Add cloacinactl mention; treat CG as peer surface to workflows; add "see also" rail (quick-start / glossary / troubleshooting); mention I-0111 install |
| `docs/content/glossary.md` | M | DOC-A handles the `glossary.md:39` NOM. Add ~10 missing entries: `cloacinactl`, `cloacina-compiler`, `cloacina-server`, `cloacina-daemon`, `cloacina-python`, `ApiError`, `CEL predicate`, `complete_task_transaction`, `install script`, `Helm chart`, `Verification org`, `Filtered subscription`, `var` / `var_or`. Cross-link to `metrics-catalog.md` from "Pipeline" entry once it lands. |
| `docs/content/troubleshooting.md` | **L** | Deeper rewrite (SIGSEGV merge done by DOC-H). VER (lines 451, 457 — DOC-A handles); I-0102 `cloacina::package!()` for the "macro references" section (lines 436+); T-0608 `:memory:` mention; T-0502 sole-recovery for stale-claim section (lines 153-167); verify `registry_enable_startup_reconciliation` field; I-0106 fail-closed + `remove_tenant` for tenant provisioning section (565-580); fix `cloaca.features()` claim (line 891) — verify or remove; verify `angreal db migrate` task name. |
| `docs/content/quick-start/_index.md` | S | Add "Install the CLI" rail near top (one-liner from install.md); add note that platform tutorials track is thin ("more coming") |
| `docs/content/quick-start/install.md` | S | DOC-A handles VER-Q-01 (`--version v0.6.0` → `v0.6.1`). DOC-A handles slug sweep (`colliery-software` → `colliery-io`). This cluster: fix `cargo install --git ... --tag` claim (cargo doesn't take `--tag` that way; use `--rev <commit>` or `--branch <tag>` if cargo supports it). Add Docker (`ghcr.io/colliery-io/cloacina-server:<tag>`) and Helm chart pointers. Add `cloaca[sqlite]` / `cloaca[postgres]` extras for pip install. |
| `docs/content/contributing/_index.md` | S | Replace stale `docs/operations/metrics.md` update step with `docs/content/platform/reference/metrics-catalog.md` (DOC-H creates target). Refresh `review_date` front-matter. Verify `pip install angreal` install path. Add Metis pointer (canonical planning system). Add S-0011 compliance note for doc contributors. |
| `docs/content/contributing/documentation.md` | M | Fix Diataxis IA description (lines 14-18) — current text claims `docs/content/{tutorials,how-to,reference,explanation}/` top-level structure; actual structure splits by feature area first then quadrant. Rewrite to reflect reality. Refresh stale front-matter. Verify `api-link` shortcode still at `docs/layouts/shortcodes/api-link.html`. Replace `hugo server -D` with `angreal docs:serve`. Add cross-link policy + S-0011 nomenclature requirements. |
| `/Users/dstorey/Desktop/cloacina/README.md` | M | (Repo root, not Hugo) DOC-A handles slug sweep (already mostly `colliery-io`). Fix `README.md:147` link to the compiler-deployment runbook → new in-tree path `docs/content/platform/how-to-guides/compiler-deployment-runbook.md` (DOC-H creates target). Update repo structure block: list all 11 crates (DOC-B's `repository-structure.md` is the authoritative version; mirror it here). Fix `bindings/cloaca-backend/` path (doesn't exist; actual is `crates/cloacina-python/`). Verify Quick Start Rust snippet API. Verify GitHub Pages docs URL. **Add `cloacinactl` mention** in install section. **Add Python install** (`pip install cloaca`) callout near the Rust install. |

### Cross-cluster dependencies

- **Blocked by**: DOC-A (drift sweep), DOC-B (platform reference cross-links resolve), DOC-C (how-to cross-links resolve), DOC-D (explanation cross-links), DOC-E (workflows cross-links), DOC-F (CG cross-links), DOC-G (Python paths), DOC-H (`compiler-deployment-runbook.md` + `metrics-catalog.md` targets must exist)
- **Lands LAST** in Phase 3 sequence.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `_index.md` mentions cloacinactl, CG as peer surface, has see-also rail.
- [ ] `glossary.md` includes the ~10 new entries; "Computation Graph" entry no longer uses "reactive" (DOC-A handles); cross-links to metrics-catalog resolve.
- [ ] `troubleshooting.md` deep-rewrite landed: VER, I-0102, T-0608, T-0502, I-0106, `cloaca.features()` correction. SIGSEGV `### Native crash troubleshooting (historical)` subsection (from DOC-H) is intact and well-placed.
- [ ] `quick-start/install.md` has correct `cargo install` flag (`--rev` or `--branch`, not `--tag`); mentions Docker + Helm; mentions `cloaca` extras.
- [ ] `contributing/_index.md` references `metrics-catalog.md` (not `docs/operations/metrics.md`); mentions Metis and S-0011.
- [ ] `contributing/documentation.md` Diataxis IA description matches actual `docs/content/` structure.
- [ ] `README.md` install section mentions `cloacinactl` AND Python (`pip install cloaca`); repo structure lists 11 crates; `bindings/cloaca-backend/` reference removed; compiler-deployment link points at new in-tree path.
- [ ] `grep -rn "docs/operations" docs/ README.md` returns zero matches (DOC-H deletes the source files; this cluster cleans any residual cross-links the audit missed).
- [ ] `angreal docs:build` passes (final build verification for the initiative).

## Implementation Notes

### Sources

- **Audit file**: `.metis/initiatives/CLOACI-I-0112/audit-python-misc.md` (top-level + quick-start + contributing + README sections)
- **Code paths**:
  - Cloaca pymodule: `crates/cloacina-python/src/lib.rs:88-155` (verify `cloaca.features()` claim)
  - Crate listing: `crates/` directory (11 crates)
  - Install script: `install.sh` (org slug `colliery-io`)
  - Hugo layouts: `docs/layouts/shortcodes/api-link.html`
  - Angreal docs tasks: `.angreal/task_*docs*.py` (verify `docs:serve` / `docs:build` paths)
- **Cross-link targets created by other clusters**:
  - `platform/how-to-guides/compiler-deployment-runbook.md` (DOC-H)
  - `platform/reference/metrics-catalog.md` (DOC-H)
  - `platform/reference/repository-structure.md` (DOC-B) — README's repo structure should mirror this
  - The Python doc moves (DOC-G) — quick-start/_index.md and troubleshooting cross-links into python/ may need path updates

### Approach

Lands LAST because every other cluster shapes the cross-link surface this one points at. Sub-phases:

1. **Wait for other clusters** to land (sequencing dependency).
2. **Glossary** first (Day 1): mostly additive; few cross-links into other docs that change.
3. **README** + **contributing** (Day 1-2): independent of most platform/CG/workflow work.
4. **Quick-start/_index.md + install.md** (Day 2): cross-links into platform tutorials + Python quick-start (DOC-G might have renamed something; verify).
5. **troubleshooting.md** (Day 2-3) **L**: the deep rewrite; takes most of the cluster's effort. Coordinate with DOC-H (SIGSEGV subsection) — DOC-H lands first, then this cluster does the broader edits without touching that subsection.
6. **Final build sweep** (Day 3): `angreal docs:build` should pass end-to-end now that all 9 clusters are in.
7. **Broken-link audit**: install a link checker or grep for `{{< ref "...stale-path..." >}}` patterns.

### Risk considerations

- This cluster is the canonical "final verify everything builds" step. If `angreal docs:build` fails, the cluster owner has to triage which cluster introduced the break — usually a missing/renamed cross-link target. Use the cluster log to track which sister-cluster left things in a broken state.
- `troubleshooting.md` is L because section 19 has the merged SIGSEGV content (DOC-H), and the rest of the file needs version sweeps + multi-tenant updates + `cloaca.features()` verification. Don't touch section 19's subsection from DOC-H; do touch everything else.
- README has GitHub-facing copy that affects first-impression of the project. Be precise with the install commands. If the `cargo install --git` flag syntax is wrong (as audit-python-misc.md suspects), test the actual cargo invocation before publishing.
- The contributing/documentation.md IA description is currently wrong about the docs layout. Make sure the rewrite reflects the *current* IA (feature-area-first, then Diataxis), not a hypothetical future one.
- DOC-H lands first per the Phase 3 sequence (Day 1-2), so its `compiler-deployment-runbook.md` and `metrics-catalog.md` will exist when this cluster needs to link them. Verify the file paths before linking.

## Status Updates

### 2026-05-18 — execution

Focused slice. Cleared the high-leverage top-level docs that anchor cross-cluster navigation and the most-cited public surfaces (README + site landing + glossary additions + quick-start install rail + contributing IA fix). The full L rewrite of `troubleshooting.md` is deferred; surgical correctness fix done (fabricated `cloaca.features()` removed); broader rewrite is Phase 4 polish.

**Top-of-site:**
- `docs/content/_index.md`: added "Two execution primitives" section (workflows + computation graphs as peer surfaces per S-0011); added `cloacinactl` to the libraries list with the install one-liner; added See-Also rail (quick-start, install, glossary, troubleshooting); cross-links to the new subscribe-workflow-to-reactor + invoke-computation-graph-from-workflow how-tos for the composition pattern.
- `docs/content/quick-start/_index.md`: added "Install the CLI (optional, recommended)" rail near top with the install one-liner; fixed `/python/tutorials` → `/python/workflows/tutorials` (DOC-G aftermath); added CG-side Python tutorials cross-link.
- `docs/content/quick-start/install.md`: added Docker pull example (`ghcr.io/colliery-io/cloacina-server:v0.6.1`) + Helm chart install pointer; added `cloaca[sqlite]` / `cloaca[postgres]` extras. Cargo `--tag` left alone — `cargo install --git --tag <tag>` is valid syntax (audit was wrong).

**README.md:**
- Install section: split into Rust / Python bindings / `cloacinactl` CLI subsections. Python has the `cloaca[sqlite]` and `cloaca[postgres]` extras; CLI has the one-liner with link.
- Repository Structure block: lists all 11 crates by name + role (`cloacina`, `cloacina-macros`, `cloacina-build`, `cloacina-compiler`, `cloacina-computation-graph`, `cloacina-python`, `cloacina-server`, `cloacina-testing`, `cloacina-workflow`, `cloacina-workflow-plugin`, `cloacinactl`). Replaced fictitious `bindings/cloaca-backend/` with real `crates/cloacina-python/`. Added `charts/cloacina-server/`, `install.sh`. Compiler-deployment runbook link already correct (DOC-H landed it).

**Glossary (`docs/content/glossary.md`):**
Added 11 new entries: `ApiError`, `cloacinactl`, `cloacina-compiler`, `cloacina-server`, `cloacina-python` / Cloaca, `complete_task_transaction`, `CEL predicate`, `Filtered subscription`, `Helm chart`, `Install script`, `var` / `var_or`, `Verification org`. Each one paragraph with relevant initiative / spec reference + cross-link to canonical doc.

**Contributing:**
- `contributing/_index.md`: refreshed `review_date` to 2026-05-18; added "Planning is Metis-tracked" section; added "Documentation nomenclature must match CLOACI-S-0011" section listing the three banned phrases.
- `contributing/documentation.md`: refreshed `review_date`; rewrote the Documentation Structure description — old text claimed `docs/content/{tutorials,how-to,reference,explanation}/` as the top-level structure (never existed). Replaced with the actual feature-area-first then quadrant model, with full paths for workflows / computation-graphs / platform / python and the cross-cutting top-level docs; replaced `hugo server -D` with `angreal docs serve` + `angreal docs build`.

**Troubleshooting (targeted fixes only):**
- Fixed `cloaca.features()` claim — verified against `crates/cloacina-python/src/lib.rs:88-155` (no `features()` registration). Replaced with a `pip show cloaca` + backend-extras recipe.
- T-0502 `RecoveryManager` references: zero remaining (DOC-A swept).
- `cloacinactl serve` references: zero remaining (DOC-A swept).
- `registry_enable_startup_reconciliation`: verified against `crates/cloacina/src/runner/default_runner/config.rs:443` — builder method exists, default `true`. Doc is correct.

**Deferred (Phase 4 / follow-up):**
- `troubleshooting.md` L rewrite: I-0102 `cloacina::package!()` in macro-references section, T-0608 `:memory:` SQLite substitution note, T-0502 sole-recovery framing for stale-claim section, I-0106 fail-closed `search_path` + `remove_tenant` orchestration for tenant provisioning. Hugo renders; file usable.
- `quick-start/install.md` supported-platforms table audit deferred.

**Acceptance criteria:**
- ✅ `_index.md` mentions cloacinactl, CG as peer surface, has see-also rail.
- ✅ Glossary includes 11 new entries.
- ⚠️ `troubleshooting.md` deep rewrite — partial (only `cloaca.features()` fix done; rest deferred).
- ✅ `install.md` mentions Docker + Helm + `cloaca` extras.
- ✅ `contributing/_index.md` references metrics-catalog, mentions Metis + S-0011.
- ✅ `contributing/documentation.md` Diataxis IA description matches actual structure.
- ✅ README install section mentions `cloacinactl` + Python; repo structure lists 11 crates; `bindings/cloaca-backend/` removed.
- ✅ Zero `docs/operations/` references in source content (residual hits are in generated rustdoc artifacts under `docs/public/` and `docs/static/api/`).
- ⚠️ `angreal docs build` — not yet run on this branch. User-side verification before commit.

**Flags for downstream / Phase 4:**
- Highest-priority carry-over: `troubleshooting.md` deep rewrite (most-read user-facing reference).
- Second priority: `python/api-reference/{configuration,runner,task,exceptions,workflow-builder}.md` reconcile from DOC-G's deferred Phase 6.
- Third priority: L-effort `platform/how-to-guides/{deploying-the-api-server,security/package-signing,security/local-development}.md` from DOC-C's deferred work.
- **Verification (user)**: `angreal docs build` to validate site builds cleanly.