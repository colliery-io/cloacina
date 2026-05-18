{{/*
CLOACI-T-0610 — subchart name helpers.

`postgresql.fullname` is the deterministic `<release-name>-postgresql`
shape the parent chart's DATABASE_URL template depends on. Hard-coded
to that suffix on purpose — changing it would break the parent's URL
without a separate migration story.
*/}}
{{- define "postgresql.fullname" -}}
{{- printf "%s-postgresql" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Standard labels stamped on every rendered object.
*/}}
{{- define "postgresql.labels" -}}
app.kubernetes.io/name: postgresql
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/component: database
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/part-of: cloacina-server
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end -}}

{{/*
Selector labels — subset of `labels` that must NOT change between
revisions (Deployment selector + Service selector both use these).
*/}}
{{- define "postgresql.selectorLabels" -}}
app.kubernetes.io/name: postgresql
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/component: database
{{- end -}}
