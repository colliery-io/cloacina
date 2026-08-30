# Contributing to Cloacina

This document covers the conventions that keep the repo verifiable. The most
important one — the packaged example standard — exists because examples that
nothing executes rot silently: this project has repeatedly found shipped,
documented features that never worked, always in the gap between "documented"
and "run by a harness".

## The packaged example standard (CLOACI-I-0138 / T-0886)

Every example demonstrates its feature through the **primary interface**:

```
pack → upload → (server compiles) → reconcile → execute/observe
```

Examples must not show the in-process `DefaultRunner` as the way to run or
test anything. Embedded remains a legitimate production *deployment* mode, but
examples and onboarding lead with the packaged/server path (maintainer
decision D-3, 2026-07-10). Migrating a feature through the server path is also
how server-path feature gaps get surfaced — loudly, which is the point.

### Where an example lives, and how it joins CI

Place it under `examples/features/<capability>/<name>/`. Discovery is
automatic and keyed on one thing:

- **has `package.toml`** → it is a packaged gold-path example. It gets a
  `angreal demos features <name>` command and joins the CI examples matrix
  via `angreal demos matrix`. No list to edit; a new directory IS the
  registration.
- **no `package.toml`** → it registers as an embedded `cargo run` example.
  New examples should not take this shape (see D-3 above).

If a directory must be excluded (it is a fixture or library crate, not a
user-facing example), add it to the exclusion set in
`.angreal/demos/_utils.py` **with a reason** — nothing user-facing may be
silently unexecuted.

### Required files (Rust)

```
<name>/
  package.toml     # name, version, interface, [metadata]
  Cargo.toml       # crates.io VERSION deps — see below
  src/lib.rs
```

**Version deps, never path deps.** `cloacina-workflow = "0.10"`, not
`{ path = ... }`. The demo/e2e compilers run with the T-0887 dev-workspace
escape hatch (`--dev-workspace` / `CLOACINA_COMPILER_DEV_WORKSPACE`), which
patches crates.io versions to local source — so version deps build against
your checkout during development *and* are the form real users ship. A path
dep bakes your machine's filesystem into the archive and fails `cargo fetch`
inside any container (observed as an unexplained 13ms build failure that took
a live cluster to diagnose).

Python examples: `package.toml` plus a `workflow/` package; the same
discovery and the same rules apply.

### The README

Follow the structure of `examples/features/workflows/simple-packaged/README.md`
(the canonical example, T-0884):

1. **What it teaches** — the feature, in the first paragraph.
2. **Layout** — the files and what each is for.
3. **Run it** — numbered steps through the real lifecycle: bring up the
   stack, point the CLI, pack, upload, execute, observe.
4. **Operate it** — the operational verbs relevant to this example
   (`trigger pause/resume/fire`, `accumulator inject`, `execution events`,
   ...), each with a real invocation.

**Every command the README documents must actually work.** This is enforced:
the demos harness (T-0893) executes the operational verbs an example's lane
declares, and CI runs every example's full gold path. Do not document a verb
you have not run — a README claim was once written for a verb that turned out
to be broken for that example's shape, and the assertion that caught it is
why the rule exists (CLOACI-T-0929).

### Verifying before you push

```bash
angreal demos features <name>       # must exit 0, end-to-end
```

The lane packs from your working tree and the compiler resolves against your
checkout, so this is a true test of what you are about to submit. CI runs the
same command for every example in the matrix.

## Other conventions worth knowing

- **Tests run through angreal**, never raw cargo/pytest invocations:
  `angreal test fast|all|integration|e2e ...`. The tasks encode flags,
  ports, and service dependencies that manual commands get wrong.
- **Verify against a live system.** `cargo check` does not build test
  targets, does not cover feature combinations, and does not expand macros in
  consumer crates. The feature matrix (`postgres,macros` / `sqlite,macros`)
  must be *run*, not just compiled.
- **Metis is the system of record** (`.metis/`): use the `metis` MCP tools or
  CLI for tickets; plans and findings that must outlive a session go there.
- **Squash-merge PRs.** One PR per Metis initiative; standalone tasks get
  their own PR.
- **`cargo fmt --all` before committing Rust** — the fmt lane fails on drift.
