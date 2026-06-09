#!/usr/bin/env bash
set -euo pipefail

REPO="thinkjones/dj"
INSTALL_DIR="${HOME}/.local/bin"

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    arm64|aarch64)
        TARGET="aarch64-apple-darwin"
        ;;
    x86_64)
        TARGET="x86_64-apple-darwin"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

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
