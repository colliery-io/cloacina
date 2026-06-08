{{/*
Expand the name of the chart.
*/}}
{{- define "cloacina-agent.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Fully qualified resource name.
*/}}
{{- define "cloacina-agent.fullname" -}}
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
{{- define "cloacina-agent.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Common labels — applied to every resource.
*/}}
{{- define "cloacina-agent.labels" -}}
helm.sh/chart: {{ include "cloacina-agent.chart" . }}
{{ include "cloacina-agent.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{/*
Selector labels — must match the deployment selector exactly.
*/}}
{{- define "cloacina-agent.selectorLabels" -}}
app.kubernetes.io/name: {{ include "cloacina-agent.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{/*
Image reference. Defaults image.tag to .Chart.AppVersion when unset.
*/}}
{{- define "cloacina-agent.image" -}}
{{- $tag := default .Chart.AppVersion .Values.image.tag -}}
{{- printf "%s:%s" .Values.image.repository $tag -}}
{{- end -}}

{{/*
Name of the Secret holding the agent API key. When `apiKey` is set inline,
this chart renders a Secret named "<fullname>-api-key"; otherwise the
operator-supplied `apiKeySecretRef.name` is used.
*/}}
{{- define "cloacina-agent.apiKeySecretName" -}}
{{- if .Values.apiKey -}}
{{- printf "%s-api-key" (include "cloacina-agent.fullname" .) -}}
{{- else -}}
{{- .Values.apiKeySecretRef.name -}}
{{- end -}}
{{- end -}}

{{/*
Key within the API-key Secret. Inline `apiKey` always lands under `api-key`;
a BYO secret uses the operator-supplied `apiKeySecretRef.key`.
*/}}
{{- define "cloacina-agent.apiKeySecretKey" -}}
{{- if .Values.apiKey -}}
api-key
{{- else -}}
{{- .Values.apiKeySecretRef.key -}}
{{- end -}}
{{- end -}}

{{/*
Validate the required wiring: a server URL and an API key (inline or via a
secret ref). Fails the install with a clear message if either is missing.
*/}}
{{- define "cloacina-agent.validate" -}}
{{- if not (and .Values.server.url (ne .Values.server.url "")) -}}
{{- fail "cloacina-agent: set `server.url` to the cloacina-server the agent should register with (e.g. http://my-release-cloacina-server:8080)" -}}
{{- end -}}
{{- $hasInline := and .Values.apiKey (ne .Values.apiKey "") -}}
{{- $hasRef := and .Values.apiKeySecretRef.name (ne .Values.apiKeySecretRef.name "") -}}
{{- if not (or $hasInline $hasRef) -}}
{{- fail "cloacina-agent: configure exactly one of `apiKey` (inline; renders a Secret) or `apiKeySecretRef.name` (bring your own Secret)" -}}
{{- end -}}
{{- end -}}
