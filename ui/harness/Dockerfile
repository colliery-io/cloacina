# syntax=docker/dockerfile:1.7
#
# cloacina seed/demo harness image (CLOACI-I-0117 / T-0660).
#
# Builds @cloacina/client (the SDK the driver shares with the UI), then runs
# the workload driver. The driver only needs the SDK at runtime — no bundler.
#
# Packages are NOT baked in: the driver reads compiled `.cloacina` files from
# HARNESS_PACKAGE_DIR (default /packages) at runtime, mounted by the demo
# compose. Compile them once with `angreal ui build-fixtures`.
#
# BUILD CONTEXT MUST BE THE REPO ROOT (the harness needs the sibling SDK):
#   docker build -t cloacina-seed-harness:dev -f ui/harness/Dockerfile .

ARG NODE_VERSION=20

FROM node:${NODE_VERSION}-slim AS builder
WORKDIR /build
COPY clients/typescript/package.json clients/typescript/package-lock.json ./clients/typescript/
RUN cd clients/typescript && npm ci
COPY clients/typescript ./clients/typescript
RUN cd clients/typescript && npm run build
COPY ui/harness/package.json ui/harness/package-lock.json* ./ui/harness/
RUN cd ui/harness && npm install --omit=dev
COPY ui/harness ./ui/harness

FROM node:${NODE_VERSION}-slim AS runtime
# Preserve the builder's directory layout so the harness's
# `file:../../clients/typescript` symlink keeps resolving.
COPY --from=builder /build /build
WORKDIR /build/ui/harness
ENV HARNESS_PACKAGE_DIR=/packages
LABEL org.opencontainers.image.source="https://github.com/colliery-software/cloacina"
LABEL org.opencontainers.image.description="cloacina seed/demo workload harness"
LABEL org.opencontainers.image.licenses="Apache-2.0"
ENTRYPOINT ["node", "src/main.mjs"]
