#!/usr/bin/env bash
set -euo pipefail

# PortZero CLI installer
# Usage: curl -fsSL https://goport0.dev/install.sh | bash

REPO="portzero-dev/portzero"
BINARY="portzero"
INSTALL_DIR="${PORTZERO_INSTALL_DIR:-/usr/local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { printf "${CYAN}info${NC}  %s\n" "$1"; }
ok()    { printf "${GREEN}ok${NC}    %s\n" "$1"; }
warn()  { printf "${YELLOW}warn${NC}  %s\n" "$1"; }
error() { printf "${RED}error${NC} %s\n" "$1" >&2; exit 1; }

# Detect OS
detect_os() {
  case "$(uname -s)" in
    Linux*)  echo "linux" ;;
    Darwin*) echo "darwin" ;;
    *)       error "Unsupported OS: $(uname -s). PortZero supports Linux and macOS." ;;
  esac
}

# Detect architecture
detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)  echo "x86_64" ;;
    arm64|aarch64)  echo "aarch64" ;;
    *)              error "Unsupported architecture: $(uname -m). PortZero supports x86_64 and aarch64." ;;
  esac
}

# Get latest release tag matching cli-v*
get_latest_version() {
  local url="https://api.github.com/repos/${REPO}/releases"
  local response

  if command -v curl &>/dev/null; then
    response=$(curl -fsSL "$url" 2>/dev/null)
  elif command -v wget &>/dev/null; then
    response=$(wget -qO- "$url" 2>/dev/null)
  else
    error "Neither curl nor wget found. Please install one of them."
  fi

  echo "$response" | grep -o '"tag_name": *"cli-v[^"]*"' | head -1 | sed 's/.*"cli-v\([^"]*\)".*/\1/'
}

# Download a URL to a file
download() {
  local url="$1"
  local dest="$2"

  if command -v curl &>/dev/null; then
    curl -fsSL "$url" -o "$dest"
  elif command -v wget &>/dev/null; then
    wget -qO "$dest" "$url"
  fi
}

main() {
  printf "\n"
  printf "${CYAN}  PortZero CLI Installer${NC}\n"
  printf "  ─────────────────────\n\n"

  local os arch version asset_name url tmp_dir

  os=$(detect_os)
  arch=$(detect_arch)

  info "Detected platform: ${os}-${arch}"

  # Allow version override via env var
  if [ -n "${PORTZERO_VERSION:-}" ]; then
    version="$PORTZERO_VERSION"
    info "Using specified version: v${version}"
  else
    info "Fetching latest release..."
    version=$(get_latest_version)
    if [ -z "$version" ]; then
      error "Could not determine the latest version. Check https://github.com/${REPO}/releases"
    fi
    info "Latest version: v${version}"
  fi

  asset_name="${BINARY}-${os}-${arch}.tar.gz"
  url="https://github.com/${REPO}/releases/download/cli-v${version}/${asset_name}"

  info "Downloading ${asset_name}..."

  tmp_dir=$(mktemp -d)
  trap 'rm -rf "$tmp_dir"' EXIT

  download "$url" "${tmp_dir}/${asset_name}" || \
    error "Download failed. Check that version v${version} exists at:\n  ${url}"

  info "Extracting..."
  tar xzf "${tmp_dir}/${asset_name}" -C "$tmp_dir"

  # Install binary
  if [ -w "$INSTALL_DIR" ]; then
    mv "${tmp_dir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
  else
    info "Installing to ${INSTALL_DIR} (requires sudo)..."
    sudo mv "${tmp_dir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
  fi

  chmod +x "${INSTALL_DIR}/${BINARY}"

  ok "Installed ${BINARY} to ${INSTALL_DIR}/${BINARY}"

  # Verify
  if command -v "$BINARY" &>/dev/null; then
    ok "$(${BINARY} --version 2>/dev/null || echo "${BINARY} installed successfully")"
  else
    warn "${INSTALL_DIR} is not in your PATH. Add it with:"
    printf "  export PATH=\"%s:\$PATH\"\n" "$INSTALL_DIR"
  fi

  printf "\n"
  printf "  ${GREEN}Get started:${NC}\n"
  printf "    portzero start -d       # Start the daemon\n"
  printf "    portzero next dev       # Run your app\n"
  printf "    portzero list           # See running apps\n"
  printf "\n"
  printf "  ${CYAN}Docs:${NC} https://goport0.dev/docs\n"
  printf "\n"
}

main
