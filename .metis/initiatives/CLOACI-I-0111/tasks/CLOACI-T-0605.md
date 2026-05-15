---
id: t-03-helm-chart-for-cloacina-server
level: task
title: "T-03: Helm chart for cloacina-server"
short_code: "CLOACI-T-0605"
created_at: 2026-05-14T22:45:17.785379+00:00
updated_at: 2026-05-15T00:26:45.408584+00:00
parent: CLOACI-I-0111
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0111
---

# T-03: Helm chart for cloacina-server

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0111]]

## Objective **[REQUIRED]**

Helm chart for `cloacina-server` published as an OCI artifact to `ghcr.io/colliery-software/charts/cloacina-server`.

### Deliverables

1. **`charts/cloacina-server/`** layout:
   - `Chart.yaml` — `appVersion` tracks the server crate; `version` is the chart's own semver
   - `values.yaml` — `image.repository|tag|pullPolicy`, `replicaCount`, `resources`, `database.url` or `postgresql.enabled` (Bitnami subchart), `apiKeySecretRef`, `ingress.{enabled,host,tls}`, `serviceMonitor.enabled`, `podSecurityContext`
   - `templates/` — `deployment.yaml`, `service.yaml`, `ingress.yaml`, `configmap.yaml`, `secret.yaml`, `servicemonitor.yaml`, `_helpers.tpl`
   - Bitnami `postgresql` subchart vendored as an optional dependency

2. **CI**: `helm lint` + `helm template` (both Postgres modes) in PR workflow; `helm push` as OCI on release tag.

3. **Docs**: `docs/content/platform/how-to-guides/deploying-to-kubernetes.md`:
   - `helm install cloacina oci://ghcr.io/colliery-software/charts/cloacina-server --version <tag>`
   - Bring-your-own-Postgres + bundled-Postgres value examples
   - Ingress / TLS notes
   - Upgrade procedure

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `helm lint` and `helm template` pass for both `postgresql.enabled=true|false`
- [ ] `helm install` to a Kind cluster brings up server + Postgres; `/v1/health` returns 200 inside the cluster
- [ ] Release tag pushes the chart as an OCI artifact
- [ ] `helm install ... --version <tag>` from the registry works on a fresh cluster
- [ ] Doc renders + survives `angreal docs build`

### Dependencies

Blocked by **CLOACI-T-0604** — chart needs an image to pull.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

*To be added during implementation*
