#!/usr/bin/env bash
# ==============================================================================
# Ailang Package Script: Builds distributable tarball and self-extracting installer
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="$SCRIPT_DIR/dist"
BUNDLE_DIR="$DIST_DIR/bundle"
VERSION="0.1.0"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
TARBALL_NAME="ailang-v${VERSION}-${OS}-${ARCH}.tar.gz"

echo "=== Building Ailang Release Package ==="

# 1. Compile Release Binary
echo "Building release binary with cargo..."
cd "$SCRIPT_DIR"
cargo build --release

# 2. Prepare Bundle Directory
rm -rf "$DIST_DIR"
mkdir -p "$BUNDLE_DIR/bin"
mkdir -p "$BUNDLE_DIR/examples"

cp "$SCRIPT_DIR/target/release/ailang" "$BUNDLE_DIR/bin/"
cp "$SCRIPT_DIR/install.sh" "$BUNDLE_DIR/"
cp "$SCRIPT_DIR/README.md" "$BUNDLE_DIR/"
cp "$SCRIPT_DIR/LICENSE" "$BUNDLE_DIR/"
cp -R "$SCRIPT_DIR/examples/"* "$BUNDLE_DIR/examples/"

# 3. Create Distributable Tarball
echo "Creating archive: $DIST_DIR/$TARBALL_NAME..."
(cd "$BUNDLE_DIR" && tar -czf "$DIST_DIR/$TARBALL_NAME" .)

# 4. Create Standalone Self-Extracting Installer (.sh)
SELF_EXTRACTOR="$DIST_DIR/ailang-installer.sh"
echo "Creating self-extracting installer: $SELF_EXTRACTOR..."

cat << 'EOF' > "$SELF_EXTRACTOR"
#!/usr/bin/env bash
# ==============================================================================
# Ailang Self-Extracting Single Installer
# ==============================================================================
set -e

TMP_DIR=$(mktemp -d /tmp/ailang_installer_XXXXXX)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "Extracting Ailang installation bundle..."
# Find the line number where the payload starts
ARCHIVE_LINE=$(awk '/^__PAYLOAD_BELOW__/ {print NR + 1; exit 0; }' "$0")

# Extract the tar.gz payload from the bottom of this script
tail -n +"$ARCHIVE_LINE" "$0" | tar -xzf - -C "$TMP_DIR"

# Execute the inner installer
bash "$TMP_DIR/install.sh" "$@"
exit 0

__PAYLOAD_BELOW__
EOF

# Append the binary tarball to the self-extracting script
cat "$DIST_DIR/$TARBALL_NAME" >> "$SELF_EXTRACTOR"
chmod +x "$SELF_EXTRACTOR"

echo "=== Package Build Succeeded ==="
echo "Tarball:   $DIST_DIR/$TARBALL_NAME ($(du -h "$DIST_DIR/$TARBALL_NAME" | cut -f1))"
echo "Installer: $SELF_EXTRACTOR ($(du -h "$SELF_EXTRACTOR" | cut -f1))"
echo ""
echo "To install on any system, run:"
echo "  ./dist/ailang-installer.sh"
