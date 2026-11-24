# cloacina-connector-mssql

The Microsoft SQL Server connector of Sync: a source (full refresh,
incremental by cursor) as one `wasm32-wasip2` component. A sync TDS client
over fidius's TCP egress through `tiberius-wasm` (in `vendor/`), with TLS
through rustls. Reads use `FOR JSON PATH`, so the server produces the JSON
rows. No CDC, and no destination.

Config: `host`, `port` (default 1433), `database`, `user`, `password`;
`table` (default: the stream name, `schema.table` allowed); `tls_mode`
(`require` by default, `prefer`, `disable`); `tls_ca_pem` (an inline PEM CA
bundle; default: the public webpki roots); `tls_insecure_skip_verify`
(encrypt without verifying the certificate). Capability: `tcp` to the
database host.

An incremental read compares the cursor column in its own type: SQL Server
converts the bound cursor text to the column's type, so an integer cursor at
9 still reads 10. The next cursor is the largest value seen, compared
numerically when both sides are numbers.

The conformance file is `conformance.toml`; it runs against SQL Server in the
integration lane. Moved from weir (COLLIERY-I-0633 / CLOACINA-T-0984).
