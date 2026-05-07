# API Design Review

## Summary

Cloacina exposes seven distinct interface surfaces — Rust library, three authoring crates, Python (`cloaca`), REST/HTTP, WebSocket, CLI (`cloacinactl`), and the fidius plugin ABI — and the consistency profile across them is uneven. The CLI's noun-verb shape is genuinely good (one purpose per file, predictable verb palette), the REST `ApiError` envelope is the right shape (`{error, code}` with codes worth grepping), the unified DAL accessor pattern carries through to the Python builder objects cleanly, and the plugin-ABI versioning has the right primitive — `#[optional(since = N)]` capability bits give the host a clean fallback for older plugins. The system has been thinking carefully about contracts.

The damage is concentrated where two surfaces meet: **the CLI talks to the server in three places where the server doesn't actually accept what the CLI sends**. `cloacinactl tenant create my_tenant --description "..."` posts `{name, description}` to `POST /v1/tenants`, but the server's `CreateTenantRequest` deserializes `{schema_name, username, password}` — every tenant create from the published CLI has been failing with a 400. `cloacinactl execution list --workflow=X --status=Y --limit=N` adds those as query parameters, but `executions::list_executions` ignores all three and returns only Pending+Running rows from `get_active_executions()` regardless. `cloacinactl tenant list`, `trigger list`, and `execution list` all hand a wrapping object (`{"tenants":[...]}`) to `render::list`, which `body.as_array()` against and silently renders empty when the cast fails. Three CLI commands that look complete are non-functional or misleading; none of them have integration tests catching the mismatch.

Beyond the CLI/server seam, there's structural drift. The Rust library docs (`crates/cloacina/src/lib.rs:75-105`) walk the reader through a `workflow!` declarative-macro syntax that doesn't exist (the actual API is `#[workflow(name = "…")] pub mod`), and the prelude doc still advertises that macro. The plugin macro `cloacina::package!()` doc says "Six methods" while the macro emits nine. The CLI's `package pack --sign <key>` flag is silently ignored with an `eprintln!` note — the documented contract is not implemented. The auth model has two orthogonal axes (`is_admin` "god mode" boolean and `permissions` string of `admin`/`write`/`read`) that are not documented anywhere in the OpenAPI/CLI sense, and `WsTokenSource::QueryTicket` plus `WsTokenSource::Header` use overlapping but different validation paths that aren't surfaced to the consumer. The list goes on, but the pattern is uniform: the inner crate's contracts are good, the consumer-facing edges have not been audited end-to-end.

## Interface Inventory

1. **Rust library API** (`cloacina::*`, ~80 re-exports at `crates/cloacina/src/lib.rs:540-588`): `DefaultRunner` + `DefaultRunnerBuilder` + `DefaultRunnerConfig`, `Runtime`, `Context`, `Task`/`TaskNamespace`, `Workflow`/`WorkflowBuilder`, `Trigger`/`TriggerResult`, retry types, error enums (`TaskError`, `WorkflowError`, `ExecutorError`, `ValidationError`, `RegistrationError`, `SubgraphError`, `CheckpointError`, `ContextError`, `TriggerError`), `TaskScheduler`/`Scheduler` (cron+trigger), `WorkflowExecutor` trait, dispatcher traits, computation-graph types, Universal* DB types, plus the macros (`#[task]`, `#[workflow]`, `#[trigger]`, `#[reactor]`, `#[computation_graph]`, accumulator macros).

2. **Authoring crates** — three "leaf" crates that packaged cdylibs compile against:
   - `cloacina-workflow` (`crates/cloacina-workflow/src/lib.rs`) — `Context`, `Task`, `TaskNamespace`, `Trigger`, `TriggerResult`, retry types, `cloacina_workflow::__private::tokio` re-export for macro-emitted code.
   - `cloacina-computation-graph` (`crates/cloacina-computation-graph/src/lib.rs`) — `InputCache`, `GraphResult`, `SourceName`, `ReactionMode`, `Reactor` trait, `Graph` trait, `ReactorRegistration`.
   - `cloacina-workflow-plugin` (`crates/cloacina-workflow-plugin/src/lib.rs`) — `CloacinaPlugin` v2 trait (9 methods, methods 4-8 `#[optional(since = 2)]`), wire-format types (`TaskMetadataEntry`, `PackageTasksMetadata`, `GraphPackageMetadata`, `ReactorPackageMetadata`, `TriggerPackageMetadata`, `TriggerlessGraphMetadataEntry`, request/result pairs), `CloacinaMetadata` (manifest schema), method index constants, the `cloacina::package!()` shell macro.

3. **Python public API** (`cloaca` module, `crates/cloacina-python/src/lib.rs:87-155`): decorators (`@task`, `@trigger`, `@reactor`, accumulator decorators, `@node`); classes (`Context`, `WorkflowBuilder`, `Workflow`, `DefaultRunner`, `DefaultRunnerConfig`, `WorkflowResult`, `TaskHandle`, `TaskNamespace`, `WorkflowContext`, `RetryPolicy`/`Builder`/`BackoffStrategy`/`RetryCondition`, `TriggerResult`, `ComputationGraphBuilder`, `DatabaseAdmin`/`TenantConfig`/`TenantCredentials` postgres-only); functions (`register_workflow`, `var`, `var_or`).

4. **REST/HTTP API** (`crates/cloacina-server/src/routes/`, mounted in `build_router` at `lib.rs:371-492`):
   - Public: `/health`, `/ready`, `/metrics`.
   - `/v1/auth/keys` (POST/GET/DELETE), `/v1/auth/ws-ticket` (POST).
   - `/v1/tenants` (POST/GET), `/v1/tenants/{schema}` (DELETE), `/v1/tenants/{tenant_id}/keys` (POST).
   - `/v1/tenants/{tenant_id}/workflows` (POST multipart, GET), `/v1/tenants/{tenant_id}/workflows/{name}` (GET), `/v1/tenants/{tenant_id}/workflows/{name}/{version}` (DELETE).
   - `/v1/tenants/{tenant_id}/triggers` (GET), `/v1/tenants/{tenant_id}/triggers/{name}` (GET).
   - `/v1/tenants/{tenant_id}/workflows/{name}/execute` (POST), `/v1/tenants/{tenant_id}/executions` (GET), `/v1/tenants/{tenant_id}/executions/{exec_id}` (GET), `/v1/tenants/{tenant_id}/executions/{exec_id}/events` (GET).
   - Graph health (auth-required, **not nested under `/v1/`** — merged at root): `/v1/health/accumulators`, `/v1/health/graphs`, `/v1/health/graphs/{name}`.
   - Body limit: 100 MB (`DefaultBodyLimit::max(100 * 1024 * 1024)` at `lib.rs:486`).
   - Error envelope: `{"error": "<message>", "code": "<slug>"}`. Request ID propagated via `x-request-id` response header.

5. **WebSocket API**:
   - `GET /v1/ws/accumulator/{name}` — producer pushes binary frames; auth via `Authorization: Bearer` header **or** `?token=<single-use-ticket>` query param. Text frames rejected with a warn.
   - `GET /v1/ws/reactor/{name}` — operator commands as JSON `ReactorCommand`; responses as JSON `ReactorResponse`. Per-command authZ (`ReactorOp` mapping at `ws.rs:321-330`).

6. **CLI (`cloacinactl`)**: top-level subcommands `daemon|server|compiler|package|workflow|graph|execution|tenant|key|trigger|status|config|admin|completions`. Verbs per noun in `crates/cloacinactl/src/nouns/*/`. Global flags: `--verbose`, `--home`, `--profile`, `--server`, `--api-key` (raw / `env:VAR` / `file:PATH`), `--tenant`, `--json`, `-o {table|json|yaml|id}`, `--no-color`. Exit codes per ADR-0003: 1 user, 2 network, 3 not-found, 4 auth, 5 server-reject (`crates/cloacinactl/src/shared/error.rs:43-55`).

