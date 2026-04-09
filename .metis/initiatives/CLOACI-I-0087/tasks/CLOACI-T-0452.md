---
id: add-tls-support-or-document
level: task
title: "Add TLS support or document reverse proxy requirement (SEC-06)"
short_code: "CLOACI-T-0452"
created_at: 2026-04-08T23:47:17.144907+00:00
updated_at: 2026-04-09T01:05:42.859773+00:00
parent: CLOACI-I-0087
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0087
---

# Add TLS support or document reverse proxy requirement (SEC-06)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0087]]

## Objective

All API traffic including Bearer tokens, tenant credentials, and package uploads is transmitted in cleartext. Combined with SEC-05 (WebSocket tokens in query params), credentials are exposed across the full network path.

**Effort**: 1-2 days (docs approach) or 3-5 days (native TLS)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

**Option A (minimum — documentation):**
- [ ] README and `cloacinactl serve --help` document that a TLS-terminating reverse proxy is required for production
- [ ] Example nginx/Caddy reverse proxy config provided in docs or `docker/`
- [ ] Server logs a WARNING at startup when no TLS is configured: "Running without TLS -- use a reverse proxy for production"

**Option B (preferred — native TLS):**
- [ ] `axum-server` + `rustls` added as optional dependencies behind a `tls` feature flag
- [ ] `--tls-cert` and `--tls-key` CLI flags on `cloacinactl serve`
- [ ] When TLS flags provided, server binds with HTTPS
- [ ] When TLS flags omitted, server binds with plain HTTP + warning log
- [ ] Self-signed cert generation docs for development

## Implementation Notes

### Technical Approach

**Option A**: Add a `## Production Deployment` section to documentation. Add startup warning:
```rust
if !tls_enabled {
    warn!("Server running without TLS -- use a TLS-terminating reverse proxy in production");
}
```

**Option B**: Add `axum-server` and `rustls-pemfile` dependencies. Modify `serve` command:
```rust
if let (Some(cert), Some(key)) = (tls_cert, tls_key) {
    let config = RustlsConfig::from_pem_file(cert, key).await?;
    axum_server::bind_rustls(bind, config).serve(app).await?;
} else {
    axum::serve(listener, app).await?;
}
```

### Dependencies
Do this LAST in I-0087 — lowest urgency since reverse proxy is the common deployment pattern.

## Status Updates

- **2026-04-08**: Went with Option A (docs + warning). Added startup `warn!("Server running without TLS...")` to `serve.rs`. Created `docs/content/how-to-guides/service/production-deployment.md` with Caddy, nginx, and Docker Compose examples, health check config, and binding guidance. Native TLS (Option B) deferred as a future enhancement.
