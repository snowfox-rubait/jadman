#!/usr/bin/env bash

# JADMan Installer Script
# Downloads and installs precompiled JADMan binaries from GitHub releases.
# Automates browser native messaging configurations for Linux and macOS.

set -euo pipefail

REPO="snowfox-rubait/JADMan"
GITHUB_API="https://api.github.com/repos/${REPO}/releases/latest"

# Styling colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color
BOLD='\033[1m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# 1. OS and Architecture Detection
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

TAR_NAME=""
if [ "$OS" = "linux" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        TAR_NAME="jadman-x86_64-unknown-linux-gnu.tar.gz"
    else
        log_error "Linux architecture '${ARCH}' is not supported. Precompiled binaries are currently only built for x86_64."
    fi
elif [ "$OS" = "darwin" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        TAR_NAME="jadman-x86_64-apple-darwin.tar.gz"
    elif [ "$ARCH" = "arm64" ]; then
        TAR_NAME="jadman-aarch64-apple-darwin.tar.gz"
    else
        log_error "macOS architecture '${ARCH}' is not supported."
    fi
else
    log_error "Operating System '${OS}' is not supported by this installer script."
fi

# 2. Select Installation Path
INSTALL_DIR=""
if [ "$EUID" -ne 0 ]; then
    # Running as normal user, install to user local bin
    INSTALL_DIR="${HOME}/.local/bin"
    log_info "Running as non-root user. Installing to '${INSTALL_DIR}'."
    mkdir -p "${INSTALL_DIR}"
    
    # Check if directory is in PATH
    if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
        log_warn "'${INSTALL_DIR}' is not in your PATH. You may need to add it to your shell profile (~/.bashrc or ~/.zshrc)."
    fi
else
    # Running as root, install to global /usr/local/bin
    INSTALL_DIR="/usr/local/bin"
    log_info "Running as root. Installing globally to '${INSTALL_DIR}'."
fi

# 3. Retrieve Latest Release Download Links
log_info "Fetching latest release information from GitHub..."
RELEASE_JSON=$(curl -s "${GITHUB_API}")

if echo "${RELEASE_JSON}" | grep -q "Not Found"; then
    log_error "No release found on GitHub repository '${REPO}' yet. Make sure a release has been published."
fi

LATEST_TAG=$(echo "${RELEASE_JSON}" | grep -oP '"tag_name":\s*"\K[^"]+')
log_info "Found latest release version: ${LATEST_TAG}"

TAR_URL="https://github.com/${REPO}/releases/download/${LATEST_TAG}/${TAR_NAME}"

# Create a temporary directory for downloads
TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT

log_info "Downloading ${TAR_NAME}..."
curl -L -o "${TMP_DIR}/${TAR_NAME}" "${TAR_URL}"

log_info "Extracting archive..."
tar -xzf "${TMP_DIR}/${TAR_NAME}" -C "${TMP_DIR}"

# 4. Install Binaries
log_info "Installing binaries into '${INSTALL_DIR}'..."
install -m 755 "${TMP_DIR}/jadm-daemon" "${INSTALL_DIR}/jadm-daemon"
install -m 755 "${TMP_DIR}/jadm-tui" "${INSTALL_DIR}/jadm-tui"

# 5. Create Default Configuration Directories
CONFIG_DIR="${HOME}/.config/jadm"
log_info "Creating default configuration directory at '${CONFIG_DIR}'..."
mkdir -p "${CONFIG_DIR}"

# 6. Configure Native Messaging Host
log_info "Configuring browser Native Messaging hosts..."

MANIFEST_CONTENT=$(cat <<EOF
{
  "name": "com.jadm.jadm",
  "description": "JADMan integration",
  "path": "${INSTALL_DIR}/jadm-daemon",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://ipiefkjcicogeoepimgebinafoelhbhd/"
  ]
}
EOF
)

CHROME_NM_DIRS=()
FIREFOX_NM_DIRS=()

if [ "$OS" = "linux" ]; then
    CHROME_NM_DIRS=(
        "${HOME}/.config/google-chrome/NativeMessagingHosts"
        "${HOME}/.config/chromium/NativeMessagingHosts"
        "${HOME}/.config/BraveSoftware/Brave-Browser/NativeMessagingHosts"
    )
    FIREFOX_NM_DIRS=(
        "${HOME}/.mozilla/native-messaging-hosts"
    )
elif [ "$OS" = "darwin" ]; then
    CHROME_NM_DIRS=(
        "${HOME}/Library/Application Support/Google/Chrome/NativeMessagingHosts"
        "${HOME}/Library/Application Support/Chromium/NativeMessagingHosts"
        "${HOME}/Library/Application Support/BraveSoftware/Brave-Browser/NativeMessagingHosts"
    )
    FIREFOX_NM_DIRS=(
        "${HOME}/Library/Application Support/Mozilla/NativeMessagingHosts"
    )
fi

# Write Chrome / Brave / Chromium manifests
for dir in "${CHROME_NM_DIRS[@]}"; do
    PARENT_DIR=$(dirname "$dir")
    if [ -d "$PARENT_DIR" ]; then
        mkdir -p "$dir"
        echo "${MANIFEST_CONTENT}" > "${dir}/com.jadm.jadm.json"
        log_info "Configured Chrome-based host manifest at: ${dir}/com.jadm.jadm.json"
    fi
done

# Write Firefox manifests
for dir in "${FIREFOX_NM_DIRS[@]}"; do
    mkdir -p "$dir"
    echo "${MANIFEST_CONTENT}" > "${dir}/com.jadm.jadm.json"
    log_info "Configured Firefox host manifest at: ${dir}/com.jadm.jadm.json"
done

log_success "JADMan (${LATEST_TAG}) has been successfully installed!"
echo -e "\n${BOLD}Next steps:${NC}"
echo -e "  1. Run the daemon:  ${GREEN}jadm-daemon &${NC}"
echo -e "  2. Run the TUI:     ${GREEN}jadm-tui${NC}"
echo -e "  3. Download the unpacked browser extensions from: https://github.com/${REPO}"
echo -e "     and load them in Developer Mode in Chrome (chrome://extensions) or Firefox (about:debugging)."
echo -e "     (The native messaging host has already been automatically registered for you!)"