7. **Plugin/fidius ABI** (`crates/cloacina-workflow-plugin/src/lib.rs`): `CloacinaPlugin` trait, `version = 2`, `buffer = PluginAllocated`. 9 methods, indices `METHOD_GET_TASK_METADATA = 0` … `METHOD_INVOKE_TRIGGERLESS_GRAPH = 8`. Methods 4-8 marked `#[optional(since = 2)]`. Wire format: debug=JSON / release=bincode (per project memory; not encoded in code anywhere). Plugin emission via `cloacina::package!();` at crate root, with a `__cloacina_package_marker` module guard against double-emission.

## Consistency Assessment

**Where conventions hold.**
- The CLI noun-verb shape is rigorous: every verb is a separate file, the verb palette is consistent (`list`/`inspect`/`status`/`create`/`delete`/`revoke`/`upload`/`pack`/etc.), `--force` is the universal destructive-confirmation flag, exit codes are typed.
- REST routes follow a single resource template: `/v1/<auth-path>` for global keys/tenants and `/v1/tenants/{tenant_id}/<resource>` for tenant-scoped. Method semantics are RESTful (POST=create with 201, GET=read, DELETE=remove).
- `ApiError` is universally used across mutation handlers; the `{error, code}` shape is uniform.
- The Rust public-error enums use `thiserror`, follow `<Subsystem>Error` naming, and are surfaced through the prelude. The `ValidationError` / `WorkflowError` / `ExecutorError` distinction is clean (validation = static graph problem; workflow = construction; executor = runtime).
- The plugin ABI's `#[optional(since = 2)]` model is applied consistently — every post-version-1 method is gated, the host treats `CallError::NotImplemented` uniformly.
- The Universal-types pattern (`UniversalUuid`, `UniversalTimestamp`, `UniversalBool`, `UniversalBinary`) gives DAL methods one shape across both backends.

**Where conventions break.**
- **CLI/server contract drift** is the dominant inconsistency. `tenant create` body shape is wrong; `execution list` filters are silently ignored; `tenant/execution/trigger list` strip the wrong field (see API-01, API-02, API-03).
- **Auth checks** use four near-identical helpers (`forbidden_response`, `admin_required_response`, `insufficient_role_response`, plus open-coded `auth.is_admin` checks in `tenants.rs:56` and `keys.rs:202`). The codes differ (`tenant_access_denied`, `admin_required`, `insufficient_permissions`, no code for the open-coded path) but the underlying semantics overlap. (See API-04.)
- **Error envelope variants**: most of the server uses `ApiError` → `{error, code}`. But `auth.rs:158, 165` returns `Json(serde_json::json!({"error": "..."}))` directly (no `code`); `health_graphs.rs:113` returns `{"error": "graph 'name' not found"}` (no `code`); the CLI's `extract_message` only knows how to parse one of these three shapes.
- **Pagination/limits** are ad-hoc. `dal.schedule().list(None, false, 100, 0)` (hard-coded 100 in the trigger handler), `dal.workflow_execution().get_active_executions()` (no limit at all), `dal.execution_event().list_by_workflow(id)` (no limit visible at the route layer). The CLI's `--limit` is ignored.
- **Naming**: `Scheduler` (the unified cron+trigger scheduler) vs `TaskScheduler` (the workflow planner) vs `ComputationGraphScheduler` — three top-level types with the most generic possible name, one of them `Scheduler` (LEG-11).
- **Macro/trait method count**: the `package!()` macro doc says "Six methods" while the trait has nine (LEG-05).
- **Public docs vs reality**: the engine `lib.rs` quick-start uses a `workflow!` macro that doesn't exist (LEG-04); the prelude doc claims `workflow!` is exported (line 452); `runtime.get_workflow` has been renamed but the example is unupdated.
- **Auth role model**: `is_admin` (DB column, "god mode") vs `permissions` (string column with values `admin`/`write`/`read`) vs `tenant_id` (scope) compose into authorization rules that are spread across `can_admin()`, `can_write()`, `can_access_tenant()` plus open-coded `auth.is_admin` checks. There is no docs page that explains the matrix.

## Findings

### API-01: `cloacinactl tenant create` posts a body shape the server cannot deserialize — every tenant-create from the CLI has been failing

**Severity**: Critical
**Location**: `crates/cloacinactl/src/nouns/tenant/mod.rs:55-58`; `crates/cloacina-server/src/routes/tenants.rs:38-47`
**Confidence**: High

#### Description

The CLI sends `POST /v1/tenants` with body `{"name": <name>, "description": <description>}`. The server's `CreateTenantRequest` (`tenants.rs:39-47`) deserializes `{"schema_name": String, "username": String, "password": String}`. Both fields the CLI sends (`name`, `description`) are absent on the server side; both fields the server requires (`schema_name`, `username`) are absent on the CLI side. axum's JSON extractor rejects on missing required fields with a 400; even if it didn't, `body.schema_name = ""` would create a tenant with an empty schema name.

This means a user following `cloacinactl tenant create my_tenant` cannot create a tenant — they have to either drop to `curl` or invoke `DatabaseAdmin` from a Rust program. The CLI was published, the server route was published, and the contract was never verified end-to-end.

#### Evidence

```rust
// crates/cloacinactl/src/nouns/tenant/mod.rs:55-59
TenantVerb::Create { name, description } => {
    let body = serde_json::json!({
        "name": name,
        "description": description,
    });
    let resp: serde_json::Value = client.post("/v1/tenants", &body).await?;
```

```rust
// crates/cloacina-server/src/routes/tenants.rs:39-47
#[derive(Deserialize)]
pub struct CreateTenantRequest {
    pub schema_name: String,
    pub username: String,
    #[serde(default)]
    pub password: String,
}
```

The CLI also drops the `--password` flag entirely — the server expects one (or auto-generates if empty), but the CLI has no way to convey it.

#### Suggested Resolution

Pick one of three resolutions:
1. **Fix the CLI**: change `TenantVerb::Create` to `Create { schema_name: String, username: String, #[arg(long)] password: Option<String> }` and post the matching body.
2. **Fix the server**: rename `CreateTenantRequest.schema_name` → `name` and treat `username` as `Option<String>` defaulting to the schema name. Add `description` as an optional field that's stored in metadata.
3. **Add a contract test**: an integration test that posts what the CLI posts and asserts the server accepts it. This finding's existence implies no such test exists today.

**Cross-cutting note**: Operability should add this to the soak-test gap inventory.

---

### API-02: `cloacinactl execution list --workflow=X --status=Y --limit=N` silently ignores all three filters; the server returns only active executions

**Severity**: Critical
**Location**: `crates/cloacinactl/src/nouns/execution/mod.rs:71-83`; `crates/cloacina-server/src/routes/executions.rs:101-151`
**Confidence**: High

#### Description

The CLI documents three filters on `execution list` (`--workflow`, `--status`, `--limit`) and serializes them as query parameters in the URL. The server's `list_executions` handler accepts `Path(tenant_id)` and `State(state)` only — no `Query` extractor, no filter parsing. It always calls `dal.workflow_execution().get_active_executions()` which returns *all* `Pending`+`Running` rows for the tenant with no LIMIT and no projection. Completed, Failed, and Cancelled executions are never returned by this endpoint.

So the user calling `cloacinactl execution list --status Failed --limit 10` gets:
- The full list of every Pending/Running workflow in the tenant (no limit).
- No matches against `--status Failed` because Failed is never returned.
- No matches against `--workflow X` because no filter is applied.

The CLI's help text creates a contract; the server doesn't honor it. The route name "list executions" suggests a paginated catalog; the implementation is "current activity dashboard."

#### Evidence

