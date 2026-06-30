# fs-grant-demo — constructor capability grants, end-to-end (CLOACI-T-0834)

A runnable security demo of cloacina's **constructor capability-grant** model. It
proves the default-closed guarantee: a WASM task constructor can reach the host
filesystem **only** when the consuming workflow's tenant explicitly grants it.

## What it proves

There is exactly **one** constructor, `read_file`, authored in the sibling crate
[`../fs-grant-constructor`](../fs-grant-constructor). Its entire body is:

```rust
let contents = std::fs::read_to_string(&self.path)?; // inside a WASM sandbox
self.set("contents", contents);
```

This demo packages that constructor into a WASM provider and runs it in **two**
`#[workflow]`s that differ in exactly one line:

| Workflow    | Grant at the `constructor!(...)` site         | Outcome                                  |
|-------------|-----------------------------------------------|------------------------------------------|
| `granted`   | `grants = { fs = ["ro:/tmp/cloacina-fs-grant-demo"] }` | reads the secret through the sandbox |
| `ungranted` | *(no `grants` field — default-closed)*        | the read is **denied**; the node fails   |

The constructor code is identical in both cases. What changes is the **tenant's**
grant at the call site. Enforcement is entirely host-side (fidius): with no `fs`
grant the guest gets a zero-capability `WasiCtx`, so the read reaches nothing. A
constructor can never widen its own access.

If the `ungranted` workflow ever manages to read the secret, the demo prints a loud
SECURITY FAILURE banner and exits non-zero.

## Running

```sh
cd examples/constructor-contract/fs-grant-demo
cargo run
```

Requires the `wasm32-wasip2` target (used to build the constructor component):

```sh
rustup target add wasm32-wasip2
```

The first run is **slow**: it compiles the constructor crate to a `wasm32-wasip2`
component and builds cloacina with the `constructors-wasm` (wasmtime) feature. The
demo writes its secret to `/tmp/cloacina-fs-grant-demo/secret.txt`, so it assumes a
unix-like host (macOS/Linux); it is not intended for Windows.

## Expected output (abridged)

```
--- Case 1: `granted` (fs = ["ro:/tmp/cloacina-fs-grant-demo"]) ---
    [granted]   SUCCESS: constructor read the secret THROUGH the grant: "the launch codes are 0000"

--- Case 2: `ungranted` (no `grants` field — default-closed) ---
    [ungranted] DENIED as expected (no fs grant): ...
```

The process exits `0` on the expected granted-reads / ungranted-denied outcome.
