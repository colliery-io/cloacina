---
title: "REST Source"
description: "The runtime of every declarative source: paged, incremental and partitioned reads of a JSON API."
sidebar:
  order: 50
---

`connectors/rest`, package `cloacina-connector-rest`. The source half of the
REST runtime: it reads one stream of a [connector
definition](../definitions/), compiled by the host into a
`cloacina_constructor_contract::sync::rest::RestConfig`. A sync does not name it
directly; it names a definition. Moved from weir (CLOACINA-T-0990).

## How it reads

- **One page at a time.** Each page of records is followed by a checkpoint
  whose opaque state holds the paginator's position, so a failure at page N
  loses only page N, and a resumed read goes on from there.
- **`max_pages`** (default 1000) stops a read at a checkpoint, with a warning
  in the log; the next read resumes from it.
- **Pagination** by page number, offset, a next-page token (from the answer,
  or from the last record), or the `Link` header. Parameters go in the query
  string or, for a POST, into the JSON body.
- **Incremental** reads send the committed cursor as a parameter and advance
  it to the largest value seen, numbers compared as numbers.
- **Partitions**: one read per value of a list, or per record of a parent
  read.
- **End of data**: a short or empty page, an absent token, a `false` at the
  stop path, or a `404` or empty body past page 1. A non-2xx status
  otherwise fails the read; a `401` is transient, so a retry mints the
  credential again.
- **Retries**: 429, 5xx and transport failures retry with backoff,
  honouring `Retry-After`, up to `max_retries` (default 4).
- **`check`** fetches the first page of the first slice.

Auth is not in the guest: the host adds the credential to each request the
grants allow.

## Testing

`cloacina_testing::connectors::rest_mock` is a mock JSON API that pages in
every style, filters by an incremental parameter, requires a header, and keeps
what it was sent. The conformance file runs against it:

```bash
cargo test -p cloacina-sync --test rest_connector
```