```rust
// crates/cloacinactl/src/nouns/execution/mod.rs:71-83
let mut query = format!("?limit={limit}");
if let Some(w) = workflow { query.push_str(&format!("&workflow={w}")); }
if let Some(s) = status { query.push_str(&format!("&status={s}")); }
let body: serde_json::Value = client
    .get(&format!("/v1/tenants/{tenant}/executions{query}"))
    .await?;
```

```rust
// crates/cloacina-server/src/routes/executions.rs:102-151
pub async fn list_executions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    Path(tenant_id): Path<String>,
) -> impl IntoResponse {
    // ...
    match dal.workflow_execution().get_active_executions().await {
```

`get_active_executions` filters by `status.eq_any(["Pending","Running"])` (`workflow_execution.rs:259-261`) — completed executions are unreachable through this endpoint.

#### Suggested Resolution

Add a `Query<ListExecutionsQuery>` extractor with `workflow_name: Option<String>`, `status: Option<String>`, `limit: u32 = 50`, `offset: u32 = 0`. Switch the DAL call to `list_recent` (which already exists at `workflow_execution.rs:1051` and takes a limit) plus an optional status filter. Document the default-limit behavior.

If the intent is to keep "list active executions" as the semantics: rename the route `/v1/tenants/{tenant_id}/executions/active` and add a separate `/executions` endpoint with full filtering. The current name promises more than it delivers.

---

### API-03: `cloacinactl tenant list / trigger list / execution list` pass a non-array JSON envelope to a renderer that demands an array — output is silently empty

**Severity**: Major
**Location**: `crates/cloacinactl/src/nouns/tenant/mod.rs:62-64`; `crates/cloacinactl/src/nouns/trigger/mod.rs:46-50`; `crates/cloacinactl/src/nouns/execution/mod.rs:79-82`; `crates/cloacinactl/src/shared/render.rs:26-27`
**Confidence**: High

#### Description

The shared renderer `render::list(body, format)` (`render.rs:26-27`) does `let items: Vec<Value> = body.as_array().cloned().unwrap_or_default();`. That works only if the caller hands in a JSON array. Three CLI commands hand in the server's wrapping object instead:

- `tenant list` calls `render::list(&body, output)` where `body` is `{"tenants": [...]}` (`tenants.rs:130-134`). `as_array()` returns `None`. Output is an empty list with no error.
- `trigger list` does the same — server returns `{"tenant_id":..., "schedules": [...]}` (`triggers.rs:61-65`).
- `execution list` does the same — server returns `{"tenant_id":..., "executions": [...]}` (`executions.rs:137-140`).

By contrast, `workflow list` (`workflow/mod.rs:65`) and `key list` (`key/mod.rs:99-101`) and `package list` (`package/list.rs:35-39`) explicitly extract the inner array first via `.get("workflows")` / `.get("keys")` / `.get("workflows")`. The mistake on the three broken paths was an honest one and would have been caught by even one integration test that checked the rendered output included expected items.

The user-visible failure mode is silent: the command succeeds (exit 0), produces a "No items." line in table mode (or `[]` in JSON), and the user thinks they have no tenants/triggers/executions. There's no "field not found, did you mean ..." hint, no warning.

#### Evidence

```rust
// crates/cloacinactl/src/nouns/tenant/mod.rs:63-64
let body: serde_json::Value = client.get("/v1/tenants").await?;
render::list(&body, output)  // body is {"tenants": [...]} — never an array
```

vs. the working pattern in `workflow/mod.rs:65`:
```rust
let workflows = body.get("workflows").cloned().unwrap_or(body.clone());
```

#### Suggested Resolution

Two layers of fix:
1. Fix each broken caller: `let items = body.get("tenants").cloned().unwrap_or(body); render::list(&items, output)`.
2. Make the helper less footgun-y: `render::list` should accept either an array or an object with a single array-valued field, and warn (or error) if neither matches. Or — better — make every server list endpoint return a top-level array and rename the wrapping field; the wrapper's only purpose is to attach `tenant_id` for logging.

---

### API-04: Auth role model is encoded in three orthogonal axes and four helper methods, none of which is documented in one place

**Severity**: Major
**Location**: `crates/cloacina-server/src/routes/auth.rs:42-49, 217-253`; `crates/cloacina-server/src/routes/keys.rs:36-44`; `crates/cloacina-server/src/routes/tenants.rs:56, 97, 123`; `crates/cloacina-server/src/routes/keys.rs:202`
**Confidence**: High

#### Description

`AuthenticatedKey` carries `permissions: String` (semantic values `"admin"`/`"write"`/`"read"`), `tenant_id: Option<String>`, and `is_admin: bool`. The latter is "god mode" — bypasses tenant scoping. The matrix:

- `can_access_tenant(t)` — `is_admin` ∨ (`tenant_id == Some(t)`) ∨ (`tenant_id == None && t == "public"`).
- `can_write()` — `is_admin` ∨ (`permissions ∈ {"admin","write"}`).
- `can_admin()` — `is_admin` ∨ (`permissions == "admin"`).
- Open-coded `auth.is_admin` check (no helper) in `tenants.rs:56, 97, 123` (tenant-create/remove/list) and `keys.rs:202` (`create_tenant_key`).

The forbidden-response codes vary: `tenant_access_denied` from `forbidden_response()`, `admin_required` from `admin_required_response()`, `insufficient_permissions` from `insufficient_role_response()`, plus an undocumented "the body just doesn't match" 400 on missing fields. There is no central document — neither a rustdoc page nor an OpenAPI description nor a `docs/auth.md` — that lays out the role matrix. The CLI's `Role::Admin/Write/Read` enum (`key/mod.rs:32-37`) is the only place a user can see all three values; nothing tells them that selecting `admin` gives `permissions=admin` but **not** `is_admin=true` (the latter is granted only via the bootstrap key — a comment in `keys.rs:80-89` is the only place this rule is articulated).

For a system pitched at multi-tenant deployments, an undocumented dual-permission model is a real ergonomic and security hazard.

#### Evidence

- `crates/cloacina-server/src/routes/auth.rs:240-248` — `can_write` / `can_admin` definitions.
- `crates/cloacina-server/src/routes/auth.rs:217-225` — `can_access_tenant` definition with the special `"public"` tenant fall-through.
- `crates/cloacina-server/src/routes/keys.rs:80-90` — the only inline doc explaining "permissions=admin gets tenant-admin, not god-mode".
- `crates/cloacina-server/src/routes/tenants.rs:56` — `if !auth.is_admin { return AuthenticatedKey::admin_required_response().into_response(); }` — open-coded, doesn't go through `can_admin()`. So a key with `permissions=admin` (but `is_admin=false`) cannot create tenants, but a developer reading `can_admin()` would think it could.
- The DB schema gives `is_admin` and `permissions` independently and the API exposes setting `permissions` only; there's no documented way to elevate a key from tenant-admin to god-mode through the REST API (you have to hit the DB directly or use the bootstrap key).

#### Suggested Resolution

1. Write a one-page auth model doc: define `is_admin` as "cross-tenant override only granted at server bootstrap" and `permissions` as "tenant-scoped role". Pin which routes require which.
2. Replace the open-coded `auth.is_admin` checks with a `require_god_mode()` helper that returns the same `admin_required_response` everywhere; this also makes the distinction grep-able.
3. Add unit tests for the matrix: 4 roles × 3 routes = a small grid of "this combination should/shouldn't pass".
4. The CLI's `--role admin` should warn/explain that this doesn't grant cross-tenant access.

**Cross-cutting note**: Security review territory.

---

### API-05: `cloacinactl package pack --sign <key>` accepts the flag and silently does nothing — the contract is documented but unimplemented

**Severity**: Major
**Location**: `crates/cloacinactl/src/nouns/package/pack.rs:32-41`
**Confidence**: High

#### Description

The `pack` verb's `--sign <key>` flag is shown in help text and documented in `nouns/package/mod.rs:55-57` ("Sign the archive with this Ed25519 key file"). The implementation reads the flag, prints `note: --sign <path> accepted but ignored — detached signature side-car generation is not implemented in the CLI yet.`, and produces an unsigned archive.

