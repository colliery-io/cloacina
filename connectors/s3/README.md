# cloacina-connector-s3

The S3 connector of Sync: a source that lists and reads NDJSON objects from an
S3-compatible store (AWS S3, MinIO) over HTTP, as one `wasm32-wasip2`
component. Each non-empty line of an object is a record.

- **Full refresh** reads every object under the prefix.
- **Incremental** uses the object key as the cursor (`start-after`), so
  append-only key schemes (date partitions, ULIDs) resume cleanly. The cursor
  is the last key read, as a JSON string.
- **Auth** is not in the guest. A sync names an `aws_sigv4` credential
  (`source_credential`), and the host signs each request the grants allow.
  The secret never enters the sandbox.
- Listings follow continuation tokens to the end; a single page would stop at
  1000 objects and report success.

Config: `endpoint` (`https://s3.amazonaws.com`, `http://minio:9000`),
`bucket`, `prefix` (default: the whole bucket), `max_keys` (the listing page
size; default S3's 1000). Capability: `http` to the endpoint.

`check` lists one key, so a wrong endpoint, bucket or key fails it.

The conformance file is `conformance.toml`; it runs against MinIO in the
integration lane. Moved from weir (COLLIERY-I-0633 / CLOACINA-T-0985).
