#!/usr/bin/env bash
#
# install.sh — one-line installer for `cloacinactl`.
#
# Usage:
#   curl -fsSL https://get.cloacina.dev/install.sh | bash
#   curl -fsSL https://get.cloacina.dev/install.sh | bash -s -- --version v0.6.0
#   curl -fsSL https://get.cloacina.dev/install.sh | bash -s -- --prefix /usr/local
#
# Or run directly from a clone:
#   bash scripts/install.sh [--version vX.Y.Z] [--prefix DIR]
#
# Detects OS + arch, downloads the matching release tarball from
# GitHub Releases, verifies its SHA256, and extracts the `cloacinactl`
# binary into ${PREFIX:-$HOME/.cloacina}/bin. Idempotent — re-running
# upgrades the installed binary in place.
#
# CLOACI-I-0111 / T-0603.

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

REPO="${CLOACINA_REPO:-colliery-software/cloacina}"
DEFAULT_PREFIX="${HOME}/.cloacina"
BINARY="cloacinactl"

VERSION=""
PREFIX="${DEFAULT_PREFIX}"
QUIET=0

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

log()  { [ "${QUIET}" = "1" ] || printf '==> %s\n' "$*" >&2; }
warn() { printf 'warning: %s\n' "$*" >&2; }
err()  { printf 'error: %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

usage() {
  cat >&2 <<EOF
Usage: install.sh [--version vX.Y.Z] [--prefix DIR] [--quiet]

Options:
  --version VERSION  Pin to a specific release tag (e.g. v0.6.0).
                     Default: latest published release.
  --prefix DIR       Install root. Binary lands in DIR/bin.
                     Default: \$HOME/.cloacina (no sudo required).
                     Use /usr/local for a system-wide install.
  --quiet            Suppress informational output.
  -h, --help         Show this help.

Environment:
  CLOACINA_REPO       Override GitHub repo (default ${REPO}).
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version)  VERSION="${2:-}";   shift 2 ;;
    --version=*) VERSION="${1#*=}"; shift   ;;
    --prefix)   PREFIX="${2:-}";    shift 2 ;;
    --prefix=*) PREFIX="${1#*=}";   shift   ;;
    --quiet)    QUIET=1;            shift   ;;
    -h|--help)  usage; exit 0 ;;
    *) err "unknown argument: $1" ;;
  esac
done

[ -n "${PREFIX}" ] || err "--prefix requires a value"

# ---------------------------------------------------------------------------
# Tool dependency check
# ---------------------------------------------------------------------------

need() {
  command -v "$1" >/dev/null 2>&1 || err "required tool '$1' not found in \$PATH"
}
need uname
need tar
need mkdir
need rm

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1" -o "$2"; }
  fetch_stdout() { curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -q "$1" -O "$2"; }
  fetch_stdout() { wget -qO- "$1"; }
else
  err "need 'curl' or 'wget' on \$PATH"
fi

# Pick a SHA256 implementation (sha256sum on Linux, shasum -a 256 on macOS).
if command -v sha256sum >/dev/null 2>&1; then
  sha256_of() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
  sha256_of() { shasum -a 256 "$1" | awk '{print $1}'; }
else
  err "need 'sha256sum' or 'shasum' on \$PATH"
fi

# ---------------------------------------------------------------------------
# Detect target triple
# ---------------------------------------------------------------------------

uname_s="$(uname -s)"
uname_m="$(uname -m)"

case "${uname_s}" in
  Linux)  os="unknown-linux-gnu" ;;
  Darwin) os="apple-darwin"      ;;
  *)      err "unsupported OS: ${uname_s}" ;;
esac

case "${uname_m}" in
  x86_64|amd64)  arch="x86_64"  ;;
  arm64|aarch64) arch="aarch64" ;;
  *)             err "unsupported arch: ${uname_m}" ;;
esac

target="${arch}-${os}"
log "detected target: ${target}"

# ---------------------------------------------------------------------------
# Resolve version
# ---------------------------------------------------------------------------

