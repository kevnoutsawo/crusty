#!/bin/sh
# Crusty installer — detects OS/arch and installs the latest release.
# Usage: curl -sSf https://raw.githubusercontent.com/kevnoutsawo/crusty/main/install.sh | sh

set -eu

REPO="kevnoutsawo/crusty"
BINARY="crusty-tui"
INSTALL_DIR="/usr/local/bin"

main() {
    need_cmd curl
    need_cmd tar

    local os arch target tag url tmpdir

    os="$(detect_os)"
    arch="$(detect_arch)"
    target="${arch}-${os}"

    printf "Detected platform: %s\n" "$target"

    tag="$(latest_tag)"
    printf "Latest release: %s\n" "$tag"

    url="https://github.com/${REPO}/releases/download/${tag}/crusty-${target}.tar.gz"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    printf "Downloading %s...\n" "$url"
    curl -sSfL "$url" -o "${tmpdir}/crusty.tar.gz"
    tar xzf "${tmpdir}/crusty.tar.gz" -C "$tmpdir"

    if [ -w "$INSTALL_DIR" ]; then
        mv "${tmpdir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    else
        printf "Installing to %s (requires sudo)...\n" "$INSTALL_DIR"
        sudo mv "${tmpdir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    fi
    chmod +x "${INSTALL_DIR}/${BINARY}"

    printf "\nCrusty %s installed to %s/%s\n" "$tag" "$INSTALL_DIR" "$BINARY"
    printf "Run 'crusty-tui' to get started.\n"
}

detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "unknown-linux-gnu" ;;
        Darwin*) echo "apple-darwin" ;;
        *)       err "Unsupported OS: $(uname -s). Use the Windows installer or build from source." ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)             err "Unsupported architecture: $(uname -m)" ;;
    esac
}

latest_tag() {
    curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" \
        | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p'
}

need_cmd() {
    if ! command -v "$1" > /dev/null 2>&1; then
        err "Required command not found: $1"
    fi
}

err() {
    printf "error: %s\n" "$1" >&2
    exit 1
}

main
