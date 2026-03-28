#!/bin/sh
set -e

REPO="danielhirt/kedge"
INSTALL_DIR="${KEDGE_INSTALL_DIR:-/usr/local/bin}"

get_arch() {
  case "$(uname -m)" in
    x86_64|amd64) echo "x86_64" ;;
    aarch64|arm64) echo "aarch64" ;;
    *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
  esac
}

get_target() {
  arch="$(get_arch)"
  case "$(uname -s)" in
    Linux)  echo "${arch}-unknown-linux-gnu" ;;
    Darwin) echo "${arch}-apple-darwin" ;;
    *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
  esac
}

get_latest_version() {
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" |
    grep '"tag_name"' | sed 's/.*"tag_name": *"//;s/".*//'
}

main() {
  target="$(get_target)"
  version="${1:-$(get_latest_version)}"

  if [ -z "$version" ]; then
    echo "Error: could not determine latest version." >&2
    echo "Usage: $0 [version]" >&2
    echo "  e.g. $0 v0.1.0" >&2
    exit 1
  fi

  url="https://github.com/${REPO}/releases/download/${version}/kedge-${target}.tar.gz"
  echo "Downloading kedge ${version} for ${target}..."

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  curl -fsSL "$url" -o "${tmpdir}/kedge.tar.gz"
  tar xzf "${tmpdir}/kedge.tar.gz" -C "$tmpdir"

  if [ -w "$INSTALL_DIR" ]; then
    mv "${tmpdir}/kedge" "${INSTALL_DIR}/kedge"
  else
    echo "Installing to ${INSTALL_DIR} (requires sudo)..."
    sudo mv "${tmpdir}/kedge" "${INSTALL_DIR}/kedge"
  fi

  chmod +x "${INSTALL_DIR}/kedge"
  echo "Installed kedge ${version} to ${INSTALL_DIR}/kedge"
}

main "$@"