if [ -z "${VERSION}" ]; then
  log "resolving latest release from github.com/${REPO}"
  api_url="https://api.github.com/repos/${REPO}/releases/latest"
  # Tolerate API rate limits by parsing the redirect from /releases/latest.
  if json="$(fetch_stdout "${api_url}" 2>/dev/null)" && [ -n "${json}" ]; then
    VERSION="$(printf '%s' "${json}" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
  fi
  if [ -z "${VERSION}" ]; then
    err "could not resolve latest release. Pass --version vX.Y.Z to pin."
  fi
fi

# Normalize: accept v0.6.0 or 0.6.0; release tags use the v-prefix form.
case "${VERSION}" in
  v*) tag="${VERSION}"      ;;
  *)  tag="v${VERSION}"     ;;
esac
log "installing ${BINARY} ${tag} for ${target}"

# ---------------------------------------------------------------------------
# Download + verify
# ---------------------------------------------------------------------------

asset="${BINARY}-${tag}-${target}.tar.gz"
sha_asset="${asset}.sha256"
base_url="https://github.com/${REPO}/releases/download/${tag}"

tmpdir="$(mktemp -d)"
trap 'rm -rf "${tmpdir}"' EXIT

log "downloading ${base_url}/${asset}"
fetch "${base_url}/${asset}"     "${tmpdir}/${asset}" \
  || err "failed to download ${asset} — does release ${tag} include this target?"

log "downloading ${base_url}/${sha_asset}"
fetch "${base_url}/${sha_asset}" "${tmpdir}/${sha_asset}" \
  || err "failed to download ${sha_asset}"

expected_sha="$(awk '{print $1}' "${tmpdir}/${sha_asset}")"
actual_sha="$(sha256_of "${tmpdir}/${asset}")"
if [ "${expected_sha}" != "${actual_sha}" ]; then
  err "SHA256 mismatch: expected ${expected_sha}, got ${actual_sha}"
fi
log "checksum verified"

# ---------------------------------------------------------------------------
# Extract + install
# ---------------------------------------------------------------------------

extract_dir="${tmpdir}/extract"
mkdir -p "${extract_dir}"
tar -xzf "${tmpdir}/${asset}" -C "${extract_dir}"

# The tarball contains a single binary at the root. Locate it.
binary_path="$(find "${extract_dir}" -type f -name "${BINARY}" -perm -u+x 2>/dev/null | head -n1)"
if [ -z "${binary_path}" ]; then
  binary_path="$(find "${extract_dir}" -type f -name "${BINARY}" 2>/dev/null | head -n1)"
fi
[ -n "${binary_path}" ] || err "could not find ${BINARY} inside ${asset}"

install_dir="${PREFIX}/bin"
case "${PREFIX}" in
  /usr/local|/opt/*|/usr/*)
    if [ -w "${PREFIX}" ] || ([ -d "${install_dir}" ] && [ -w "${install_dir}" ]); then
      mkdir -p "${install_dir}"
      cp "${binary_path}" "${install_dir}/${BINARY}"
    else
      log "installing to ${install_dir} (requires sudo)"
      sudo mkdir -p "${install_dir}"
      sudo cp "${binary_path}" "${install_dir}/${BINARY}"
      sudo chmod +x "${install_dir}/${BINARY}"
    fi
    ;;
  *)
    mkdir -p "${install_dir}"
    cp "${binary_path}" "${install_dir}/${BINARY}"
    chmod +x "${install_dir}/${BINARY}"
    ;;
esac

log "installed ${BINARY} to ${install_dir}/${BINARY}"

# ---------------------------------------------------------------------------
# PATH advice
# ---------------------------------------------------------------------------

case ":${PATH}:" in
  *":${install_dir}:"*) ;;
  *)
    cat <<EOF >&2

To finish setup, add the install directory to your PATH:

  export PATH="${install_dir}:\$PATH"

You can append that line to ~/.bashrc, ~/.zshrc, or your shell's
equivalent so it persists across sessions.
EOF
    ;;
esac

log "done. run '${BINARY} --version' to verify."
