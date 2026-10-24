---
title: "Behaviour specs"
description: "How to run, read and write the Gherkin features of cloacina-bdd"
sidebar:
  order: 62
reviewer: "dstorey"
review_date: "2026-10-24"
---

The behaviour specs say what cloacina promises a user, in Gherkin. Each
scenario runs in CI. A failing one means a behaviour changed. The approach is
recorded in ADR CLOACINA-A-0015.

## Where they live

All of it is in the test crate `crates/cloacina-bdd`:

| Path | Holds |
| --- | --- |
| `features/<area>/*.feature` | The features. Sync's are in `features/sync/`. |
| `features/definitions/` | Connector definitions that sit next to the corpus while the suite runs. |
| `tests/bdd.rs` | The runner: every feature, one scenario at a time. |
| `tests/steps/<area>.rs` | The step definitions. |
| `tests/support/world.rs` | The World: the state of one scenario, and how a step reaches cloacina. |

The suite uses the Rust [`cucumber`](https://crates.io/crates/cucumber)
crate.

## Run them

The scenarios keep their tables in a Postgres database. Locally, that is the
dev Postgres:

```bash
angreal services up
angreal test bdd
```

`angreal test bdd` runs every feature twice: first with the scenario's runner
on SQLite, then on Postgres. Its options:

| Option | Does |
| --- | --- |
| `--runner sqlite` or `--runner postgres` | Runs one runner database only. |
| `--name <regex>` | Runs the scenarios whose name matches. |
| `--tags <expression>` | Runs the scenarios that match a tag expression, such as `@wip`. |
| `--junit <path>` | Writes a JUnit report per runner, to `<path>-<runner>.xml`. |
| `--start-postgres` | Starts the compose Postgres first. The CI lane uses this. |

Cargo runs the suite too, with cucumber's own flags after `--`:

```bash
cargo test -p cloacina-bdd --test bdd -- --name "cursor"
```

The runner reads these variables:

| Variable | Means |
| --- | --- |
| `CLOACINA_BDD_RUNNER` | `sqlite` (default) or `postgres`: the database of each scenario's runner. |
| `CLOACINA_BDD_PG_URL` | The Postgres for the scenarios' tables. The default is the dev Postgres on port 15432. |
| `CLOACINA_BDD_JUNIT` | A path for a JUnit report. |

In CI the **BDD Features** lane runs `angreal test bdd` against a Postgres of
its own. It keeps the JUnit reports as an artifact.

## What a scenario gets

Each scenario starts with a fresh World:

- **A runner.** It sits on a SQLite file, or in a Postgres schema of its own. A
  second runner on the same database can take over from the first. The
  resume feature uses this to replace a runner that died.
- **Tables.** The tables in the scenario's Postgres are prefixed with the
  scenario's name. Two scenarios that both name `orders` never meet.
- **A sync.** It is named after the scenario.
- **A mock JSON API**, when a step asks for one. It records every request.
- **Stored secrets**, which the runner resolves.

When the scenario ends, the runner stops and the tables, the schema and the
SQLite file are dropped.

## Write a scenario

Start from the steps that exist; a feature in the same area shows them. A
records table has a header row. Its first column is the primary key. A column
whose every value is an integer becomes `BIGINT`, and any other column becomes
`TEXT`.

```gherkin
Scenario: A destination that refuses a later segment keeps the cursor where it was
  Given the sync runs a pass
  And the source table "orders" gains the records
    | id | name  |
    | 4  | Robin |
  And the destination's database refuses connections
  When the sync runs a pass
  Then the pass fails
  And the committed cursor is "3"
```

A feature holds a promise: what a user can rely on. The edge cases of one
function stay in Rust tests next to it.

## Add a step

A step is an async function on the World, with a `given`, `when` or `then`
attribute:

```rust
#[then(expr = "the committed cursor is {string}")]
async fn cursor_is(w: &mut SyncWorld, expected: String) {
    let cursor = w.committed_cursor().await;
    assert_eq!(cursor.as_deref(), Some(expected.as_str()), "the committed cursor");
}
```

Word a step as what a user does or sees, not as how the test does it. "The
destination's database refuses connections" is a good step. "The destination
URL points at port 9" is not. The how goes in the World. That way, a later
driver can run the same features against a running server and agents, without
the features being rewritten.

A step that the features in that area share goes in `tests/steps/<area>.rs`.
State that a step needs to keep goes on the World.

## Faults

Some Sync promises are about a crash. The suite builds `cloacina-sync` with
its test-only `fault-injection` feature, so a step can ask for a fault:

| Fault | Effect |
| --- | --- |
| `fail_after_write_once` | Fails the write task once per sync, after the destination took the segment and before the cursor commits. |
| `slow_write_ms:<n>` | Holds the write task for `n` milliseconds at that same point. |

A shipped binary has no faults: without the feature, the param is ignored.

## The features so far

| Feature | Promise |
| --- | --- |
| `sync/pass.feature` | A pass moves the records, and the cursor follows them. |
| `sync/cursor.feature` | The cursor never passes a record that wasn't written. |
| `sync/resume.feature` | A sync resumes after a crash, and each record arrives once. |
| `sync/dead_letters.feature` | Dead-letters never stop a run. |
| `sync/credentials.feature` | A credential never widens the grants. |
| `sync/declarative.feature` | A declarative sync works end to end. |
