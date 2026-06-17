---
id: authoring-surface-cruft-sweep-let
level: task
title: "Authoring-surface cruft sweep — let workflow authors get away with just types (minimize required boilerplate)"
short_code: "CLOACI-T-0720"
created_at: 2026-06-17T03:15:23.168112+00:00
updated_at: 2026-06-17T11:48:01.312927+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Authoring-surface cruft sweep — let workflow authors get away with "just types" (minimize required boilerplate)

## Objective

Do a serious, deliberate sweep of the **workflow/computation-graph authoring
surface** to find and remove the cruft an author is currently forced to bring in
— boilerplate registration, manifests, builder ceremony, trait impls,
re-declaration of things the types already imply — so that the common case is as
close to **"just write your types and your functions"** as possible. The
guiding test: *what is the minimum an author must type to get a working
workflow, and how much of everything-else can the framework infer, default, or
derive?*

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (DX / adoption; not blocking, but compounds on every new author)

### Technical Debt Impact
- **Current Problems**: Authoring a workflow/graph today plausibly requires more
  than the essential logic — e.g. boilerplate task/graph registration, restating
  dependency wiring the function signatures already imply, hand-maintained
  manifest/metadata fields, builder calls that duplicate what attributes declare,
  context/IO plumbing, and Rust↔Python parity gaps that force authors into the
  heavier path. Every bit of required ceremony is a thing to get wrong, a thing
  to document, and a tax paid on every new workflow. (The embedded-first
  philosophy makes low-friction authoring core, not cosmetic —
  see the embedded-first principle in project memory.)
- **Benefits of Fixing**: Faster authoring, fewer footguns, smaller surface to
  teach in docs ([[CLOACI-T-0686]], [[CLOACI-T-0687]]), and a cleaner story for
  the "just types" mental model. Less generated/required cruft also shrinks what
  the compiler, packaging, and registration paths must handle.
- **Risk Assessment**: Not addressing it keeps friction high for every new
  author and lets the gap between the "minimal example" in docs and the real
  required boilerplate widen. Risk of *over*-correcting: too much magic/inference
  can hurt explicitness and debuggability — the sweep must distinguish
  "ceremony that buys nothing" from "explicitness that earns its keep."

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] An inventory: for each authoring primitive (Task, Workflow, Computation
      Graph + `#[computation_graph]`, Reactor/Trigger, Accumulator, Package),
      document the *minimum required* author-written surface vs. what is
      boilerplate/derivable/defaultable. Produce a concrete "what must you type"
      list per primitive, Rust **and** Python.
- [ ] A ranked list of cruft-removal opportunities, each tagged: derive-it /
      default-it / infer-from-types / macro-away / delete-as-redundant, with an
      effort + risk estimate and a note on whether it's a breaking change.
- [ ] A target "minimal author" reference example per primitive (the smallest
      thing that should compile and run) — used as the north-star and as a
      regression guard against re-accreting boilerplate.
- [ ] Findings that are large enough to be their own work are split out into
      scoped tasks (this item is the sweep + recommendations; not every fix has to
      land here).
- [ ] Rust↔Python parity is explicitly assessed — authors shouldn't be pushed to
      the heavier language path just to avoid boilerplate ([[CLOACI-T-0688]]).

## Implementation Notes

### Technical Approach
This is primarily an **audit/spike that produces recommendations + scoped
follow-up tasks**, not a single big refactor. Drive it from the author's seat:
take the canonical minimal package ([[CLOACI-T-0678]] `package new`) and a real
example, and for each line ask "did the author *have* to write this, or could the
framework have supplied it from the types/signatures/attributes?"