The server's `--require-signatures` mode then refuses the upload (with a meaningful error per LEG-coverage) — but the user has been told their package is signed. The packaging path documented in W5 (`signed → package_signatures table on upload`) does not work from the CLI alone.

This is a design surface that exists in the type system but not in the runtime; either the flag should error (`unimplemented_signing_feature`) or the implementation should be wired up. Half-complete is the worst state — users encounter it as "the system says it signed but the server rejects".

#### Evidence

```rust
// crates/cloacinactl/src/nouns/package/pack.rs:32-41
if let Some(key_path) = sign {
    eprintln!(
        "note: --sign {} accepted but ignored — detached signature side-car generation is \
         not implemented in the CLI yet.",
        key_path.display()
    );
}
```

#### Suggested Resolution

Either:
1. Hook up the existing `cloacina::security::package_signer` (the comment says "Signature infrastructure exists at cloacina::security::package_signer") to produce a `<archive>.sig` sidecar and have `upload`/`publish` push it to the `package_signatures` table. The infrastructure is there; the wiring isn't.
2. Until then, change `eprintln!` to a hard `Err(CliError::UserError("--sign is not yet implemented; remove the flag or use the Rust API directly"))`. Don't accept a flag that doesn't do anything.

---

### API-06: REST error envelope has three different shapes — `{error, code}`, `{error}`, and `{error: <plain string>}`

**Severity**: Major
**Location**: `crates/cloacina-server/src/routes/error.rs:80-87`; `crates/cloacina-server/src/routes/auth.rs:155-167`; `crates/cloacina-server/src/routes/health_graphs.rs:111-116`; `crates/cloacinactl/src/shared/error.rs:76-86`
**Confidence**: High

#### Description

Three error response shapes co-exist:
1. `ApiError::IntoResponse` → `{"error": "<message>", "code": "<slug>"}`. The dominant pattern (used everywhere `ApiError::*()` is called).
2. `auth.rs::validate_token` returns `Err((StatusCode, Json(serde_json::json!({"error": "..."}))))` — *no* `code` field. This bubbles back through `require_auth` middleware as raw `Json` (line 158, 165), bypassing `ApiError`.
3. `health_graphs.rs:111-116` — `Json(serde_json::json!({"error": format!("graph '{}' not found", name)}))` — flat `{"error": "..."}`, no code.

The CLI's `extract_message` (`error.rs:76-86`) tries to handle both:
```rust
body.get("error").and_then(|e| e.get("message"))   // looks for {error: {message: ...}}
    .or_else(|| body.get("message"))               // falls back to {message: ...}
```
But neither branch matches the `{error: "<plain string>", code: "..."}` shape that's actually emitted. The CLI ends up showing `{"error":"<msg>","code":"<slug>"}` literally as a JSON string in the user-facing error message — useful for debugging, ugly for end users.

#### Evidence

- `error.rs:82-85` (server format): `{"error": self.message, "code": self.code}`.
- `auth.rs:158` (different format): `Json(serde_json::json!({"error": "invalid or revoked API key"}))` — note no code.
- `health_graphs.rs:113` (third format): `{"error": format!("graph '{}' not found", name)}`.
- CLI `extract_message` looks for `error.message` (nested object form) — none of the server formats match.

#### Suggested Resolution

1. Make `ApiError::into_response` the *only* way the server emits errors. Replace the raw `Json` returns in `auth.rs` and `health_graphs.rs` with `ApiError::unauthorized(msg)` / `ApiError::not_found("graph_not_found", msg)` (the latter already has the path).
2. Update CLI `extract_message` to read `error` as a string (the actual format) — and treat `code` as a hint surfaced separately on `ServerReject` errors.
3. Document the envelope in `error.rs`'s rustdoc as **the** envelope, with examples.

---

### API-07: `cloacina::package!()` macro doc says "Six methods"; the macro emits nine; the trait has nine and only documents two

**Severity**: Major
**Location**: `crates/cloacina-workflow-plugin/src/lib.rs:91-94, 700-711`
**Confidence**: High

#### Description

This is LEG-05 from the legibility lens, viewed from the API-design angle: the documented contract of the cdylib-author surface (the macro) is wrong. A cdylib author reading `cloacina::package!()` thinks they get six methods; the macro emits nine; the host treats methods 4-8 as `#[optional(since = 2)]`, so plugins built without those bodies will return `NotImplemented` and the host treats that as "no entries of that kind" — fine for backward compat, but the wrong message for a new author. They will reasonably think methods 7 and 8 (`get_triggerless_graph_metadata`, `invoke_triggerless_graph`) don't exist.

Compounding this, the `CloacinaPlugin` trait's `## Methods` rustdoc section (`lib.rs:706-711`) lists only two methods. The `METHOD_*` constants (`lib.rs:682-698`) all redirect to `METHOD_GET_TASK_METADATA` for their docs ("/// See [`METHOD_GET_TASK_METADATA`].") so a hover in IDE produces nothing useful.

#### Evidence

- `crates/cloacina-workflow-plugin/src/lib.rs:91-94` — "Six methods: get_task_metadata, execute_task, get_graph_metadata, execute_graph, get_reactor_metadata, get_trigger_metadata."
- `crates/cloacina-workflow-plugin/src/lib.rs:128-672` — the macro body emits nine impl methods.
- `crates/cloacina-workflow-plugin/src/lib.rs:706-711` — `## Methods` doc lists only `get_task_metadata` and `execute_task`.
- `crates/cloacina-workflow-plugin/src/lib.rs:682-698` — every constant past index 0 is `/// See [`METHOD_GET_TASK_METADATA`].`.

#### Suggested Resolution

Per LEG-05: rewrite the `package!()` macro doc to enumerate all nine methods with one-line semantics each; rewrite the `## Methods` section to do the same; give every `METHOD_*` constant its own one-line doc that names the method. This is the public surface for cdylib authors — it should be the most-documented thing in the workspace, not the least.

---

### API-08: Graph health routes are mounted at the router root with `merge`, breaking the `/v1/` prefix invariant the rest of the API enforces

**Severity**: Major
**Location**: `crates/cloacina-server/src/lib.rs:449-465, 481-483`
**Confidence**: High

#### Description

The router does `.nest("/v1", auth_routes)` for the bulk of authenticated routes (good — every route under `/v1/`), then `.merge(graph_health_routes)` and `.merge(ws_routes)` (bad — these routes already include `/v1/health/...` and `/v1/ws/...` literally in their `route(...)` paths, but they're not nested). The result is correct paths today, but the convention is broken: a future refactor that relies on `nest("/v1")` to namespace everything will silently break the graph-health and WS routes because they're not in the nest.

There's a regression test for "unprefixed auth route returns 404" (`lib.rs:927-951`) but no equivalent test for the graph-health or WS paths' prefix invariant. If someone refactors the auth_routes path to be relative (drops the `/v1/` literal in the route definitions because nest provides it), the merged routes won't move with it.

The asymmetry is also a documentation hazard: the system overview's section "Computation graph health" notes "**note nesting** — these merge into the router root, not under `/v1`" — that should not be a comment on a published API.

#### Evidence

```rust
// crates/cloacina-server/src/lib.rs:449-461
let graph_health_routes = Router::new()
    .route("/v1/health/accumulators", get(...))
    .route("/v1/health/graphs", get(...))
    // ...
    .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));
```

```rust
// crates/cloacina-server/src/lib.rs:481-483
.nest("/v1", auth_routes)
.merge(graph_health_routes)
.merge(ws_routes)
```

vs. inside `auth_routes` the routes are defined relatively (`/auth/keys`, `/tenants`, etc.) and the `/v1` comes from the nest.

#### Suggested Resolution

Restructure: move the `/v1/health/*` and `/v1/ws/*` routes into the `auth_routes` (or a sibling `Router` that's `.nest("/v1", ...)`'d the same way). The routes lose the literal `/v1/` prefix in their `route(...)` paths but gain consistency. WS routes need their auth handling preserved (auth-in-handler, not auth-in-middleware).

---

### API-09: `Authorization: Bearer <key>` in WS query `token=` — overlapping but different validation paths with no error if you mix them

**Severity**: Minor
**Location**: `crates/cloacina-server/src/routes/ws.rs:62-94`
**Confidence**: High

#### Description

WebSocket auth accepts a token via two channels:
1. `Authorization: Bearer <api-key>` header — validated as a long-lived API key against the DAL.
2. `?token=<ticket>` query parameter — validated as a single-use, short-TTL ticket against `WsTicketStore`.

`extract_ws_token` returns the **first** channel found in this order (header preferred). If a client sends both a Bearer header AND a `?token=` query param, the query param is silently ignored. There's no error, no warning, no way to surface the redundancy. A client that intends to use the single-use ticket but happens to also send a (leftover) Bearer header will burn the API key validation path instead of the ticket — and the ticket sits in the store until it expires (so it's effectively wasted).

