#!/bin/sh
#
# Runtime server-URL injection (CLOACI-I-0117 / OQ-5, T-0659).
#
# Renders index.html from the committed template, substituting the target
# server URL from $CLOACINA_SERVER_URL into window.__CLOACINA_CONFIG__.
# Rendering from the template each start keeps this idempotent across
# container restarts. Empty/unset URL → the /connect form asks the user.
#
# nginx:alpine's base entrypoint runs every script in
# /docker-entrypoint.d/ before launching nginx, so this executes once at
# container start.
set -eu

HTML_DIR="/usr/share/nginx/html"
TMPL="${HTML_DIR}/index.html.tmpl"
OUT="${HTML_DIR}/index.html"

SERVER_URL="${CLOACINA_SERVER_URL:-}"

# Escape characters that would break the sed replacement / JS string
# literal (the URL itself shouldn't contain quotes, but be defensive).
ESCAPED=$(printf '%s' "$SERVER_URL" | sed -e 's/[\\&|"]/\\&/g')

sed "s|defaultServerUrl: \"\"|defaultServerUrl: \"${ESCAPED}\"|" "$TMPL" > "$OUT"

echo "cloacina-ui: rendered index.html with defaultServerUrl=\"${SERVER_URL}\""
