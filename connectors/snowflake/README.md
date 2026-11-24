# cloacina-connector-snowflake

The Snowflake connector of Sync: a source and a destination over the
Snowflake SQL API, as one `wasm32-wasip2` component.

- **Source** reads a table, or a configured SELECT, as one statement. Full
  refresh reads it all; incremental wraps it with `WHERE <cursor> > ?` and
  orders by the cursor. Rows arrive across every result partition and become
  JSON objects with lowercased field names. The cursor is the largest value
  seen, as a JSON string, compared numerically when both sides are numbers.
- **Destination** creates the table from the first record's fields, then
  writes in chunks: `INSERT` to append, `MERGE` by business keys to upsert
  (idempotent on replay), `TRUNCATE` once for overwrite, and `DELETE` by key
  for CDC deletes.
- **Auth** is not in the guest. A sync names a `snowflake_keypair_jwt`
  credential, and the host signs a JWT with the private key and adds it to
  each request the grants allow.
- A `202` answer polls the statement handle until it settles; 429 and 5xx
  retry with backoff, honouring `Retry-After`.

Config: `account`, `database`, `schema`, `warehouse`, `role`; `table`
(default: the stream name); `query` (the source SELECT); `base_url` (default
`https://<account>.snowflakecomputing.com`; PrivateLink and tests set it);
`batch_rows` (rows per statement, default 500); `max_retries` (default 4).
Capability: `http` to the API host.

`check` runs `SELECT 1`, so a wrong account, endpoint or key fails it.

The conformance file is `conformance.toml`; it runs against the mock SQL API
in `cloacina_testing::connectors::snowflake_mock`. A live run is opt-in.
Moved from weir (COLLIERY-I-0633 / CLOACINA-T-0986).
