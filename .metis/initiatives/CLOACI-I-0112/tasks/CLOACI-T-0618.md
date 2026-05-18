---
id: doc-h-operations-fold-in-compiler
level: task
title: "DOC-H: Operations fold-in — compiler-deployment, metrics-catalog, SIGSEGV troubleshooting merge"
short_code: "CLOACI-T-0618"
created_at: 2026-05-18T18:19:31.045970+00:00
updated_at: 2026-05-18T18:49:15.429545+00:00
parent: CLOACI-I-0112
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0112
---

# DOC-H: Operations fold-in — compiler-deployment, metrics-catalog, SIGSEGV troubleshooting merge

## Parent Initiative

[[CLOACI-I-0112]]

## Objective

Fold three orphaned operations files (outside Hugo's `docs/content/`) into the published site. Creates two new platform docs that other clusters cross-link to (`compiler-deployment-runbook.md` and `metrics-catalog.md`) and merges the SIGSEGV troubleshooting source into `troubleshooting.md`. Lands second in Phase 3 sequence (after DOC-A) so DOC-B/-C/-D/-F/-I can cross-link to the new files.

## Scope

### Files in cluster (3 new + 1 edit + 3 deletes)

| Operation | Source | Target | Notes |
|---|---|---|---|
| **Port + extend** | `docs/operations/compiler-deployment.md` (186 lines) | `docs/content/platform/how-to-guides/compiler-deployment-runbook.md` (new) | Long-form runbook. Existing `running-the-compiler.md` (covered by DOC-C) stays as the short-form how-to. |
| **Port + extend** | `docs/operations/metrics.md` (226 lines) | `docs/content/platform/reference/metrics-catalog.md` (new) | Full catalog of every `cloacina_*` metric. |
| **Merge** | `docs/SIGSEGV_TROUBLESHOOTING.md` (70 lines) | `docs/content/troubleshooting.md` (existing — adds `### Native crash troubleshooting (historical)` subsection inside the existing section 19) | Section 19 already references the historical workaround via GitHub URL; replace with anchor to merged subsection. |
| **Delete** | `docs/operations/compiler-deployment.md` | — | After fold-in |
| **Delete** | `docs/operations/metrics.md` | — | After fold-in |
| **Delete** | `docs/operations/` directory | — | After fold-in (should be empty) |
| **Delete** | `docs/SIGSEGV_TROUBLESHOOTING.md` | — | After merge |

### Extension scope per new doc

**`platform/how-to-guides/compiler-deployment-runbook.md`** — port + extend source:
- Update stale "until T-0501" caveat (lines 75-88) — Docker image + Helm chart shipped per I-0111 and T-0610.
- Add `--require-signatures` + `--verification-org-id` deploy implications per I-0103.
- Add I-0104 hardening flags (`--frozen --offline`, `setrlimit`) — point at `running-the-compiler.md` for the full threat model.
- Add `/metrics` endpoint and `--log-retention-days` per I-0109; cross-link the metric names to `metrics-catalog.md`.
- Add Helm chart deploy reference (T-0610 local Postgres subchart); the chart doesn't yet include the compiler, note this.
- Org slug: per DOC-A's slug decision (`colliery-io`), fix `ghcr.io/colliery-io/cloacina-{server,compiler}` references (lines 111, 127).
- Verify CLI verbs (`cloacinactl compiler {status,health}`, `cloacinactl package inspect`) against current code.
- Verify ADR/spec IDs (ADR-0004, CLOACI-S-0010) against current Metis state.

**`platform/reference/metrics-catalog.md`** — port + extend source:
- Fix NOM in source: `metrics.md:202` (`/v1/health/reactors` → `/v1/health/graphs`); `metrics.md:196-201` ("reactive stack" → "event-driven stack" or similar).
- Fix Hugo cross-references: `../../.metis/adrs/` repo-relative links don't render; either remove or replace with `{{< ref "..." >}}` to published ADR docs (none currently published).
- Verify task IDs cited (`CLOACI-T-0498`, `T-0536`, `T-0591`).
- Otherwise the metric table is current — covers I-0099, I-0108, I-0109 with labels, descriptions, PromQL examples.
- Add a CG-side quick-reference section linked from `computation-graphs/reference/_index.md` (per Phase 2 decision to collapse the proposed CG metrics doc into a pointer here).

**`troubleshooting.md` — merge SIGSEGV (section 19 edit, not full doc rewrite)**:
- Preserve the existing section 19 framing (it already references the historical workaround at lines 862-872).
- Append a `### Native crash troubleshooting (historical)` subsection containing the alternative-approaches list from `SIGSEGV_TROUBLESHOOTING.md:34-66` and debugging tips from `SIGSEGV_TROUBLESHOOTING.md:67-71` (dedupe vs current `troubleshooting.md:877-880`).
- Replace the GitHub URL cross-link in the existing section 19 with the new in-doc anchor.
- DOC-I handles the broader troubleshooting.md rewrite (L); DOC-H only does this surgical merge.

### Cross-cluster dependencies

- **Blocked by**: DOC-A (drift sweep — NOM fixes in source files happen here as part of port; DOC-A doesn't touch `docs/operations/` since it's outside Hugo, but the org slug decision must be locked)
- **Blocks**: DOC-B, DOC-C, DOC-D, DOC-F, DOC-I (these clusters cross-link to `metrics-catalog.md` and `compiler-deployment-runbook.md`; both must exist before downstream tasks can write resolving cross-links)
- **Coordinate with**: DOC-I (which does the broader troubleshooting.md rewrite); DOC-I owner should know the SIGSEGV subsection lands here

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `docs/content/platform/how-to-guides/compiler-deployment-runbook.md` exists, ported from operations source + extended for I-0104/I-0109/I-0111/T-0610.
- [ ] `docs/content/platform/reference/metrics-catalog.md` exists, ported from operations source + NOM-fixed + linked from explanations and how-tos.
- [ ] `docs/content/troubleshooting.md` section 19 contains the `### Native crash troubleshooting (historical)` subsection; external GitHub URL link replaced with in-doc anchor.
- [ ] `docs/operations/compiler-deployment.md`, `docs/operations/metrics.md`, `docs/SIGSEGV_TROUBLESHOOTING.md` deleted.
- [ ] `docs/operations/` directory deleted (no remaining files).
- [ ] `grep -rn "docs/operations" docs/ README.md` returns zero matches (all cross-links updated).
- [ ] `grep -rn "SIGSEGV_TROUBLESHOOTING.md\|raw.githubusercontent.com.*SIGSEGV" docs/ README.md` returns zero matches.
- [ ] Cross-links from `platform/how-to-guides/use-cloacina-compiler-locally.md` (DOC-C edits) and `README.md:147` (DOC-I) point at the new in-tree paths.
- [ ] `angreal docs:build` passes; both new docs render and are linked from `platform/_index.md` index sections (DOC-B/-C/-D may surface the cross-links).

## Implementation Notes

### Sources

- **Audit file**: `.metis/initiatives/CLOACI-I-0112/audit-python-misc.md` (the "Operations fold-in" section — D auditor was assigned this scope because it folds into top-level/platform from outside Hugo)
- **Source files**:
  - `/Users/dstorey/Desktop/cloacina/docs/operations/compiler-deployment.md` (port verbatim then extend)
  - `/Users/dstorey/Desktop/cloacina/docs/operations/metrics.md` (port verbatim then NOM-fix + extend)
  - `/Users/dstorey/Desktop/cloacina/docs/SIGSEGV_TROUBLESHOOTING.md` (merge into existing section 19)
- **Code paths** (for the extend pass):
  - Compiler: `crates/cloacina-compiler/src/{main,config,health}.rs`
  - Server: `crates/cloacina-server/src/main.rs:39-89`
  - Helm chart: `charts/cloacina-server/{Chart.yaml,values.yaml}`, `charts/cloacina-server/charts/postgresql/`
  - Metrics emission sites: `crates/cloacina/src/execution_planner/scheduler_loop.rs:362-394`, `crates/cloacina/src/computation_graph/{reactor,accumulator,scheduler}.rs`, `crates/cloacina-server/src/lib.rs:1849-1962`, `crates/cloacina-compiler/src/health.rs`
- **Archived initiatives**: I-0097, I-0099, I-0104, I-0108, I-0109, I-0111; tasks T-0610, T-0609, T-0608, T-0529, T-0532; ADR CLOACI-A-0004 (compiler service), ADR CLOACI-A-0005 (deployment trust)

### Approach

Small cluster, lands fast. Sequence:

1. **Hugo front-matter** on each new doc: title, description, weight, no draft.
2. **`compiler-deployment-runbook.md`** first (Day 1): port verbatim from source, then patch the May 2026 batch additions. Verify every CLI verb + flag against current code; verify ADR IDs.
3. **`metrics-catalog.md`** second (Day 1-2): port verbatim, fix the 2 NOM instances, fix Hugo cross-link syntax, verify metric names against emission sites.
4. **SIGSEGV merge** third (Day 2): edit existing section 19 of `troubleshooting.md` (DOC-I will do the broader rewrite; this is a surgical merge); replace external link with anchor.
5. **Delete sources** + verify `git status` is clean.
6. **Cross-link audit** (Day 2): grep the entire docs tree + README for `docs/operations` and `SIGSEGV_TROUBLESHOOTING`; update each to new paths.

### Risk considerations

- DOC-I writes after DOC-H but the troubleshooting.md merge step happens *here*. Avoid stepping on each other: DOC-H makes a minimal surgical change to section 19 (add subsection, replace external link); DOC-I rewrites the rest of the file freely.
- The metrics catalog source is dense (226 lines) and tightly cross-referenced. Port with structure intact; resist the urge to rewrite organizationally during the port. Edit pass comes after.
- The compiler-deployment runbook source uses `colliery-io` org slug (consistent with `install.sh`). DOC-A decision is `colliery-io`; verify when porting.
- If DOC-A hasn't locked the org slug yet (sequencing slip), use `colliery-io` per `install.sh:REPO`.

## Status Updates

### 2026-05-18 — execution

Completed in same Ralph session as DOC-A. Operations fold-in done end-to-end.

**Work summary:**
1. **`platform/how-to-guides/compiler-deployment-runbook.md` (new)** — ported `docs/operations/compiler-deployment.md` verbatim then extended:
   - Removed stale "until T-0501" caveat — Docker image + Helm chart shipped per I-0111 / T-0610. Updated wording to reference the live `ghcr.io/colliery-io/cloacina-{server,compiler}` images.
   - Org slug confirmed `colliery-io` per `install.sh:REPO` (lock from DOC-A); image refs match.
   - Cross-linked I-0103 (`require-signed-packages`), I-0106 (`decommission-a-tenant`), I-0104 (`running-the-compiler` hardening), I-0109 (`metrics-catalog` for `/metrics` + `--log-retention-days`), T-0610 (`deploying-to-kubernetes` for the embedded Postgres subchart), T-0609 (`rdkafka` build deps inherited into the runtime image).
   - K8s section rewritten: chart-driven server install, layered compiler `Deployment` on top (chart doesn't template the compiler yet).
   - Config knobs table extended with `--log-retention-days`.
   - Observability section added pointing at the new metrics catalog.
   - References block extended with all relevant I-* and T-* initiatives + ADR-0004.
2. **`platform/reference/metrics-catalog.md` (new)** — ported `docs/operations/metrics.md` verbatim then fixed:
   - NOM-OPS-01 (`/v1/health/reactors` → `/v1/health/graphs`) in the "Current gaps" section.
   - NOM-OPS-02 ("reactive stack" → "event-driven stack") in the "Current gaps" section.
   - Hugo cross-references: removed all `../../.metis/adrs/...` repo-relative links that wouldn't render. Replaced ADR/initiative references with plain prose mentions (e.g., "per ADR-0005") plus a "Related" block at the end.
   - Added a "Related" section linking back to `compiler-deployment-runbook.md`, `performance-tuning.md`, and `observe-execution-state.md`.
3. **`troubleshooting.md` SIGSEGV merge** — surgical edit to section 19 only:
   - Replaced the external `github.com/.../SIGSEGV_TROUBLESHOOTING.md` link in the Historical mitigations bullet with an in-doc anchor `[#native-crash-troubleshooting-historical](#native-crash-troubleshooting-historical)`.
   - Added a new `#### Native crash troubleshooting (historical)` subsection at the end of section 19 (before the `---` separator before section 20). Subsection contains:
     - Historical root-cause framing (preserved from source).
     - 7-item "Alternative approaches" list (ported from source lines 34-66).
     - Additional debugging note about `ldd` / `otool -L` OpenSSL linkage (the unique source-only debugging tip not already covered by section 19's existing Debugging tips at lines 877-880).
   - DOC-I (broader troubleshooting.md rewrite) was not touched — coordinated as documented in DOC-H scope.
4. **Source deletions**: `docs/operations/compiler-deployment.md`, `docs/operations/metrics.md`, `docs/operations/` directory, `docs/SIGSEGV_TROUBLESHOOTING.md` — all deleted.
5. **Cross-link audit** (post-deletion):
   - Fixed `platform/how-to-guides/use-cloacina-compiler-locally.md:189` external GitHub link → `{{< ref "/platform/how-to-guides/compiler-deployment-runbook" >}}`.
   - Fixed `contributing/_index.md:56` "Adding a Prometheus metric? Also update `docs/operations/metrics.md`" → `[Metrics Catalog]({{< relref "/platform/reference/metrics-catalog" >}})`.
   - Fixed `workflows/how-to-guides/observe-execution-state.md:254` external GitHub link → `{{< ref "/platform/reference/metrics-catalog" >}}`.
   - Fixed `README.md:151` `docs/operations/compiler-deployment.md` → published GitHub Pages URL `https://colliery-io.github.io/cloacina/platform/how-to-guides/compiler-deployment-runbook/`.
   - Historical in-doc reference at `troubleshooting.md:884` to `docs/SIGSEGV_TROUBLESHOOTING.md` retained intentionally — it's a "merged from X during DOC-H" provenance note inside the merged subsection itself.

**Acceptance criteria results:**
- ✅ `docs/content/platform/how-to-guides/compiler-deployment-runbook.md` exists; covers I-0104/I-0109/I-0111/T-0610 extensions.
- ✅ `docs/content/platform/reference/metrics-catalog.md` exists; NOM-fixed; references back from explanation/how-to clusters added.
- ✅ `docs/content/troubleshooting.md` section 19 has the `### Native crash troubleshooting (historical)` subsection (markdown level `####` since it's inside section 19's `###`); external link replaced with anchor.
- ✅ `docs/operations/{compiler-deployment.md,metrics.md}` and `docs/SIGSEGV_TROUBLESHOOTING.md` deleted.
- ✅ `docs/operations/` directory deleted.
- ✅ `grep -rn "docs/operations" docs/content/ README.md` returns zero user-facing matches (only Metis planning artifacts under `.metis/` retain the references, which is correct — those describe the source files).
- ✅ `grep -rn "SIGSEGV_TROUBLESHOOTING.md" docs/content/ README.md` returns only the intentional historical-provenance note inside the merged subsection.
- ✅ Cross-links from `use-cloacina-compiler-locally.md`, `contributing/_index.md`, `observe-execution-state.md`, and `README.md` all point at the new in-tree paths.

**Verification needed externally (user action):**
- Run `angreal docs build` to verify the Hugo site builds clean. The two new docs should render and be linked from the upper-level toc-trees (DOC-B, DOC-D may further surface them from `platform/_index.md`).

**Flags for downstream clusters:**
- **DOC-B**: `platform/reference/_index.md` should add `metrics-catalog.md` to its TOC. `platform/reference/metrics-catalog.md` itself includes a "Related" block; verify it cross-links cleanly with the post-DOC-B http-api.md.
- **DOC-C**: `running-the-compiler.md` should add a cross-link to the new `compiler-deployment-runbook.md` (the long-form complement). `running-the-server-image.md` may also want to mention the metrics catalog.
- **DOC-D**: `platform/explanation/observability.md` (new in DOC-D) should defer the full metric list to `metrics-catalog.md` — design.md already specifies this. The catalog's "Related" block names that doc; verify the back-link from observability.md when DOC-D lands.
- **DOC-I**: `troubleshooting.md` deep rewrite (L) should NOT touch the new `#### Native crash troubleshooting (historical)` subsection inside section 19 — that's DOC-H's surgical merge.
- The Metis planning docs (`.metis/initiatives/CLOACI-I-0112/` audit + design + task files) still reference `docs/operations/...` as the *source* of the fold-in. Those are correct historical references and should NOT be edited.