Areas to probe (confirm against the real API during the sweep — don't assume):
- **Registration / wiring boilerplate** — task/graph registration, dependency
  declaration that duplicates what function signatures or attributes already
  encode. Can dependencies be inferred from input/output types instead of
  re-stated?
- **Builder ceremony** — `WorkflowBuilder` calls that restate what attribute
  macros already declare ([[CLOACI-T-0686]], [[CLOACI-T-0687]] document this
  surface today).
- **Macro coverage** — does `#[computation_graph]` ([[CLOACI-S-0006]]) and the
  task/workflow macro set carry enough that the author writes types + functions
  and little else? Where does the author still hand-write what a macro could
  derive?
- **Manifest / package metadata** — hand-maintained fields in the package
  manifest that could be derived at pack time ([[CLOACI-I-0119]] authoring DX;
  `package validate` [[CLOACI-T-0679]] already knows the canonical shape).
- **Context / IO plumbing** — boilerplate to read/write context vs. "take typed
  inputs, return typed outputs."
- **Accumulators / reactors** — trait impls an author must provide vs. built-ins
  + derives ([[CLOACI-S-0004]] accumulator trait; reactor [[CLOACI-S-0005]]).
- **Rust↔Python parity** — where one language forces more ceremony than the other
  ([[CLOACI-T-0688]] tracks known parity gaps).

### Dependencies
Builds on the authoring-DX initiative [[CLOACI-I-0119]] (scaffold, one-command
pack, author-time validation, canonical format) — that lowered setup friction;
this targets the per-workflow *code* friction. Coordinates with the docs
authoring surface ([[CLOACI-T-0686]], [[CLOACI-T-0687]]): if the docs need a long
"boilerplate you must write" section, that's a signal of cruft to cut.

### Risk Considerations
- Inference/magic vs. explicitness: each "infer it" recommendation must keep the
  result debuggable and the error messages good. Prefer derive-with-escape-hatch
  over hidden behavior.
- Breaking changes: removing required boilerplate may change the authoring API;
  flag which recommendations are additive (new ergonomic path, old path still
  works) vs. breaking, and sequence accordingly.
- Don't conflate "author cruft" with "compiler/runtime internals" — the goal is
  the *author-facing* minimum, even if the framework does more under the hood.

### Related code
- `crates/cloacina-macros/` (or equivalent) — `#[computation_graph]`, task/workflow
  attribute macros; the biggest lever for "derive it from types."
- `WorkflowBuilder` surface (per [[CLOACI-T-0686]]/[[CLOACI-T-0687]] docs).
- `crates/cloacina/src/registry/...` + packaging/manifest paths from
  [[CLOACI-I-0119]] / [[CLOACI-T-0677]] (canonical format) /
  [[CLOACI-T-0678]] (`package new`) / [[CLOACI-T-0679]] (`package validate`).
- Python authoring surface (cloaca bindings) for the parity assessment.
- `.metis/code-index.md` for module locations — read before exploring.

## Sweep Findings (2026-06-17)

First pass complete — a four-surface audit (Rust task/workflow, computation
graph / reactor / accumulator, Python `cloaca`, packaging/scaffolding) driven
from real examples + macro/loader source. All line cites verified against the
live `crates/` and `examples/` trees. **This section is the research output to
guide the follow-up work; the fixes below are recommendations, not yet done.**

### Headline conclusion

The framework already hides a *lot* — `#[workflow]`/`WorkflowBuilder` auto-discover
tasks, build + validate the DAG, detect cycles, and auto-register via `inventory`;
the packaged task list / dependencies / DAG / `workflow_name` are **derived from
compiled code via FFI** (`crates/cloacina/src/packaging` + `package!()`’s
`get_task_metadata`), not hand-written. So "just types" is *close* on the happy
path. The remaining cruft is concentrated, repetitive, and mostly **additive to
remove** (new ergonomic path, old path still compiles). The single biggest theme:
**we already made several things optional in code but every example still writes
them**, so authors learn the ceremony as if it were required.

### Cross-cutting themes (ranked by leverage-per-effort)

**T1 — Stop requiring/teaching attrs that are already optional. [S, additive, highest ROI]**
The code already defaults these; the examples don't use the defaults:
- Rust `dependencies = []` on leaf tasks — already defaulted to `Vec::new()`
  (`crates/cloacina-macros/src/tasks.rs:76,209-211`); only `id` is required
  (`tasks.rs:205`). 48/122 example tasks still write it.
- Rust `id = "fn_name"` duplicates the fn ident the macro already has
  (`tasks.rs:643,687-690`). Default `id` to the fn name → a bare `#[task]` becomes
  valid.
- ~~Python `return context` — wrapper already re-clones input ctx on `None`
  return (`crates/cloacina-python/src/task.rs:233-238`); every example returns
  anyway.~~ **CORRECTED 2026-06-17 (during [[CLOACI-T-0732]]):** this is WRONG.
  The `None`-return path rebuilds context from `original_data`, a snapshot taken
  *before* the body runs (`task.rs:207`), so it **discards in-body
  `context.set(...)` mutations**. `return context` is REQUIRED for any mutating
  task — not redundant. (Possible author footgun worth its own item.)
- Python `id=` — falls back to `func.__name__` (`task.rs:498-502`); every example
  passes it.
- Python stringly deps — function-ref deps already accepted (`task.rs:642-659`).
Fix = mostly a docs + examples pass + 1–2 tiny macro defaults. Near-zero risk.

**T2 — Stringly-typed Context I/O (Rust). [M, additive]**
No typed accessor exists: `Context::get` returns `Option<&serde_json::Value>`
(`crates/cloacina-workflow/src/context.rs:193`), so every input read is a
`get().and_then(|v| v.as_array()).ok_or_else(…)?.clone()` + `from_value` block —
repeated ~8× in one example (`examples/tutorials/workflows/library/03-dependencies/src/main.rs:118-174`)
— and every write wraps in `json!(...)`. Add `get_as::<T>` / `get_required::<T>` /
`insert_as<T: Serialize>` helpers (fold the boilerplate, emit a `TaskError` on
miss/mismatch). This is the biggest line-count sink in real Rust task bodies.
NOTE: Python's `context.get(k, default)` / `set(k,v)` is already clean —
this is a **Rust-side parity catch-up**, not a both-language gap.

**T3 — Invariant type ceremony in Rust signatures. [S, additive]**
The macro hardcodes `Context<serde_json::Value>` (`tasks.rs:919-920`) and rebuilds
the error as `TaskError::ExecutionFailed` regardless (`tasks.rs:923-941`), so
`context: &mut Context<serde_json::Value>) -> Result<(), TaskError>` is 100%
restated on every task. Let the macro accept `context: &mut Context` /
`-> Result<()>` (+ a `use super::*;` injection into the generated module so authors
stop writing it — `workflow_attr.rs:314-318`). Keep the *typed-error contract*; just
default the *spelling*.

**T4 — `package.toml` carries constants + restates the code. [S–M, mostly additive]**
- Constant-for-everyone triple `interface="cloacina-workflow-plugin"`,
  `interface_version=1`, `extension="cloacina"` — identical in every fixture and
  template (`crates/cloacinactl/src/nouns/package/new.rs:167-170,354-357`). Default
  in the manifest loader.
- `language` is inferable from layout (`Cargo.toml`+`src/lib.rs` vs `workflow/`);
  the validators already key on exactly that (`.../package/manifest.rs:84-149`).
  This field already caused a costly drift bug (T-0666). Infer it; keep as override.
- `entry_module` is conventionally `<module>.tasks`/`.graph`; default it
  (`new.rs:176,217`).
- `requires_python` is unused at build (`crates/cloacina-compiler/src/build.rs:217-223`);
  default it.
- `workflow_name` / `description` / `reaction_mode` / `input_strategy` are **already
  in the macro attrs** and extractable via the same FFI path that derives the task
  DAG (`crates/cloacina-build`/`packaging` + `lib.rs:128-159`). Deriving them kills
  the #1 drift-bug class (manifest disagreeing with code — exactly T-0666).
  A minimal Python `package.toml` could shrink to `name` + `version` +
  `workflow_name`.

**T5 — Rust package shell boilerplate. [M, additive]**
Every Rust package hand-maintains a byte-identical 3-line `build.rs`
(`new.rs:289-292`; validator only *warns* on absence — `manifest.rs:141-147`, so
it's not author intent) and a ~30-line `Cargo.toml` with `crate-type=["cdylib",
"rlib"]`, `[features] packaged`, and 4 `cloacina-*` deps + serde/async-trait/futures
(`new.rs:319-348`). Collapse via (a) a single `cloacina-workflow-sdk` umbrella crate
→ one dep line, and (b) compiler-injected `build.rs`/`crate-type` when absent (it
already drives cargo and knows it needs a cdylib — `build.rs:396-569`). Also lint
out the `__WORKSPACE__` path-dep templates still in some fixtures
(`examples/fixtures/demo-pipeline-rust/Cargo.toml:1-2`).

**T6 — Computation-graph manual runtime wiring. [M, additive]**
The embedded CG path makes authors copy-paste a ~60-line `main()` block — four
`mpsc::channel`s, an always-`None`-field `AccumulatorContext`, the
`CompiledGraphFn = Arc::new(|c| Box::pin(async move { <mod>_compiled(&c).await }))`
closure (which the macro *already* emits for inventory —
`crates/cloacina-computation-graph` codegen `:290-294`), a "required but unused"
`manual_rx` (`08-accumulators/src/main.rs:170-171`), a restated
`InputStrategy::Latest`, and two `tokio::spawn`s — verbatim across tutorial-08,
tutorial-10, and `examples/performance/computation-graph/src/main.rs:274-328`. The
*production* scheduler already proves this is a ~3-line `load_graph(decl)`
(`scheduler.rs:99-115`). Expose an embedded-friendly builder (`Graph::spawn(&shutdown)`
+ `<mod>_graph_fn()` ctor) and the whole block disappears.

**T7 — Accumulator passthrough boilerplate (macros exist, unused). [S–M, additive]**
Every example hand-writes `#[async_trait] impl Accumulator { type Output; fn
process(&mut self, Vec<u8>) -> Option<…> { deserialize(&event).ok() } }` while
`#[passthrough_accumulator]`/`#[stream_accumulator]`/`#[polling_accumulator]`/
`#[batch_accumulator]`/`#[state_accumulator]` are fully implemented + exported
(`crates/cloacina-macros/src/lib.rs:170-240`) with **zero** non-test author uses.
Fix: a blanket `process` default for `Output: DeserializeOwned` + surface the macros
in tutorials.

