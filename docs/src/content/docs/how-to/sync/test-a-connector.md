---
title: "Test a Connector with the Conformance Kit"
description: "Write a conformance file for a Sync connector and run it through the real connector host against the connector's test service."
sidebar:
  order: 10
---

Every connector that ships with cloacina passes a **conformance file**: TOML
beside the connector that names its config, a case for each method, and what
each case must return. The kit runs the file through the real connector host
and the built component, and reports each failure by method, case and
difference (CLOACINA-A-0014).

The kit is `cloacina_testing::connectors::conformance`.

## 1. Write the file

Put `conformance.toml` beside the connector's `Cargo.toml`:

```toml
connector = "orders"          # names the report
crate = "."                   # the connector crate, relative to this file

[source]
config = { url = "http://${ORDERS_HOST:-localhost}:8080", table = "orders" }
grants = { http = ["${ORDERS_HOST:-localhost}:8080"] }
credential = { kind = "header", secret = "api", name = "Authorization", field = "token", prefix = "Bearer " }

# The secrets the credentials name. Test values only.
[secrets.api]
token = "test-token"

[secrets.revoked]
token = "revoked-token"

[spec]
name = "orders"
roles = ["Source"]

[[check]]
case = "good config"
success = true

[[check]]
case = "a revoked token"
secret = "revoked"
success = false
message_contains = "401"

[discover]
streams = [{ name = "orders" }]

[[read]]
case = "full refresh"
stream = { stream = "orders" }
records = [{ id = 1 }, { id = 2 }]

[[read]]
case = "incremental resumes past 9"
stream = { stream = "orders", sync_mode = "Incremental", cursor_field = "id" }
state = { cursor = 9 }
records = [{ id = 10 }]
checkpoints = [{ cursor = 10 }]
```

What the sections mean:

| Section | Holds |
|---|---|
| `[source]`, `[destination]` | The member (`source` and `destination` by default), its `config`, its `grants`, and its `credential`. A side the connector lacks is left out. |
| `[secrets.<name>]` | The fields of a secret a credential names. |
| `[[setup]]` | Steps run once before the cases, against the test service. A read or write case can have its own `setup` and `verify` steps. |
| `[spec]` | Fields the spec must have. The kit also checks that `config_schema` is a JSON object and that the roles cover the sides the file tests. |
| `[[check]]` | A config to check (`config` merged over the side's, or raw `config_text`), and whether it must succeed. `secret` swaps in another secret. |
| `[discover]` | The streams it must offer. `exact = false` allows others. |
| `[[read]]` | A configured `stream`, a `state` to read from, and the `records`, `changes`, `checkpoints`, `dead_letters` or `fatal` error it must yield. `max_checkpoints` stops a source that does not end. |
| `[[write]]` | A `stream`, the `batches` to write, and the `accepted` count, `dead_letters` or `error` it must return. |

## 2. Know how values match

- An expected object matches when every field it names matches. Fields it
  leaves out are not compared.
- Lists match in any order, except checkpoints, and records with
  `ordered = true`.
- Record and cursor text is parsed as JSON first, and numbers compare by
  value.
- A dead-letter matches on `record`, and on `reason_contains` as a substring.

`${NAME}` and `${NAME:-default}` are replaced before the file is parsed: from
the variables the test passes, then from the environment.

## 3. Run it

A test target in `cloacina-sync` runs the file through the connector host.
The host adapter and the service hooks for Postgres, SQL Server and MinIO live
in `crates/cloacina-sync/tests/support/`:

```rust
#[test]
#[ignore = "needs the orders test service"]
fn orders_conforms() {
    let report = conformance::run(
        &repo("connectors/orders/conformance.toml"),
        &ConnectorHost::new("orders"),
        Some(&OrdersService),
    );
    report.assert_passed();
}
```

`ConnectorHost` builds the crate for `wasm32-wasip2`, stages it as a provider
package, and loads each member with the side's grants and credential. A
`Service` runs the file's steps: implement `run(step)` for the test service,
and return rows for a step that queries. A mock on a random port is passed
in with `conformance::run_with` and a map of variables.

A failure reads like this:

```text
conformance `orders`: 1 failed, 6 passed
  - read `incremental resumes past 9`: records differ: missing [{"id":10}] (expected 1, got 0)
```

## 4. Add it to CI

A file that needs a service runs in the Postgres integration lane
(`.angreal/test/integration.py`), with the service started under its own
compose profile. A file that needs nothing, like the echo fixture's or the
Snowflake mock's, runs wherever the target runs.

## Related

- [Sync Connectors](../../../reference/connectors/) for the shipped
  connectors and their files.
- [Connector Contract](../../../reference/connector-contract/) for the
  methods and types.
- [Credential Injection](../../../explanation/primitives/sync/credentials/).
