# syntax=docker/dockerfile:1.7
#
# cloacina-ui — production SPA image (CLOACI-I-0117 / T-0659)
#
# Multi-stage build:
#   Stage 1 (builder): node — builds @cloacina/client (the SDK the UI
#     depends on via `file:../clients/typescript`), then the Vite SPA.
#   Stage 2 (runtime): nginx — serves the static bundle with SPA fallback.
#
# Runtime server-URL config (OQ-5): the bundle is server-agnostic. The
# entrypoint renders `index.html` from a committed template at container
# start, injecting `CLOACINA_SERVER_URL` into `window.__CLOACINA_CONFIG__`
# so a single image works against any server. Rendering from the template
# each start makes it idempotent across restarts.
#
# BUILD CONTEXT MUST BE THE REPO ROOT (the UI build needs the sibling SDK):
#   docker build -t cloacina-ui:dev -f ui/Dockerfile .
#
# Version-locked to the server release (NFR-004): the UI and SDK both
# carry the server's version in package.json; CI builds them in lockstep.

ARG NODE_VERSION=20
ARG NGINX_VERSION=1.27

# ---------------------------------------------------------------------------
# Stage 1: builder
# ---------------------------------------------------------------------------
FROM node:${NODE_VERSION}-slim AS builder
WORKDIR /build

# Build the SDK first — the UI resolves `@cloacina/client` from its built
# `dist/`, so it must exist before the Vite build runs. The committed
# `generated/types.ts` is shipped in the package, so no codegen (and thus
# no openapi.json) is needed here.
COPY clients/typescript/package.json clients/typescript/package-lock.json ./clients/typescript/
RUN cd clients/typescript && npm ci
COPY clients/typescript ./clients/typescript
RUN cd clients/typescript && npm run build

# UI deps, then source. The `file:../clients/typescript` dep resolves to
# the sibling dir built above.
COPY ui/package.json ui/package-lock.json ./ui/
RUN cd ui && npm ci
COPY ui ./ui
RUN cd ui && npm run build

# ---------------------------------------------------------------------------
# Stage 2: runtime
# ---------------------------------------------------------------------------
FROM nginx:${NGINX_VERSION}-alpine AS runtime

# SPA-fallback server config.
COPY ui/deploy/nginx.conf /etc/nginx/conf.d/default.conf

# Static bundle. Stash index.html as a template the entrypoint renders
# from at start, so runtime config injection is idempotent.
COPY --from=builder /build/ui/dist /usr/share/nginx/html
RUN mv /usr/share/nginx/html/index.html /usr/share/nginx/html/index.html.tmpl

COPY ui/deploy/docker-entrypoint.sh /docker-entrypoint.d/40-cloacina-config.sh
RUN chmod +x /docker-entrypoint.d/40-cloacina-config.sh

EXPOSE 80

LABEL org.opencontainers.image.source="https://github.com/colliery-software/cloacina"
LABEL org.opencontainers.image.description="cloacina-ui — tenant-scoped web control plane (React SPA)"
LABEL org.opencontainers.image.licenses="Apache-2.0"

# nginx:alpine's own entrypoint runs every executable in
# /docker-entrypoint.d/ before launching nginx — our config renderer
# (40-…) lands there, so we keep the base CMD/ENTRYPOINT.
