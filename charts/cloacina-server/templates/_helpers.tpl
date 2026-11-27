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
Selector labels as a `key=value,key=value` CSV — the form the K8s actuator
parses from CLOACINA_SERVER_POD_SELECTOR to build the agent NetworkPolicy egress
rule (CLOACI-T-0819). MUST stay in lockstep with selectorLabels above.
*/}}
{{- define "cloacina-server.selectorLabelsCSV" -}}
app.kubernetes.io/name={{ include "cloacina-server.name" . }},app.kubernetes.io/instance={{ .Release.Name }}
{{- end -}}

{{/*
Image reference. Defaults image.tag to .Chart.AppVersion when unset.
*/}}
{{- define "cloacina-server.image" -}}
{{- $tag := default .Chart.AppVersion .Values.image.tag -}}
{{- printf "%s:%s" .Values.image.repository $tag -}}
{{- end -}}

{{/*
Compiler image reference (CLOACI-I-0105). Defaults the repository to the
server repo with `-compiler` swapped in, and the tag to .Chart.AppVersion.
*/}}
{{- define "cloacina-server.compilerImage" -}}
{{- $repo := .Values.compiler.image.repository | default (.Values.image.repository | replace "cloacina-server" "cloacina-compiler") -}}
{{- $tag := default .Chart.AppVersion .Values.compiler.image.tag -}}
{{- printf "%s:%s" $repo $tag -}}
{{- end -}}

{{/*
Image of the per-tenant compilers (CLOACINA-T-1034). Defaults to the image of
the chart's own compiler.
*/}}
{{- define "cloacina-server.tenantCompilerImage" -}}
{{- $repo := .Values.fleet.compiler.image.repository | default .Values.compiler.image.repository | default (.Values.image.repository | replace "cloacina-server" "cloacina-compiler") -}}
{{- $tag := .Values.fleet.compiler.image.tag | default .Values.compiler.image.tag | default .Chart.AppVersion -}}
{{- printf "%s:%v" $repo $tag -}}
{{- end -}}

{{/*
Fleet-actuator ServiceAccount name. Defaults to "<fullname>-fleet".
*/}}
{{- define "cloacina-server.fleetServiceAccountName" -}}
{{- if .Values.fleet.serviceAccount.name -}}
{{- .Values.fleet.serviceAccount.name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-fleet" (include "cloacina-server.fullname" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}
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

{{/*
Validate where tenant code runs (CLOACINA-T-1075). The server runs no tenant
code in its process unless `server.singleTrust` is true: every task goes to
the agent fleet. An install must say where the agents come from, or choose
single trust.
*/}}
{{- define "cloacina-server.validateFleet" -}}
{{- $actuator := eq .Values.fleet.actuator "kubernetes" -}}
{{- $external := .Values.fleet.externalAgents -}}
{{- $singleTrust := .Values.server.singleTrust -}}
{{- if not (or $actuator $external $singleTrust) -}}
{{- fail "cloacina-server: the server runs tenant tasks only on agents. Set `fleet.actuator=kubernetes` (the server makes the agents), or `fleet.externalAgents=true` (you run agents, for example with the cloacina-agent chart), or `server.singleTrust=true` (every tenant is trusted as much as the server, and tenant code runs in the server process)" -}}
{{- end -}}
{{- end -}}