**T8 — Reactor-declaration redundancy. [S–M, mixed]**
- `accumulators=[a,b]` and `criteria=when_any(a,b)` restate each other — the parser
  even validates one is a subset of the other (reactor_attr `:215-229`). Make
  `criteria=when_any` (no args) default to all declared accumulators.
- `manual_rx`, `InputStrategy::Latest` → default them on a `Reactor` builder
  (see T6).
- Two parallel enums `ReactionMode` (macro) vs `ReactionCriteria` (runtime) bridged
  by `From` — collapsing them is the one **breaking** item here.

**T9 — Hard Rust↔Python parity failures (force a language switch). [M each]**
These are not ergonomics — Python authors **cannot** do these at all and must drop
to Rust (confirms [[CLOACI-T-0688]] #1/#2):
- No `@cloaca.state_accumulator` — 0 hits in `crates/cloacina-python/src`; only
  passthrough/stream/polling/batch exist.
- No `cron`/`timezone` params on `@cloaca.trigger` (`task`/`trigger.rs:97`) vs Rust
  `#[trigger(cron=…, timezone=…)]` (`trigger_attr.rs:291-298`). Python can only
  cron-schedule at the runner level, not author a *packaged* cron trigger.
Closing these lets the primitive docs drop their "Rust-only" caveats.

### Explicitness worth KEEPING (do not "simplify" these)

- The `#[workflow] pub mod` / `with WorkflowBuilder` boundary — it's what enables
  auto-discovery, cycle detection, and namespacing.
- The CG `graph = { … }` edge DSL and per-node `ingest(...)` cache inputs — deps are
  *not* restated elsewhere; the macro cross-validates fn↔topology (codegen `:39-66`).
  Inferring edges from types would be ambiguous for fan-out/routing.
- Routing enums (`enum DecisionOutcome { Trade(...), NoAction(...) }`) — variant names
  appear twice but the enum carries per-branch payload *types* the topology can't;
  worth *adding* a name cross-check, not removing the enum.
- The typed `TaskError` contract, `?` on `context.insert` (real `KeyExists` guard),
  `async` on tasks (macro genuinely branches), and `#[node(blocking)]` opt-in.

### Proposed follow-up tasks (decompose from here)

Ordered by ROI. Items 1, 4 are near-pure docs/loader wins; the rest are scoped code.

1. **Make already-optional attrs optional in practice + retire them from examples**
   (T1) — Rust `id`/`dependencies`, Python `id`/deps/`return context`. S, additive.
2. **Typed Context accessors** (T2) — `get_as`/`get_required`/`insert_as`. M, additive.
3. **Signature/module de-ceremony** (T3) — bare `Context` / `Result<()>` + prelude
   injection. S–M, additive.
4. **`package.toml` minimization** (T4) — loader defaults the constant fields, infer
   `language`/`entry_module`. S, additive.
5. **FFI-derive manifest metadata** (T4 tail) — `workflow_name`/`description`/
   `reaction_mode` from code; kills the T-0666 drift class. M.
6. **Rust package shell elimination** (T5) — `cloacina-workflow-sdk` umbrella +
   compiler-injected `build.rs`/`crate-type`. M.
7. **Embedded CG runtime builder** (T6) — absorb the manual wiring block. M.
8. **Accumulator passthrough default + surface the macros** (T7). S–M.
9. **Reactor declaration defaults** (T8) — criteria=all, optional channel, default
   strategy; (separately) collapse the two enums [breaking]. S–M.
10. **Python parity: `@state_accumulator` + cron `@trigger`** (T9) — feeds
    [[CLOACI-T-0688]]. M each.

### Caveats on this pass

- Specs CLOACI-S-0004 (accumulator) / CLOACI-S-0005 (reactor) were **not** opened;
  findings are grounded in macro/trait/example *source*, not spec text. A
  spec-vs-impl drift read is a separate follow-up if wanted.
- The audit read real fixtures + tutorials; it did not attempt to *compile* a
  stripped-down "minimal author" example to prove each default actually holds end to
  end. Each follow-up task should start by writing that minimal example as its
  regression guard (the AC already asks for this).

## Status Updates

- 2026-06-17: Filed as a tech-debt sweep. Goal: minimize what a workflow author
  is *required* to write so the common case is "just types + functions." Scoped
  as audit → ranked cruft-removal recommendations → split large fixes into their
  own tasks. Builds on the authoring-DX work [[CLOACI-I-0119]]; coordinates with
  the workflow-builder docs and the Rust↔Python parity item [[CLOACI-T-0688]].
- 2026-06-17: **Decomposed** into initiative [[CLOACI-I-0125]] (Authoring-surface
  cruft removal). The 9 in-scope follow-ups are now child tasks
  CLOACI-T-0732…0740; item 10 (Rust↔Python parity failures) lives in
  [[CLOACI-T-0688]]. This task remains the research/north-star; execution tracked
  under I-0125.
- 2026-06-17: First sweep pass complete — four-surface audit (Rust task/workflow,
  CG/reactor/accumulator, Python `cloaca`, packaging) written up under **Sweep
  Findings** with file:line evidence and 10 ranked, scoped follow-up tasks.
  Headline: "just types" is already close on the happy path (DAG/deps/task-list
  are FFI-derived from code, not hand-written); remaining cruft is concentrated and
  mostly additive to remove. Highest-ROI wins are docs/loader-level (T1 retire
  already-optional attrs from examples; T4 default the constant `package.toml`
  fields). Two hard Rust↔Python parity *failures* surfaced (no Python
  `@state_accumulator`, no cron `@trigger`) — routed to [[CLOACI-T-0688]]. Next:
  decompose the 10 follow-ups into their own tasks (or an initiative) when picked up.- 2026-06-17: **Completed (research done).** This sweep's job — audit the
  authoring surface and produce ranked, file:line-grounded recommendations — is
  finished (see Sweep Findings above). It was decomposed into initiative
  [[CLOACI-I-0125]]; 4 of those follow-ups landed (T-0732/0733/0734/0739) and the
  packaging/FFI cluster (T-0735/0736/0737/0738/0740) is blocked pending the fidius
  wasm-traits direction ([[project_fidius_wasm_authoring_shift]]). Closing the
  research task; execution tracking lives under I-0125.