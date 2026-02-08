#!/bin/sh
set -eu

# Color output helpers
if [ -t 1 ]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[0;33m'
  BLUE='\033[0;34m'
  BOLD='\033[1m'
  RESET='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  BLUE=''
  BOLD=''
  RESET=''
fi

info()    { printf "${BLUE}info${RESET}  %s\n" "$1"; }
warn()    { printf "${YELLOW}warn${RESET}  %s\n" "$1"; }
error()   { printf "${RED}error${RESET} %s\n" "$1" >&2; }
success() { printf "${GREEN}  ok${RESET}  %s\n" "$1"; }

REPO="juicyjusung/nr"
BINARY_NAME="nr"
INSTALL_DIR="${HOME}/.local/bin"

usage() {
  cat << EOF
${BOLD}nr installer${RESET}

Install the nr binary from GitHub releases.

${BOLD}USAGE${RESET}
    install.sh [OPTIONS]

${BOLD}OPTIONS${RESET}
    --install-dir <DIR>   Installation directory (default: ~/.local/bin)
    --help                Show this help message

${BOLD}EXAMPLES${RESET}
    curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh
    curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- --install-dir /usr/local/bin
EOF
  exit 0
}

# Parse arguments
while [ $# -gt 0 ]; do
  case "$1" in
    --help)
      usage
      ;;
    --install-dir)
      INSTALL_DIR="$2"
      shift 2
      ;;
    *)
      error "Unknown option: $1"
      usage
      ;;
  esac
done

# Detect OS
detect_os() {
  case "$(uname -s)" in
    Darwin) echo "macos" ;;
    Linux)  echo "linux" ;;
    *)
      error "Unsupported OS: $(uname -s)"
      error "nr supports macOS and Linux. For Windows, use Scoop or cargo-binstall."
      exit 1
      ;;
  esac
}

# Detect architecture
detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)  echo "x86_64" ;;
    aarch64|arm64)  echo "aarch64" ;;
    *)
      error "Unsupported architecture: $(uname -m)"
      exit 1
      ;;
  esac
}

# Map OS/arch to Rust target triple
target_triple() {
  local os="$1"
  local arch="$2"
  case "${os}-${arch}" in
    macos-x86_64)   echo "x86_64-apple-darwin" ;;
    macos-aarch64)  echo "aarch64-apple-darwin" ;;
    linux-x86_64)   echo "x86_64-unknown-linux-gnu" ;;
    linux-aarch64)  echo "aarch64-unknown-linux-gnu" ;;
    *)
      error "Unsupported platform: ${os} ${arch}"
      exit 1
      ;;
  esac
}

# Find a download tool
find_downloader() {
  if command -v curl > /dev/null 2>&1; then
    echo "curl"
  elif command -v wget > /dev/null 2>&1; then
    echo "wget"
  else
    error "Neither curl nor wget found. Please install one and try again."
    exit 1
  fi
}

# Download a URL to a file
download() {
  local url="$1"
  local output="$2"
  local downloader
  downloader="$(find_downloader)"

  case "$downloader" in
    curl) curl -fsSL "$url" -o "$output" ;;
    wget) wget -q "$url" -O "$output" ;;
  esac
}

# Download a URL to stdout
download_stdout() {
  local url="$1"
  local downloader
  downloader="$(find_downloader)"

  case "$downloader" in
    curl) curl -fsSL "$url" ;;
    wget) wget -q "$url" -O - ;;
  esac
}

# Checksum verification
verify_checksum() {
  local file="$1"
  local expected="$2"
  local os="$3"
  local actual

  if [ "$os" = "macos" ]; then
    actual="$(shasum -a 256 "$file" | awk '{print $1}')"
  else
    actual="$(sha256sum "$file" | awk '{print $1}')"
  fi

  if [ "$actual" != "$expected" ]; then
    error "Checksum verification failed!"
    error "  Expected: ${expected}"
    error "  Actual:   ${actual}"
    exit 1
  fi
}

main() {
  printf "\n${BOLD}  nr installer${RESET}\n\n"

  local os arch target
  os="$(detect_os)"
  arch="$(detect_arch)"
  target="$(target_triple "$os" "$arch")"

  info "Detected platform: ${os} ${arch} (${target})"

  # Fetch latest version
  info "Fetching latest release..."
  local latest_json latest_tag version
  latest_json="$(download_stdout "https://api.github.com/repos/${REPO}/releases/latest")"
  latest_tag="$(printf '%s' "$latest_json" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//')"

  if [ -z "$latest_tag" ]; then
    error "Failed to determine latest version"
    exit 1
  fi

  version="${latest_tag#v}"
  info "Latest version: ${version} (${latest_tag})"

  # Build download URLs
  local archive_name="nr-v${version}-${target}.tar.gz"
  local checksums_name="nr-v${version}-checksums.sha256"
  local base_url="https://github.com/${REPO}/releases/download/${latest_tag}"
  local archive_url="${base_url}/${archive_name}"
  local checksums_url="${base_url}/${checksums_name}"

  # Create temp directory
  local tmpdir
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  # Download archive
  info "Downloading ${archive_name}..."
  download "$archive_url" "${tmpdir}/${archive_name}"
  success "Downloaded archive"

  # Download checksums and verify
  info "Verifying checksum..."
  download "$checksums_url" "${tmpdir}/${checksums_name}"
  local expected_hash
  expected_hash="$(grep "${archive_name}" "${tmpdir}/${checksums_name}" | awk '{print $1}')"

  if [ -z "$expected_hash" ]; then
    error "Could not find checksum for ${archive_name}"
    exit 1
  fi

  verify_checksum "${tmpdir}/${archive_name}" "$expected_hash" "$os"
  success "Checksum verified"

  # Extract binary
  info "Extracting binary..."
  tar xzf "${tmpdir}/${archive_name}" -C "$tmpdir"
  success "Extracted"

  # Install
  info "Installing to ${INSTALL_DIR}..."
  mkdir -p "$INSTALL_DIR"
  cp "${tmpdir}/nr-v${version}-${target}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
  chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
  success "Installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"

  # Check if install dir is in PATH
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      printf "\n"
      warn "${INSTALL_DIR} is not in your PATH."
      info "Add the following to your shell profile:"
      printf "\n    ${BOLD}export PATH=\"%s:\$PATH\"${RESET}\n\n" "$INSTALL_DIR"
      ;;
  esac

  printf "\n  ${GREEN}${BOLD}nr v${version}${RESET} installed successfully! Run ${BOLD}nr${RESET} to get started.\n\n"
}

main
