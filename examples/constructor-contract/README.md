# constructor-contract — what runs, what's a fixture

This tree holds the constructor/provider surface's examples **and** its test
fixtures. They look alike (each is a small crate), so this inventory says
explicitly which is which — per the packaged example standard (CONTRIBUTING /
CLOACI-T-0892), nothing user-facing may be silently unexecuted.

## Runnable demo (in the demos surface + CI matrix)

| Directory | What it proves | Run with |
|---|---|---|
| `fs-grant-demo` | The full provider lifecycle (package → stage → `constructor!` consumption) and the **default-closed grant model**: two workflows identical except a `grants =` line; the ungranted one must be denied. Self-checking — exits loudly if the sandbox leaks. | `angreal demos features fs-grant-demo` |

## Provider crates (libraries, not demos)

Consumed by the demo above and by the constructor test lanes; they have no
standalone run story by design.

| Directory | Role |
|---|---|
| `cloacina-provider-fs` | The `read_file` constructor suite `fs-grant-demo` packages and consumes |
| `cloacina-provider-extract` | Extraction constructor suite (constructor e2e lanes) |
| `cloacina-provider-quorum` | Quorum reactor-constructor suite (constructor e2e lanes) |
| `cloacina-provider-sensor` | Sensor trigger/accumulator-constructor suite (constructor e2e lanes) |
| `constructor-contract` | The shared contract crate the fixtures compile against |

## Test fixtures (executed by `angreal test` lanes, not demos)

Deliberately minimal crates that exist to be compiled/loaded by integration
and e2e suites. Not examples; do not model user code on them.

| Directory | Consuming lane |
|---|---|
| `accumulator-constructor-fixture` | constructor integration suite |
| `reactor-constructor-fixture` | constructor integration suite |
| `task-constructor-macro-fixture` | macro expansion tests |
| `task-constructor-twocfg-fixture` | macro expansion tests (two `#[config]` fields) |
| `trigger-constructor-macro-fixture` | macro expansion tests |
| `native-task-provider-fixture` | native provider load path (T-0902) |
| `packaged-consumer-fixture` | packaged `constructor!` consumption tests |
| `provider-consumer-fixture` | provider resolution tests |
