---
id: helm-chart-replace-bitnamilegacy
level: task
title: "Helm chart: replace bitnamilegacy postgres pin with maintained alternative"
short_code: "CLOACI-T-0610"
created_at: 2026-05-16T00:53:30.076692+00:00
updated_at: 2026-05-16T00:53:30.076692+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Helm chart — replace `bitnamilegacy/postgresql` pin with a maintained alternative

## Type
Tech Debt

## Priority
P2 — Medium. Helm e2e is green today, but `bitnamilegacy/*` is explicitly Bitnami's "we'll keep it around for a while" repo, not a long-term commitment.

## Current state
`charts/cloacina-server/Chart.yaml` declares a soft dependency on Bitnami's `postgresql` Helm chart (`16.x.x` from `oci://registry-1.docker.io/bitnamicharts`). The chart pins the postgres image to `bitnami/postgresql:17.6.0-debian-12-r4`. Bitnami removed all `*-rN` tags from the free `bitnami/postgresql` repo in late 2025; only `bitnamilegacy/postgresql` still hosts them.

In CI we override the image registry on `helm install` (commit `e7f40a7f`):
```
--set postgresql.image.registry=docker.io
--set postgresql.image.repository=bitnamilegacy/postgresql
```

This works but is explicitly temporary — Bitnami's deprecation notice says "backup will be available for some time at the 'Bitnami Legacy' repository". When they pull it, our e2e breaks again.

## Options to evaluate
1. **Switch the subchart** to one actively maintained on free public
   registries: CloudNativePG, zalando-postgres-operator, bitnami's
   "secure images" (paid path), or a thin internal chart wrapping the
   official `postgres:17` Docker image.
2. **Drop the subchart entirely**: keep `postgresql.enabled` false-by-
   default (already the case), and provide a tiny `examples/postgres/`
   chart for users who want an in-cluster Postgres. Production users
   point at managed Postgres anyway.
3. **Pin a specific Bitnami chart version** that still resolves to a
   `bitnami/postgresql` tag that still exists. Likely needs going back
   to a pre-2025 chart version, which carries its own staleness debt.

Option 2 is the cleanest answer for the chart's stated production use
case ("operators wire their own managed Postgres"). Worth a short ADR.

## Acceptance criteria
- [ ] Decide and document the path forward (probably option 2 + an
      ADR amendment to [[CLOACI-A-0005]] or a new helm-chart ADR).
- [ ] Update the chart accordingly.
- [ ] Drop the `bitnamilegacy` override from
      `.github/workflows/ci.yml` (the helm-chart-e2e install step) and
      from `.angreal/task_helm.py` once the new path is in.
- [ ] CI e2e + `angreal helm test` still green.

## Related
- Workaround commit: `e7f40a7f` (ci: bitnami legacy postgres image)
- [[CLOACI-T-0609]] (other tech-debt followup from this iteration)

## Status Updates

*To be added during implementation*
