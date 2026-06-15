---
id: audience-first-docs-restructure-3
level: initiative
title: "Audience-first docs restructure (3/3): Doors, migration & cutover"
short_code: "CLOACI-I-0123"
created_at: 2026-06-15T14:15:40.355556+00:00
updated_at: 2026-06-15T14:15:40.355556+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: audience-first-docs-restructure-3
---

# Audience-first docs restructure (3/3): Doors, migration & cutover Initiative

## Context

Initiative **3 of 3** — the **migration + cutover + review** that makes the new
IA live (parent design + ratified IA: [[CLOACI-I-0121]]; page map:
[[CLOACI-T-0689]]; primitives: [[CLOACI-I-0122]]). Runs on the shared branch
`docs/audience-first-restructure`. **This is the initiative whose completion
triggers the single big PR to `main`** — until it lands, the live site is
unchanged.

By the time this starts, the new tree (I-0121) and the `/engine` primitives
(I-0122) exist additively alongside the old content. This initiative **moves the
remaining body content into the doors and `/reference`, authors the net-new pages,
then performs the cutover** (home, nav, redirects, old-section removal) and the
**final adversarial review**.

Two themes that must be enforced throughout (the *reason* for the whole effort):
- **Co-equal doors, embedded-first as principle.** Kill every "start small →
  graduate to the server" framing ([[project_embedded_first_philosophy]]). Embedded
  is a permanent, production-legitimate end-state, chosen by use case, not skill.
- **Both dialects.** Door tutorials use the `{{< tabs >}}` Rust-default/Python model.

## Goals & Non-Goals

**Goals:**
- **`/embed` door** fully populated: quick-start, the tabbed Rust/Python tutorial track (in-process workflows + computation graphs), embedded how-tos, embedded explanation — **including the net-new "running embedded in production" guide**.
- **`/service` door** fully populated: quick-start, tutorials (deploy, web UI, multi-tenant, packaging, fleet), all platform how-tos, service explanation.
- **`/reference`** consolidated: Python API, generated Rust/Python API (routed), CLI, HTTP/WS, config + env-vars, metrics, SDKs, glossary, troubleshooting.
- **Net-new orientation**: `/start/what-is-cloacina` stating the **embedded-first principle** + two-ways framing; `/start/is-cloacina-for-you` (from when-to-use).
- **Cutover**: home page rewritten to **two co-equal doors**; `hugo.toml` **nav** flipped to the new IA; **`aliases`** applied to every moved page (from [[CLOACI-T-0689]] map); **old sections removed**.
- **"Graduate" framing eliminated** everywhere; co-equal language pass.
- **Final gate passed**: accuracy / completeness / clarity / diátaxis-compliance reviewers to zero blockers+majors; **full internal link-check** (every `{{< ref >}}` resolves); old-URL redirect spot-check; `hugo` clean build.

**Non-Goals:**
- Changing the IA decisions (locked in I-0121) or re-authoring `/engine` (I-0122).
- Closing Rust↔Python parity gaps → [[CLOACI-T-0688]].
- Regenerating the auto-produced API reference (routed/linked, not hand-edited).

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Detailed Design

**Migration mechanics**
- **Move + reframe, don't blind-copy.** Relocate each page per the [[CLOACI-T-0689]]
  map (`git mv` to preserve history where the body is largely reused), then edit
  for the new framing (co-equal, dialect tabs, cross-links to `/engine`).
- **Redirects:** every moved page gets its old path(s) in the new page's
  `aliases:` front matter. Derive the alias list mechanically from the T-0689 map.
- **Tabbed tutorials:** merge the Rust (`library/*`) and Python in-process
  tutorials into one track per topic using `{{< tabs >}}{{< tab "Rust" >}}…{{< tab "Python" >}}…`,
  Rust default. Service-side tutorials similarly consolidate.
- **Generated API reference** (`/api-reference/rust/*`, `/api-reference/cloaca/*`):
  routed/linked under `/reference`, **never hand-edited**; confirm the rustdoc/
  pdoc generation still targets the right output path.

**Cutover sequence (do last, in order, in one focused pass):**
1. Rewrite home `/_index.md` → two co-equal doors (geekdoc `columns` + `button`).
2. Flip `hugo.toml [menu.main]` to the new IA (`/start /embed /service /engine /reference /contributing`).
3. Apply all `aliases`; remove the now-empty old sections (`/quick-start`,
   `/workflows`, `/computation-graphs`, `/python`, `/platform`, `/sdks`,
   top-level `/glossary` & `/troubleshooting` once relocated).
4. Build + link-check + redirect spot-check.

**Final review gate (the docs-diataxis 4-reviewer loop):** dispatch accuracy,
completeness, clarity, diátaxis-compliance reviewers in parallel; fix every
blocker+major; re-run until accuracy/completeness/compliance are clean and clarity
has no blockers. Plus a mechanical **link-check** (no broken `{{< ref >}}`) and an
**old-URL redirect spot-check**.

**"Graduate"-framing sweep:** grep the tree for the banned framing ("graduate",
"grow out of", "start small and …", "without a rewrite" overstatements) and
rewrite to co-equal / repackaging language.

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered

- **Big-bang move without per-page redirects** — rejected: breaks every inbound
  link / bookmark / search result. `aliases` on every moved page is mandatory.
- **Copy instead of `git mv`** — rejected where the body is reused: loses file
  history and doubles content during the branch. Move + edit.
- **Flip nav early / incrementally** — rejected: the cutover is one focused pass
  at the end so the branch state before it stays coherent (and the single PR diffs
  cleanly).
- **Skip the 4-reviewer gate to ship faster** — rejected: this is a full
  re-IA touching ~120 pages; the adversarial gate + link-check is the only thing
  that catches quadrant violations, broken refs, and ungrounded claims at scale.

## Implementation Plan

Decomposed as Metis tasks. Content tasks (1–6) are additive/parallel-ish; the
cutover (7) and gate (8) are strictly last and in order.

1. **`/start` orientation content** — `what-is-cloacina` (NEW: two-ways + embedded-first principle), `is-cloacina-for-you` (from when-to-use), move concepts/features/install; reframe to co-equal.
2. **`/embed` tutorials (tabbed)** — merge Rust `library/*` + Python in-process tutorials + CG library/python into one Rust-default/Python-tab track; renumber.
3. **`/embed` how-to + explanation + quick-start** — relocate embedded-flavored how-tos + runtime/PyO3 explanation; **author NEW "running embedded in production" guide**.
4. **`/service` tutorials + quick-start** — deploy-a-server, web UI, multi-tenant, packaging, fleet; service-side tabbed where dual-language.
5. **`/service` how-to + explanation** — all `/platform/how-to-guides/*` + migration/multi-tenant how-tos; `/platform/explanation/*` deploy/security/scaling/observability.
6. **`/reference` consolidation** — Python API, route generated Rust/Python API, CLI, HTTP/WS, config+env-vars, metrics, SDKs, glossary, troubleshooting.
7. **CUTOVER** — home dual-door rewrite; `hugo.toml` nav flip; apply all `aliases`; remove old sections; `hugo` clean build + redirect spot-check.
8. **FINAL GATE** — 4-reviewer adversarial loop (accuracy/completeness/clarity/diátaxis) to zero blockers+majors; full internal link-check; "graduate"-framing grep sweep; then open the **single PR** to `main`.

Definition of done (whole 3-initiative arc): the new IA is live, every old URL
redirects, every reference claim is code-traceable, no quadrant violations, no
"graduate" framing, both dialects shown, `hugo` builds clean.
