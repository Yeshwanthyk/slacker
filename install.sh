#!/bin/sh
# Install the slacker binary from GitHub Releases.
#
#   curl -fsSL https://raw.githubusercontent.com/Yeshwanthyk/slacker/main/install.sh | sh
#
# Override behavior with environment variables:
#   SLACKER_VERSION      tag to install (default: latest), e.g. v0.1.0
#   SLACKER_INSTALL_DIR  install directory (default: $HOME/.local/bin)

set -eu

REPO="Yeshwanthyk/slacker"
BIN="slacker"
VERSION="${SLACKER_VERSION:-latest}"
INSTALL_DIR="${SLACKER_INSTALL_DIR:-$HOME/.local/bin}"

os=$(uname -s)
arch=$(uname -m)

case "$os" in
  Darwin) platform="apple-darwin" ;;
  Linux)  platform="unknown-linux-gnu" ;;
  *) echo "slacker: unsupported OS '$os'" >&2; exit 1 ;;
esac

case "$arch" in
  x86_64 | amd64)  cpu="x86_64" ;;
  arm64 | aarch64) cpu="aarch64" ;;
  *) echo "slacker: unsupported architecture '$arch'" >&2; exit 1 ;;
esac

target="${cpu}-${platform}"
archive="${BIN}-${target}.tar.gz"
checksum="${BIN}-${target}.sha256"

if [ "$VERSION" = "latest" ]; then
  base="https://github.com/${REPO}/releases/latest/download"
else
  base="https://github.com/${REPO}/releases/download/${VERSION}"
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

echo "slacker: downloading ${base}/${archive}"
curl -fSL --proto '=https' --tlsv1.2 "${base}/${archive}" -o "${tmp}/${archive}"

if curl -fsSL --proto '=https' --tlsv1.2 "${base}/${checksum}" -o "${tmp}/${checksum}" 2>/dev/null; then
  echo "slacker: verifying checksum"
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$tmp" && sha256sum -c "${checksum}")
  elif command -v shasum >/dev/null 2>&1; then
    (cd "$tmp" && shasum -a 256 -c "${checksum}")
  else
    echo "slacker: no checksum tool found, skipping verification" >&2
  fi
else
  echo "slacker: checksum not available, skipping verification" >&2
fi

tar -xzf "${tmp}/${archive}" -C "$tmp"
mkdir -p "$INSTALL_DIR"
install -m 0755 "${tmp}/${BIN}" "${INSTALL_DIR}/${BIN}"
echo "slacker: installed to ${INSTALL_DIR}/${BIN}"

case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *) echo "slacker: ${INSTALL_DIR} is not on your PATH — add it to use 'slacker' directly" >&2 ;;
esac
