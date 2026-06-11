#!/usr/bin/env bash
set -euo pipefail

REPO="thinkjones/dj"
INSTALL_DIR="${HOME}/.local/bin"

# Detect OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in
    darwin)
        OS_TARGET="apple-darwin"
        ;;
    linux)
        OS_TARGET="unknown-linux-gnu"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    arm64|aarch64)
        ARCH_TARGET="aarch64"
        ;;
    x86_64|amd64)
        ARCH_TARGET="x86_64"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

TARGET="${ARCH_TARGET}-${OS_TARGET}"

echo "→ Detected architecture: $TARGET"

# Create install directory
mkdir -p "$INSTALL_DIR"

# Download latest release
URL="https://github.com/${REPO}/releases/latest/download/dj-${TARGET}.tar.gz"
echo "→ Downloading dj from ${URL}..."

curl -fsSL "$URL" -o /tmp/dj.tar.gz

# Extract
tar -xzf /tmp/dj.tar.gz -C /tmp
rm /tmp/dj.tar.gz

# Install binary
mv /tmp/dj "$INSTALL_DIR/dj"
chmod +x "$INSTALL_DIR/dj"

# Ensure PATH
if ! command -v dj &> /dev/null; then
    echo ""
    echo "⚠ ${INSTALL_DIR} is not on your PATH."
    echo "  Add this to your shell rc file:"
    echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
    echo ""
fi

echo "✓ dj installed to ${INSTALL_DIR}/dj"

# Run onboarding
if command -v dj &> /dev/null; then
    echo ""
    echo "→ Running dj onboard..."
    dj onboard
else
    echo ""
    echo "→ Run '${INSTALL_DIR}/dj onboard' to set up your catalog."
fi
