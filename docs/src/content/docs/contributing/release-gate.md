---
title: "The Release Gate"
description: "The check that compares a pull request with the last release, what it compares, how to mark an intended break in the changelog, and how a release bump moves the changelog."
sidebar:
  order: 63
---

While Cloacina is 0.x, a patch release breaks nothing and a minor release may
break, with each break in `CHANGELOG.md`. The release gate holds a pull
request to that: it compares the head with the last release tag, lists each
change that can break a user, and fails when a break is not named in the
changelog.

The gate is `scripts/check_release_compat.py`. It runs in the workflow
`.github/workflows/release-gate.yml` on each pull request to `main` and to an
integration branch (`burn-down-*`), and at release in the `verify-version`
job of `.github/workflows/unified_release.yml`.

## What it compares

The base is the last `v*` tag reachable from the head
(`git describe --tags --abbrev=0 --match 'v*'`). Each file is read at the tag
with `git show <tag>:<path>` and compared with the head.

| Surface | Files | A break: needs an entry | Reported, no entry needed in a minor |
|---|---|---|---|
| HTTP API | `docs/public/openapi.json` | an operation removed; a parameter removed or made required; a response status removed; a request field removed, made required, or newly required; a field type changed; an enum value removed; a response field that may now be `null`; a request field that may no longer be `null` | an enum value added to a response |
| Chart values | `charts/cloacina-server/values.yaml`, `charts/cloacina-agent/values.yaml` | a key removed or renamed; a value that became a map | a default changed |
| Chart render | the release's `values.yaml` of each chart | `helm template` of the head chart with that file fails | the head chart refuses it with its own `fail` or `required` message |
| Metrics | `docs/src/content/docs/reference/metrics-catalog.md` | a metric removed or renamed; a label removed; a kind changed | |
| Version constants | `plugin_interface(version = …)`, `AGENT_PROTOCOL_VERSION`, `DELIVERY_PROTOCOL_VERSION`, `PACKAGE_DESCRIPTION_VERSION`, the `*_CONSTRUCTOR_INTERFACE_VERSION` constants | a number changed | |
| Migrations | `crates/cloacina/src/database/migrations/{postgres,sqlite}/` | a folder added (an upgrade note) | |
| Server flags | `crates/cloacina-server/src/main.rs` and the server code | a flag removed or renamed; its environment variable removed or renamed; an environment variable the code no longer names | a default changed |

Some checks read the head only, and fail with no way to mark them:

- a metric name or label in the code that is not in the metrics catalog, or a
  catalog row that no code registers or records. The code is every `.rs`
  file under `crates/`, less the `tests` and `benches` folders, `tests.rs`
  files, and each file from its first `#[cfg(test)]` module on. A metric
  name must be a string literal or a `const` of the same file;
- a released migration folder removed, renamed, or with a changed `up.sql`;
- the [server flags page](../../reference/server-flags/) that differs from the
  server command. Regenerate it with
  `python3 scripts/check_release_compat.py flags --write`;
- a changelog with no `## [Unreleased]` section.

## Mark an intended break

Name the thing in backticks in `## [Unreleased]` of `CHANGELOG.md`: a break
under `### Changed (breaking)`, with the step the user takes; a migration
under `### Upgrade notes`. A name in another section does not count. One
entry may name several things. Each failure says which name to write and in
which section, for example:

```text
FAIL charts/cloacina-server/values.yaml: value `server.drainSeconds` removed or renamed since v0.13.0; name `server.drainSeconds` under `## [Unreleased]` / `### Changed (breaking)`
```

The names the gate looks for:

| Change | Name |
|---|---|
| An operation, or a parameter or status of it | `DELETE /v1/tenants/{tenant_id}/workflows/{name}/{version}` |
| A field of a schema | `CreateKeyRequest.role`, or the operation |
| A chart value | `server.logFormat` (or `cloacina-server.server.logFormat`) |
| A render with the old values | `charts/cloacina-server/values.yaml` |
| A metric or a label of it | `cloacina_tasks_total` |
| A version constant | `AGENT_PROTOCOL_VERSION`, `plugin_interface` |
| A migration | its number, `064`, or its folder name |
| A flag or an environment variable | `--default-executor` or `CLOACINA_DEFAULT_EXECUTOR` |

A changed default needs no entry in a minor; when it asks the user to act,
say so under `### Upgrade notes`.

## Run it

```sh
pip install pyyaml
python3 scripts/check_release_compat.py                  # against the last v* tag
python3 scripts/check_release_compat.py --base v0.12.0   # against another tag
python3 scripts/check_release_compat.py --skip-render    # with no helm
python3 -I scripts/tests/test_check_release_compat.py    # its unit tests
```

The render needs `helm` and the chart dependencies
(`helm dependency update charts/cloacina-server`). The exit code is 0 when
nothing fails, 1 when something fails, 2 when no tag is found: a clone needs
the tags (`git fetch --tags`; in a workflow, `fetch-depth: 0`).

The workflow has a switch, `RELEASE_GATE_WARN_ONLY`. While it is `"true"`,
the gate prints its failures as warnings and the job passes. With
`"false"`, a failure fails the job. At release the gate is always enforced.

`ci.yml` does not run on a pull request that changes only `docs/**`,
`*.md` or `CHANGELOG.md`. The release gate has no such filter, so a pull
request that only edits the changelog or the metrics catalog runs it.

## A release bump

`angreal release bump X.Y.Z` sets every version touchpoint to `X.Y.Z` and
moves the changelog: `## [Unreleased]` becomes `## [X.Y.Z] - <date>`, with
its empty subsections dropped, and a new empty `## [Unreleased]` with
`### Changed (breaking)` and `### Upgrade notes` goes above it.

When the workspace version differs from the last tag (the bump pull request,
and every pull request after it until the tag), the gate reads
`## [X.Y.Z]` in place of `## [Unreleased]`, and also fails when:

- `CHANGELOG.md` has no `## [X.Y.Z]`, or no `## [Unreleased]` above it;
- `## [Unreleased]` has entries: they are not in the release;
- the release is a patch (same minor as the last tag) and the gate lists any
  change at all, marked or not, or `## [X.Y.Z]` has entries under
  `### Changed (breaking)`. Cut a minor instead.

At release, `verify-version` runs
`python3 scripts/check_release_compat.py --release vX.Y.Z`: the base is the
tag before `vX.Y.Z`, and the same rules apply.