The query-param-as-ticket-not-API-key model is documented inline (`ws.rs:43-48`) but the asymmetry isn't: the ticket is single-use; the API key is not. Switching channels mid-conversation is not possible (auth is one-shot at upgrade), but a client author has to read the source to know that.

#### Evidence

```rust
// crates/cloacina-server/src/routes/ws.rs:62-77
fn extract_ws_token(headers: &HeaderMap, query: &WsAuthQuery) -> Option<WsTokenSource> {
    // Prefer header — accepts Bearer API keys
    if let Some(val) = headers.get(AUTHORIZATION) {
        if let Ok(s) = val.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ") {
                return Some(WsTokenSource::Header(token.to_string()));
            }
        }
    }
    // Query param — treated as a single-use ticket, NOT a raw API key
    query.token.as_ref().map(|t| WsTokenSource::QueryTicket(t.clone()))
}
```

The `WsAuthQuery` struct has only `token: Option<String>` — there's no way to pass a real API key in the query. So users following the docstring "WebSocket clients can't set custom headers on the upgrade request in browsers, so we accept a **ticket**" find that the documented path requires three steps (POST /auth/ws-ticket → get ticket → connect WS), and there's no error path if they try to put the API key in the query directly (it'll just fail the ticket validation with `invalid or expired WebSocket ticket` — confusing because the user knows their key is valid).

#### Suggested Resolution

