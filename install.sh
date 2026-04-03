#!/usr/bin/env bash
# install.sh — Download and install the latest cloacinactl binary.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/colliery-io/cloacina/main/install.sh | bash
#
# Options (environment variables):
#   CLOACINACTL_VERSION   — specific version to install (e.g. "v0.3.2"); default: latest
#   INSTALL_DIR           — where to put the binary; default: ~/.local/bin

set -euo pipefail

REPO="colliery-io/cloacina"
BINARY="cloacinactl"

# ── Detect platform ──────────────────────────────────────────────────────────

detect_os() {
  case "$(uname -s)" in
    Linux*)  echo "linux" ;;
    Darwin*) echo "darwin" ;;
    *)
      echo "Unsupported OS: $(uname -s)" >&2
      exit 1
      ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)   echo "x86_64" ;;
    aarch64|arm64)   echo "aarch64" ;;
    *)
      echo "Unsupported architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

OS="$(detect_os)"
ARCH="$(detect_arch)"

# Map to Rust target triple
case "${OS}-${ARCH}" in
  linux-x86_64)    TARGET="x86_64-unknown-linux-gnu" ;;
  linux-aarch64)   TARGET="aarch64-unknown-linux-gnu" ;;
  darwin-x86_64)   TARGET="x86_64-apple-darwin" ;;
  darwin-aarch64)  TARGET="aarch64-apple-darwin" ;;
  *)
    echo "No pre-built binary for ${OS}-${ARCH}" >&2
    exit 1
    ;;
esac

echo "Detected platform: ${OS}/${ARCH} -> ${TARGET}"

# ── Resolve version ──────────────────────────────────────────────────────────

if [ -n "${CLOACINACTL_VERSION:-}" ]; then
  VERSION="${CLOACINACTL_VERSION}"
else
  echo "Fetching latest release..."
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
  if [ -z "${VERSION}" ]; then
    echo "Failed to determine latest version" >&2
    exit 1
  fi
fi

echo "Installing ${BINARY} ${VERSION}..."

# ── Download ─────────────────────────────────────────────────────────────────

ARCHIVE="${BINARY}-${VERSION}-${TARGET}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "${TMPDIR}"' EXIT

echo "Downloading ${ARCHIVE}..."
curl -fsSL "${BASE_URL}/${ARCHIVE}" -o "${TMPDIR}/${ARCHIVE}"
curl -fsSL "${BASE_URL}/${ARCHIVE}.sha256" -o "${TMPDIR}/${ARCHIVE}.sha256"

# ── Verify checksum ──────────────────────────────────────────────────────────

echo "Verifying checksum..."
cd "${TMPDIR}"
if command -v sha256sum &>/dev/null; then
  sha256sum -c "${ARCHIVE}.sha256"
elif command -v shasum &>/dev/null; then
  shasum -a 256 -c "${ARCHIVE}.sha256"
else
  echo "Warning: no sha256sum or shasum found, skipping checksum verification" >&2
fi
cd - >/dev/null

# ── Install ──────────────────────────────────────────────────────────────────

INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
mkdir -p "${INSTALL_DIR}"

tar xzf "${TMPDIR}/${ARCHIVE}" -C "${TMPDIR}"
install -m 755 "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"

echo ""
echo "Installed ${BINARY} to ${INSTALL_DIR}/${BINARY}"

# ── Verify ───────────────────────────────────────────────────────────────────

if command -v "${BINARY}" &>/dev/null; then
  echo ""
  "${BINARY}" --version
else
  echo ""
  echo "Note: ${INSTALL_DIR} is not in your PATH."
  echo "Add it with:"
  echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
  echo ""
  "${INSTALL_DIR}/${BINARY}" --version
fi

echo ""
echo "Done!"
