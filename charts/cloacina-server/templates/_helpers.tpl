{{/*
Expand the name of the chart.
*/}}
{{- define "cloacina-server.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Fully qualified resource name.
*/}}
{{- define "cloacina-server.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}

{{/*
Chart label.
*/}}
{{- define "cloacina-server.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Common labels — applied to every resource.
*/}}
{{- define "cloacina-server.labels" -}}
helm.sh/chart: {{ include "cloacina-server.chart" . }}
{{ include "cloacina-server.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{/*
Selector labels — must match the deployment selector exactly.
*/}}
{{- define "cloacina-server.selectorLabels" -}}
app.kubernetes.io/name: {{ include "cloacina-server.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{/*
Image reference. Defaults image.tag to .Chart.AppVersion when unset.
*/}}
{{- define "cloacina-server.image" -}}
{{- $tag := default .Chart.AppVersion .Values.image.tag -}}
{{- printf "%s:%s" .Values.image.repository $tag -}}
{{- end -}}

{{/*
Validate the database wiring. Fails the install with a clear message if
neither path is configured.
*/}}
{{- define "cloacina-server.validateDatabase" -}}
{{- $hasUrl := and .Values.database.url (ne .Values.database.url "") -}}
{{- $hasSecret := and .Values.databaseUrlSecretRef.name (ne .Values.databaseUrlSecretRef.name "") -}}
{{- $hasBundled := .Values.postgresql.enabled -}}
{{- if not (or $hasUrl $hasSecret $hasBundled) -}}
{{- fail "cloacina-server: configure exactly one of `database.url`, `databaseUrlSecretRef.name`, or `postgresql.enabled=true`" -}}
{{- end -}}
{{- end -}}