1. If both channels are present: error with a clear `multiple_auth_methods` code rather than silently preferring header.
2. If `?token=<api-key>` (i.e., the value looks like an API key, e.g., starts with `clk_`): return a 400 with a code `api_key_in_query_param` and a hint to use `POST /auth/ws-ticket` first. Document this in the WS protocol section of the README.
3. The "auth is one-shot at upgrade; no per-message reauth" rule should be in a header-comment on `accumulator_ws` / `reactor_ws` (currently it's only in the system overview).

---

### API-10: Hardcoded `dal.schedule().list(None, false, 100, 0)` in `list_triggers` ignores any client `--limit` / `--offset` and silently caps at 100

**Severity**: Major
**Location**: `crates/cloacina-server/src/routes/triggers.rs:42, 87`
**Confidence**: High

#### Description

The `list_triggers` and `get_trigger` handlers both call `dal.schedule().list(None, false, 100, 0)` — explicitly hardcoded `100`. There's no `Query` extractor, no `?limit=` parsing, no pagination cursor. A tenant with more than 100 schedules gets the first 100 and no signal that there are more. The CLI doesn't surface a `--limit` flag for the trigger noun (`trigger/mod.rs:34`), so the limit is invisible from both ends.

`get_trigger`'s pattern is worse: it does the same `list(None, false, 100, 0)` and then does `find()` over the results — so a user trying to inspect the 101st trigger by name silently gets a 404 (`trigger_not_found`) regardless of whether the trigger actually exists.

The DAL itself takes pagination args correctly; the route handlers just don't surface them.

#### Evidence

```rust
// crates/cloacina-server/src/routes/triggers.rs:42
match dal.schedule().list(None, false, 100, 0).await {
```

```rust
// crates/cloacina-server/src/routes/triggers.rs:87
let schedules = match dal.schedule().list(None, false, 100, 0).await {
```

#### Suggested Resolution

1. `list_triggers`: accept `Query<{limit: Option<u32>, offset: Option<u32>}>` (default 50/0, max 1000). Include `next_offset` or `total` in the response so clients can paginate.
2. `get_trigger`: query the schedule directly by name via a new DAL method `schedule().find_by_name(workflow_or_trigger_name)`; don't list-and-scan.
3. Same pattern review for `list_executions` (PERF-11 already flagged the no-LIMIT issue from the perf angle; it's an API contract issue too).

**Cross-cutting note**: PERF-11 ("`get_active_executions` and `get_ready_for_retry` return un-paginated full table scans") is the same family — DAL methods take pagination, route handlers don't expose it.

---

### API-11: The Rust prelude advertises a `workflow!` macro that doesn't exist; the engine `lib.rs` quick-start uses it; users copy-pasting fail to compile

**Severity**: Major
**Location**: `crates/cloacina/src/lib.rs:75-105, 452, 484-486`
**Confidence**: High

#### Description

Same as LEG-04, viewed from API-design: the Rust public API's most-read piece of documentation — the engine crate's top-level rustdoc plus the prelude module — promotes a declarative `workflow! { name: …, tasks: [...] }` macro syntax. The macros actually exported (when `feature = "macros"` is on) are `task` and `workflow` — but `workflow` is the `#[workflow]` attribute proc-macro applied to a module, not a declarative `workflow!` macro. So `let w = workflow! { name: "foo", tasks: [a, b] };` fails with "macro `workflow` not found in this scope".

The prelude doc at `lib.rs:452` says: "Macros: `#[task]` and `workflow!` (when "macros" feature is enabled)" — the syntax `workflow!` (with bang) and `#[task]` (with attribute marker) is the user's only signal which kind of macro each is. They're inconsistent in the doc and inconsistent with reality. The prelude actually exports `cloacina_macros::{task, workflow}` (line 485) — both attribute proc-macros.

A new user's first 30 minutes of cloacina is reading these docs and writing code that doesn't compile.

#### Evidence

- `crates/cloacina/src/lib.rs:75-105` — quick-start uses `let workflow = workflow! { name: "etl_pipeline", description: "...", tasks: [extract_data, transform_data, load_data] };`
- `crates/cloacina/src/lib.rs:452` — prelude doc: "Macros: `#[task]` and `workflow!`".
- `crates/cloacina-macros/src/lib.rs` — exports `#[workflow]` attribute, not a `workflow!` declarative macro.
- README has the same problem (`README.md:72-77`).

#### Suggested Resolution

Per LEG-04: rewrite the quick-start to match `#[workflow(name = "...", description = "...")] pub mod my_workflow { #[task(id = "extract", dependencies = [])] async fn extract(ctx: ...) -> ... { ... } #[task(id = "load", dependencies = ["extract"])] async fn load(ctx: ...) -> ... { ... } }`. Update the prelude doc to say `#[task]` and `#[workflow]` consistently. Bump the version pin in README from `0.1.0` to `0.5.1`.

---

### API-12: Python `RuntimeMessage` enum is the bottleneck for the Python public API — every Python-facing engine method is a 4-touch ripple

**Severity**: Major (Evolvability cross-ref EVO-05; API-design framing distinct)
**Location**: `crates/cloacina-python/src/bindings/runner.rs:48-200`
**Confidence**: High

#### Description

The Python `cloaca` module exposes 28 RPC methods (one per `RuntimeMessage` variant) — `Execute`, `RegisterCronWorkflow`, `ListCronSchedules`, `SetCronScheduleEnabled`, `DeleteCronSchedule`, `GetCronSchedule`, `UpdateCronSchedule`, `GetCronExecutionHistory`, `GetCronExecutionStats`, `ListTriggerSchedules`, `GetTriggerSchedule`, `SetTriggerEnabled`, `GetTriggerExecutionHistory`, etc. From the Python *consumer's* perspective, that means every method on `PyDefaultRunner` has a hand-coded shape with hand-coded argument types (no auto-shape derivation from the Rust signature) and no shared documentation pattern — each method's docstring is open-coded inline.

The asymmetries:
- Some methods take `enabled_only`/`limit`/`offset` (cron/trigger list); others take just an id (`GetCronSchedule`); others take a workflow_name (`Execute`). No pagination convention.
- Cron and trigger schedules have parallel but-not-identical APIs: `ListCronSchedules(enabled_only, limit, offset)` vs `ListTriggerSchedules(enabled_only, limit, offset)`, `GetCronExecutionHistory(schedule_id, limit, offset)` vs `GetTriggerExecutionHistory(trigger_name, limit, offset)`. The lookup key changes (uuid vs name) — that's a leak from the DB schema and not necessarily a Python-API choice.
- The Python `WorkflowResult` shape (visible at `runner.execute("name", ctx)`) is documented in `bindings/runner.rs:218+` but isn't surfaced cleanly from the `cloaca` module's docstring. Python users have to read the Rust source to see what `result.status` returns.

The 28-variant enum is the wrong abstraction level: Python users want to call `runner.execute()`, `runner.cron_schedules.list()`, `runner.cron_schedules.get(id)` — a hierarchy that mirrors the noun-verb CLI. Today they get a flat namespace of 28 methods on `PyDefaultRunner`, each one of which is a separate enum variant + match arm + Python wrapper.

#### Evidence

- `crates/cloacina-python/src/bindings/runner.rs:48-200+` — enum `RuntimeMessage` with 28 variants.
- The variants are uneven in shape: `Execute { workflow_name, context, response_tx }`, `ListCronSchedules { enabled_only, limit, offset, response_tx }`, `Shutdown` (no payload), etc.

#### Suggested Resolution

1. Group related methods into Python sub-objects: `runner.cron`, `runner.triggers`, `runner.executions`. Each sub-object has 5-6 methods. The flat 28-method `PyDefaultRunner` becomes a small dispatch surface plus three sub-classes.
2. Document the response shapes — `WorkflowResult`, `Schedule`, `ScheduleExecution` should all have rustdoc that's mirrored into the Python `cloaca.WorkflowResult.__doc__` etc.
3. Per EVO-05: shrink the enum by passing futures through the channel rather than named operations; the Python wrapper builds closures.

---

### API-13: `DefaultRunnerConfig` exposes 30+ knobs as builder methods but no precedence/units doc — `cron_max_recovery_age: Duration` vs `cron_lost_threshold_minutes: i32` vs `cron_max_recovery_attempts: usize`

**Severity**: Minor
**Location**: `crates/cloacina/src/runner/default_runner/config.rs:67-499`
**Confidence**: High

#### Description

`DefaultRunnerConfig` has 30 fields, each with a getter and a builder setter (`max_concurrent_tasks`, `scheduler_poll_interval`, `task_timeout`, `workflow_timeout`, `db_pool_size`, ...). The setters are uniformly typed: `Duration`, `usize`, `bool`, `Option<...>`. The minor concern is **unit drift**: most time-related knobs use `Duration` (`scheduler_poll_interval`, `task_timeout`, `cron_recovery_interval`, etc.), but **two** use raw integers in named units:
- `cron_lost_threshold_minutes: i32` — minutes
- `cron_max_recovery_attempts: usize` — count

The first is a unit drift (other thresholds are `Duration`); the second is fine but called out for completeness. A user setting `cron_lost_threshold_minutes(60)` thinks they got 60 minutes — they did — but reading the surrounding code that's `Duration::from_secs(...)` repeatedly, the unit named in the field is jarring.

The config is also `#[non_exhaustive]` (line 67) — good, evolvability win — but the doc doesn't say users **must** use the builder, not struct literal. (Today the setters are the only public-mutable surface, but the doc doesn't connect those dots.)

The validation in `build()` (lines 471-498) is a small handful of bounds checks (`> 0`, `<= 1000`). There's no cross-knob validation: e.g., `cron_poll_interval` should be ≤ `cron_max_recovery_age`, but nothing enforces that.

#### Evidence

- `config.rs:80-82` — `cron_lost_threshold_minutes: i32`, `cron_max_recovery_age: Duration`, `cron_max_recovery_attempts: usize` — three different shapes for related knobs.
- `config.rs:471-498` — `build()` has 5 validation rules; nothing about `enable_recovery && !enable_claiming` (which would be incoherent), nothing about pool size vs concurrency.

#### Suggested Resolution

1. Convert `cron_lost_threshold_minutes` to `cron_lost_threshold: Duration` for unit consistency; keep an `_minutes` deprecation alias if needed for backcompat.
2. Add cross-knob validation: e.g., `db_pool_size >= max_concurrent_tasks + 4` (executor + heartbeat tasks); error with explicit messages naming both knobs.
3. Add a config-validation doc section pointing to all the rules in one place.

---

### API-14: `cloacina::package!()` macro emits all-or-nothing — there's no way to author a "tasks-only" or "trigger-only" package without invoking the full nine-method surface

**Severity**: Minor
**Location**: `crates/cloacina-workflow-plugin/src/lib.rs:110-672`
**Confidence**: Medium

#### Description

The shell macro `cloacina::package!()` is a single, monolithic emission that always implements all nine `CloacinaPlugin` methods. A package that only declares tasks gets the full body anyway — the unused method bodies walk empty inventories and return empty vectors. That's correct (the inventory walk is the right shape for symmetry) but the macro doesn't compose: there's no `cloacina::package!(only = [tasks])` to opt out of the trigger / reactor / CG paths. For very simple packages, the cdylib carries 4 separate `OnceLock<tokio::runtime::Runtime>` initializations (one per shape) even when only one shape is used.

A composable version would let authors say `cloacina::package!(tasks);` or `cloacina::package!(tasks, triggers);` and emit only those methods. The unused method indices would still be defined as required-by-trait but would return `CallError::NotImplemented`. Authors writing "I just want a workflow with tasks" — by far the most common case — pay the full FFI surface.

This is also the surface called out by EVO-01: every plugin-ABI evolution is a single point of churn for every cdylib in the world. Decomposing the macro into per-method emitters helps both ABI evolution and authoring ergonomics.

#### Evidence

- The 560-line monolithic `package!()` body in `lib.rs:110-672`.
- `lib.rs:217-225, 333-340, 480-487, 577-583` — four separate `OnceLock<Runtime>` initializations, all with the same body (LEG-14).

#### Suggested Resolution

1. Decompose into per-method helpers (per EVO-01's resolution suggestion) so the shell becomes the orchestrator.
2. Add `cloacina::package!(includes = [tasks, triggers, ...])` for explicit shape selection. Default = all (current behavior).
3. Share a single `OnceLock<Runtime>` across all four shapes.

---

### API-15: `ReactorCommand` uses serde-tagged JSON (`{"type":"force_fire"}`-shaped) but the variants don't all have a documented schema

**Severity**: Minor
**Location**: `crates/cloacina/src/computation_graph/reactor.rs:163-181`; `crates/cloacina-server/src/routes/ws.rs:273-410`
**Confidence**: High

#### Description

The reactor WS protocol sends/receives JSON-encoded `ReactorCommand` and `ReactorResponse` enum values. The set is `ForceFire | FireWith { cache: HashMap<String, Vec<u8>> } | GetState | Pause | Resume`. The wire format is serde-tagged (default `{"ForceFire"}` or `{"FireWith":{"cache":...}}` shape — depends on whether the enum has a `#[serde(tag = "type")]`). I couldn't find an explicit serde tag attribute, so the format is the default externally-tagged shape. Either way:

- **The wire format is undocumented in user-facing code** — there's no example WS exchange in any doc, no JSON schema in `docs/`, no fixture file showing `{"type": "force_fire"}` vs `{"ForceFire": null}` etc.
- The `cache` field of `FireWith` carries `HashMap<String, Vec<u8>>` — `Vec<u8>` serializes as a JSON array of integers, not as base64. Operators sending boundary state via `ReactorCommand::FireWith` over WS have to base64-encode/decode out-of-band. This is a sharp edge with no doc.
- `ManualCommand` (the internal channel type) is a strict subset of `ReactorCommand` (LEG-12). The WS handler converts via match (`ws.rs:355-381`) — if `ReactorCommand` ever gains a variant that should also flow through the manual command channel, the WS handler is the only thing that knows.

#### Evidence

- `crates/cloacina/src/computation_graph/reactor.rs:172-181` — `ReactorCommand` enum.
- `crates/cloacina-server/src/routes/ws.rs:273-289` — `serde_json::from_str::<ReactorCommand>(&text)` with no version negotiation, no schema validation beyond serde's defaults.
- No fixtures, no example exchanges in `docs/`.

#### Suggested Resolution

1. Add `#[serde(tag = "type", rename_all = "snake_case")]` (or document the existing externally-tagged shape) and ship a `ws_protocol.md` doc with example exchanges.
2. Change `cache: HashMap<String, Vec<u8>>` to `cache: HashMap<String, String>` where the value is base64; or use `serde_bytes` for the field; or make this explicit in a custom serializer. The wire format gets readable.
3. Bound `ReactorCommand` to a versioned envelope: `{"version": 1, "command": {"type": "force_fire"}}` so additive changes are forward-compatible.

---

### API-16: `WorkflowExecution::execute` returns `WorkflowExecutionResult` with `final_context: Context<serde_json::Value>` — JSON is the only context type users can interact with

**Severity**: Minor
**Location**: `crates/cloacina/src/executor/workflow_executor.rs:55-101`; `crates/cloacina/src/runner/default_runner/workflow_executor_impl.rs:55-100`
**Confidence**: High

#### Description

`Context<T>` is generic in the type system (`crates/cloacina-workflow/src/context.rs`) but the public `WorkflowExecutor::execute` method is fixed to `Context<serde_json::Value>`. Users with strongly-typed domain values cannot pass a `Context<MyDomainStruct>` and get one back; they have to JSON-roundtrip. The constraint is correct (the DB persists JSON; cross-task context flows JSON; FFI layer flows JSON), but the public API doesn't acknowledge it: `Context<T>` is `pub` and parametric, but the only callable shape is `T = serde_json::Value`. Users discover this when they try `runner.execute::<MyType>("name", ctx).await` and get a type error.

Concretely, a workflow author writing
```rust
let ctx: Context<MyType> = Context::new();
runner.execute("foo", ctx).await
```
has to first convert: `let ctx: Context<serde_json::Value> = Context::new(); ctx.insert("my_key", serde_json::to_value(my_thing)?)?;` — boilerplate every time.

#### Evidence

- `crates/cloacina/src/executor/workflow_executor.rs:55-58` — `async fn execute(&self, workflow_name: &str, context: Context<serde_json::Value>) -> ...`.
- `crates/cloacina-workflow/src/context.rs:3-100` — `pub struct Context<T>` with full generic parametricity.

#### Suggested Resolution

Either:
1. Make the public API explicit: rename the field `Context<serde_json::Value>` → a type alias `WorkflowContext = Context<serde_json::Value>` and use it everywhere the wire format is JSON. The generic `Context<T>` lives for in-memory composition only.
2. Add a `WorkflowExecutor::execute_typed<T: Serialize + DeserializeOwned>(...)` convenience that does the JSON roundtrip internally.

---

### API-17: `cloacinactl execution events --follow` is documented but errors immediately with "tracked under spec Open Items"

**Severity**: Minor
**Location**: `crates/cloacinactl/src/nouns/execution/mod.rs:91-94`
**Confidence**: High

#### Description

The CLI defines `execution events --follow` as "Follow live events (SSE) until Ctrl-C." The handler returns `Err(CliError::UserError("--follow streaming is tracked under spec Open Items; not in v1"))` immediately. This is the same shape as API-05 (`pack --sign`) — the contract is on the CLI but not implemented. At least this one errors instead of silently doing nothing. Still: the help text shouldn't advertise a flag that always errors.

#### Evidence

```rust
// execution/mod.rs:91-94
if follow {
    return Err(CliError::UserError(
        "--follow streaming is tracked under spec Open Items; not in v1".into(),
    ));
}
```

The flag is in `clap::Subcommand`, so it shows up in `cloacinactl execution events --help`.

#### Suggested Resolution

Either:
1. Implement: the server has `execution_events` table; SSE streaming from Postgres `LISTEN/NOTIFY` is a small lift if the rest of the SSE infra is in place.
2. Hide the flag until implemented: `#[arg(hide = true)]` so `--help` doesn't list it; the user error message stays for someone who explicitly types `--follow`.

---

### API-18: `Trigger` trait's three lifecycle hooks (`poll`, `cron_expression`, `allow_concurrent`) are split between the trait and a wire-format struct — authors don't have a single contract

**Severity**: Minor
**Location**: `crates/cloacina-workflow/src/trigger.rs:91+`; `crates/cloacina-workflow-plugin/src/types.rs:248-264`
**Confidence**: Medium

#### Description

A trigger author writing `impl Trigger for MyTrigger` provides `poll(&self) -> TriggerResult`, `cron_expression(&self) -> Option<&str>`, `poll_interval(&self) -> Duration`, and `allow_concurrent(&self) -> bool`. The same fields show up in `TriggerPackageMetadata` (`types.rs:249-264`):
```rust
pub struct TriggerPackageMetadata {
    pub name: String,
    pub package_name: String,
    pub poll_interval: String,        // humantime ("5s", "1m") — different type!
    pub cron_expression: Option<String>,
    pub allow_concurrent: bool,
}
```

Note: `poll_interval` is `Duration` in the trait but a humantime `String` in the wire format. The macro emits a humantime serializer in `cloacina::package!()`'s `get_trigger_metadata` body. Authors don't see this — they impl `Duration` and the macro converts.

The split itself isn't wrong (wire format is wire format, trait is trait), but the **doc** on the `Trigger` trait doesn't mention that `poll_interval` is wire-encoded as humantime, doesn't reference `TriggerPackageMetadata`, and doesn't tell authors that their `poll_interval(&self) -> Duration::from_millis(500)` will be rounded to humantime resolution at the FFI boundary. A user who returns `Duration::from_micros(100)` from `poll_interval` will silently get something that humantime parses as `0s` after roundtrip — and the trigger will busy-loop.

#### Evidence

- `crates/cloacina-workflow/src/trigger.rs` — `Trigger` trait.
- `crates/cloacina-workflow-plugin/src/types.rs:255-256` — `pub poll_interval: String` with humantime parse.

#### Suggested Resolution

1. Document the wire-format coercion in `Trigger` trait doc: "Returned `Duration` values smaller than 1 millisecond are rounded up to 1 ms when traversing the FFI boundary."
2. Validate at the macro layer: if `poll_interval()` returns a `Duration` smaller than 1ms, fail compile/run with a clear error.
3. Consider unifying: change the wire format to `poll_interval_ms: u64` and parse on the receiving side. humantime is operator-friendly when written by hand in `package.toml`, but here it's the bridge between in-process Rust types — `u64` ms is more honest.

---

### API-19: `cloacina-server` `--bootstrap-key` and `CLOACINA_BOOTSTRAP_KEY` env var let an operator pin a known plaintext key — but the docs don't warn that this defeats the "plaintext returned exactly once" promise

**Severity**: Minor
**Location**: `crates/cloacina-server/src/lib.rs:115-117`
**Confidence**: Medium

#### Description

`cloacina-server` accepts a `bootstrap_key: Option<String>` argument and a `CLOACINA_BOOTSTRAP_KEY` env var. When set, it short-circuits the auto-generated bootstrap key path — the operator pins a specific plaintext that becomes the initial god-mode admin key. The documented contract elsewhere (`keys.rs:65-67`: "Returns the plaintext key exactly once. It cannot be retrieved again.") is violated by this path: the plaintext is now in the operator's shell history, env file, or systemd unit. There's no docstring on `bootstrap_key` warning about this.

The `bootstrap_admin_key` function persists the plaintext to `~/.cloacina/bootstrap-key` mode 0600 (per system overview §3) regardless of source. So even with `--bootstrap-key`, the file gets written. This is sensible (so the operator can recover the plaintext after restart), but again: undocumented.

#### Evidence

- `crates/cloacina-server/src/lib.rs:117` — `bootstrap_key: Option<String>` parameter, no doc comment explaining when it's used vs auto-generated.
- `crates/cloacina-server/src/lib.rs:294-296` — only documented endpoint mentions are `POST/GET/DEL /v1/auth/keys`. Nothing about bootstrap-key handling.

#### Suggested Resolution

1. Doc on `bootstrap_key` (and on the CLI flag): "If set, this plaintext key becomes the initial admin god-mode key. It must be 32+ bytes and is stored in `~/.cloacina/bootstrap-key` mode 0600 on first run. Subsequent runs ignore this flag if the file already exists. Treat this value as equivalent to a bare-secret credential — do not commit it to a docker-compose file or systemd unit without secret-management."
2. Add a startup log line that logs whether the bootstrap key was read from `--bootstrap-key`/env vs auto-generated, so operators can see in their logs which path ran.

**Cross-cutting note**: Security review territory.

---

### API-20: `WsTicketStore::issue` evicts oldest tickets when at capacity — but `consume` of a missing ticket gives the same error message as an expired one

**Severity**: Minor
**Location**: `crates/cloacina-server/src/routes/auth.rs:294-331`
**Confidence**: High

#### Description

`WsTicketStore` has TTL-based eviction (good) and capacity-based oldest-eviction (good — bounded memory). But `consume(ticket)` returns `Option<AuthenticatedKey>` — there's no distinction between:
- "ticket never existed" (typo, replay attempt, etc.)
- "ticket existed but expired" (slow client)
- "ticket existed but was already consumed" (replay)
- "ticket was evicted under capacity pressure" (DoS)

All four return `None` and the WS handler maps to a single `ApiError::unauthorized("invalid or expired WebSocket ticket")` (`ws.rs:92`). For an operator debugging a "WS auth failed" issue, all four paths look identical.

#### Evidence

```rust
// crates/cloacina-server/src/routes/auth.rs:323-331
pub async fn consume(&self, ticket: &str) -> Option<AuthenticatedKey> {
    let mut store = self.tickets.lock().await;
    if let Some(entry) = store.remove(ticket) {
        if entry.expires_at > Instant::now() {
            return Some(entry.auth);
        }
    }
    None
}
```

`ws.rs:88-92`:
```rust
WsTokenSource::QueryTicket(ticket) => state
    .ws_tickets
    .consume(&ticket)
    .await
    .ok_or_else(|| ApiError::unauthorized("invalid or expired WebSocket ticket")),
```

#### Suggested Resolution

`consume` returns `Result<AuthenticatedKey, TicketError>` with variants `NotFound`, `Expired`, `EvictedUnderCapacity`. The WS handler logs the reason at debug level and returns the same UNAUTHORIZED to the client (don't leak which case to a potential attacker), but operators get the diagnosable reason in logs.

---

### API-21: `cloacinactl daemon stop` / `server stop` / `compiler stop` — verb is consistent across nouns but health/status output structure isn't

**Severity**: Observation
**Location**: `crates/cloacinactl/src/nouns/{daemon,server,compiler}/{health,status,start,stop}.rs`
**Confidence**: Medium

#### Description

The three runtime services (`daemon`, `server`, `compiler`) each implement `start`/`stop`/`status`/`health` verbs in their own subdirectory. The verb palette is consistent (good); the implementation shapes likely diverge across the three (each is a separate file). This is by-design noun-verb file structure but worth noting: a user looking at `cloacinactl daemon health` vs `cloacinactl server health` may get different fields — daemon's health checks a unix socket, server's health checks an HTTP endpoint, compiler's health is its own thing. There's no shared schema for what a "health" response looks like across the three, and no doc explaining the differences.

#### Evidence

- `crates/cloacinactl/src/nouns/daemon/health.rs`, `.../server/health.rs`, `.../compiler/health.rs` — three separate files.
- I didn't read all three but the system overview suggests they're not unified.

#### Suggested Resolution

If the three health responses are structurally similar (status, uptime, version, recent error count), define a `ServiceHealth` struct in `cloacinactl` and have all three return it. If they're genuinely different, document each one — the user should be able to see at `cloacinactl daemon health --help` what fields they'll get.

---

## Positive Patterns

1. **`cloacinactl` noun-verb file structure with consistent verb palette.** Every noun has its own directory; each verb is one file with a single purpose; the verbs (`list`/`inspect`/`create`/`delete`/`upload`/`pack`) are reused across nouns. A user navigating the CLI source learns the entire palette from one example. The exit-code typology (`CliError::UserError|Network|NotFound|Auth|ServerReject`, mapped to exit codes 1/2/3/4/5 per ADR-0003) is grep-able and stable.

2. **`ApiError` with `{error, code}` envelope and dedicated constructors.** `ApiError::bad_request(code, message)` and friends keep error responses uniform across the bulk of the server. The codes are informative (`signature_verification_unconfigured`, `untrusted_signer`, `package_tampered`, `tenant_access_denied`) — they tell the consumer what went wrong and what they can do. Request-ID propagation via `x-request-id` header is the right pattern (no body pollution).

3. **`#[optional(since = N)]` plugin-ABI versioning.** Methods 4-8 on `CloacinaPlugin` are gated; the host treats `CallError::NotImplemented` uniformly as "no entries of that kind"; a v1 plugin against a v2 host works. This is the right primitive for additive ABI evolution and it's correctly applied.

4. **`#[serde(deny_unknown_fields)]` on `CloacinaMetadata` with friendly migration hints.** A package author with a stale `package_type` or `[[triggers]]` field gets a hard error explaining what to do (`reconciler/loading.rs:175-205`). Hard-fail at the boundary, soft-fail in the message — the right shape for breaking changes.

5. **Universal-types pattern.** `UniversalUuid`, `UniversalTimestamp`, `UniversalBool`, `UniversalBinary` give DAL methods one shape across both backends. The public surface is a single `dal.workflow_execution().get_by_id(UniversalUuid(uuid::Uuid))` regardless of whether the storage is Postgres `bytea`/`uuid` or SQLite `TEXT`/`BLOB`. Naming communicates intent.

6. **DefaultRunnerBuilder with `#[non_exhaustive]` config.** Forward-compatible: adding a new knob doesn't break callers who construct config via the builder. `build()` does basic validation; `with_config()` and `with_schema()` are convenient shortcuts. The `#[must_use = "DefaultRunner runs background tasks; call shutdown() before dropping"]` on the runner type itself is the right reminder for an async-resource type that can't shut down in `Drop`.

7. **Per-tenant route namespace.** `/v1/tenants/{tenant_id}/...` is unambiguous, audit-friendly, and self-documenting. A path-based tenant scoping beats query-param or header-based because logs/metrics naturally include the tenant. The CLI's `--tenant` flag plus `tenant_segment()` fallback to `"public"` keeps single-tenant deployments simple.
