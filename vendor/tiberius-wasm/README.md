# tiberius-wasm

A minimal maintained fork of [tiberius](https://github.com/prisma/tiberius)
0.12.3 (upstream is dormant), moved from weir's `weir-tiberius` with the MSSQL
connector (COLLIERY-I-0633, CLOACINA-T-0984). It exists so
the `mssql` connector — a wasm32-wasip2 guest — can speak TLS to
Force-Encryption SQL Server. The lib target is still named `tiberius`, so the
connector's imports are unchanged.

## The delta (everything else is upstream 0.12.3 verbatim)

1. **`src/client/tls_stream/rustls_tls_stream.rs` — rewritten.** The stock
   `rustls` backend (tokio-rustls 0.24 + tokio-util compat + native certs)
   cannot build for wasm32-wasip2. The rewrite drives **rustls 0.23 directly**
   with the pure-Rust `rustls-rustcrypto` provider and compiled-in webpki
   roots. Crucially, the handshake loop terminates on `!is_handshaking()` — a
   `wants_read()` drain (what futures-rustls does) deadlocks on always-ready
   blocking transports like the guest's SyncSock, blocking forever on a TDS
   header the server never sends. The TDS 7.x in-PRELOGIN tunneling is
   upstream's own `TlsPreloginWrapper`, untouched.
2. **`Config::trust_cert_pem` (inline-PEM trust).** Guests have no filesystem;
   path-based `trust_cert_ca` is unusable there. New `TrustConfig::CaCertificatePem`.
3. **32-bit PLP sentinel** (`src/tds/codec/type_info.rs`): upstream's
   `0xfffffffffffffffe_usize` literals only compile on wasm32 because
   dependency lint-capping silences the overflow; now an explicit
   truncating-cast constant with the routing invariant documented.
4. Manifest trimmed to the deps the kept features need (`tds73`, `rustls`,
   value-type optionals); tokio/native-tls/opentls/sql-browser paths dropped.

## Known limitations

- **TDS 8.0 "strict" (TLS-from-byte-0) is unsupported** — no Rust client
  exists for it; tiberius tops out at TDS 7.4. Azure SQL's default (7.x +
  Force Encryption) works. Watch Microsoft's `mssql-tds-preview` for a future
  strict-mode client ([[WEIR-A-0041]] review trigger).
- `rustls-rustcrypto` is an alpha, unaudited provider — the recorded
  [[WEIR-A-0041]] trade-off, with review triggers on the ADR.

Wire-proof gates live in `crates/weir-engine/tests/wasm_mssql_engine.rs`
(`mssql_tls_*`, against the `mssql-tls` Force-Encryption compose service).
