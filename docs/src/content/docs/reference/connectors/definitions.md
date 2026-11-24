---
title: "Connector Definition Format"
description: "The YAML a declarative connector is written in: a source's streams, a destination's objects, and their auth."
sidebar:
  order: 45
---

A connector definition is YAML. Every field is checked: an unknown or
misspelled field is an error that names it. The model lives in
`cloacina_declarative::definition`; an Airbyte low-code source is imported into
it (see [Import an Airbyte Connector](../../../how-to/sync/import-an-airbyte-connector/)).

## A source

```yaml
spec:
  name: example                 # the name a sync names it by
  version: 0.1.0                # optional
base_url: "https://{{ config['subdomain'] }}.example.com/api"
auth:
  kind: bearer
  field: api_token              # the field of the sync's secret
streams:
  - name: posts
    path: /posts
    primary_key: [id]
    schema:
      - { name: id, type: int64, nullable: false }
      - { name: title, type: utf8 }
      - { name: updated_at, type: timestamp }
    record_selector: data       # a dot-path to the records; absent: the body is the list
    incremental:
      cursor_field: updated_at
      cursor_param: since       # empty: track the cursor, send nothing
      start_value: "{{ config['start_date'] }}"
    pagination: { kind: page, page_param: page, size_param: per_page, size: 50 }
```

| Stream field | Meaning |
|---|---|
| `name`, `path` | The stream, and the path after `base_url`. |
| `primary_key` | The stream's key. |
| `schema` | Fields with `type` `int64`, `float64`, `utf8`, `bool` or `timestamp`, and `nullable` (default true). The sync enforces it unless it gives its own. |
| `record_selector` | A dot-path to the records in the answer. |
| `incremental` | `cursor_field`; `cursor_param` (sent as a query parameter, or with `inject_into: body`, as a dot-path of the POST body); `cursor_value_path`; `start_value`; `end_param` and `end_value`. |
| `pagination` | `page` (`page_param`, `size_param`, `size`); `offset` (`offset_param`, `limit_param`, `size`); `cursor` (`token_param`, and `cursor_path` or `cursor_record_field`; `stop_on_false_path`; `size_param`, `size`); or `link_header`. Each may `inject_into: body`. |
| `partition` | `list` (`field`, `values`) or `substream` (`field`, `parent_path`, `parent_key`, `parent_record_selector`). The value replaces `{{ stream_partition.<field> }}` in `path`. |
| `header_row` | The records are row arrays whose first row names the fields. |
| `http_method`, `request_body`, `request_headers` | `GET` or `POST`, a JSON body, static headers. |
| `mapping` | Transforms applied before the stream's own mapping. |

`{{ config['key'] }}` may appear in `base_url`, `path`, `request_body`, a
header value, `start_value`, `end_value`, a partition value and a parent path.
It is rendered from the sync side's config when the side is lowered.

## A destination

```yaml
spec: { name: acme-crm }
base_url: https://api.acme.com
auth: { kind: bearer, field: token }
objects:
  - name: contacts              # matched by the stream's name
    path: "/crm/contacts/{{ record.email }}"
    method: PATCH               # absent: PATCH to upsert, POST otherwise
    upsert_key: email
    field_map: [[email, email], [full_name, name]]
    body_wrap: properties
    delete_path: "/crm/contacts/{{ record.email }}"   # for CDC deletes
```

`{{ record.<field> }}` is replaced per record by the record's value,
percent-encoded.

## Auth

`auth` names a scheme and the fields of the sync's secret; the host builds the
credential from them and adds it to each request.

| `kind` | Fields | Becomes |
|---|---|---|
| `none` | | No credential. |
| `bearer` | `field` (default `token`) | `Authorization: Bearer <field>` |
| `api_key` | `header`, `field` | A header carrying the field. |
| `api_key_query` | `param`, `field` | A query parameter carrying the field. |
| `basic` | `username_field`, `password_field` | HTTP Basic. |
| `oauth2` | `token_url`, `grant` (`client_credentials` or `refresh_token`), `scopes` | A token minted on the host from the secret's `client_id`, `client_secret` and `refresh_token`. |
| `session_token` | `login_url`, `token_path`, `inject_header` | A token from a login, sent as a header. |
| `google_service_account` | `scopes` | A token minted from the secret's `service_account_key`. |
| `snowflake_keypair_jwt` | `account_key`, `user_key` | A key-pair JWT from the secret's `private_key`; account and user from the config. |

See [Credential Injection](../../../explanation/primitives/sync/credentials/).
