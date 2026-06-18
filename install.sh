#!/usr/bin/env bash
# Tethys Log — system-wide install / uninstall script.
#
# Usage:
#   ./install.sh              — build release and install to /usr/local (needs sudo)
#   ./install.sh --user       — build release and install to ~/.local  (no sudo)
#   ./install.sh --uninstall [--user]  — remove all installed files
#
# Runtime dependencies that must be present before running:
#   GTK 4, GStreamer good+bad plugins, gdk-pixbuf, yt-dlp (optional, for YouTube playback)
#   See README.md for per-distro package names.
set -euo pipefail

APP_ID="com.tethyslog.app"
APP_NAME="tethys-log"
VERSION="0.1.0"

# ── argument parsing ──────────────────────────────────────────────────────────

USER_INSTALL=0
UNINSTALL=0
for arg in "$@"; do
    case "$arg" in
        --user)      USER_INSTALL=1 ;;
        --uninstall) UNINSTALL=1 ;;
    esac
done

if [[ $USER_INSTALL -eq 1 ]]; then
    PREFIX="$HOME/.local"
else
    PREFIX="/usr/local"
fi

BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share"
ICON_DIR="$SHARE_DIR/icons/hicolor/scalable/apps"
DESKTOP_DIR="$SHARE_DIR/applications"

# ── uninstall path ────────────────────────────────────────────────────────────

if [[ $UNINSTALL -eq 1 ]]; then
    echo "Removing Tethys Log from $PREFIX ..."
    rm -f "$BIN_DIR/$APP_NAME"
    rm -f "$ICON_DIR/$APP_ID.svg"
    rm -f "$DESKTOP_DIR/$APP_ID.desktop"
    gtk-update-icon-cache "$SHARE_DIR/icons/hicolor" 2>/dev/null || true
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
    echo "Done."
    exit 0
fi

# ── pre-flight ────────────────────────────────────────────────────────────────

if [[ ! -f "Cargo.toml" ]]; then
    echo "Error: run install.sh from the project root (where Cargo.toml lives)." >&2
    exit 1
fi

if ! command -v cargo &>/dev/null; then
    echo "Error: cargo not found. Install Rust from https://rustup.rs" >&2
    exit 1
fi

# ── build ─────────────────────────────────────────────────────────────────────

echo "Building Tethys Log $VERSION (release) ..."
cargo build --release

BINARY="target/release/$APP_NAME"
if [[ ! -f "$BINARY" ]]; then
    echo "Error: build succeeded but $BINARY not found." >&2
    exit 1
fi

# ── install ───────────────────────────────────────────────────────────────────

mkdir -p "$BIN_DIR" "$ICON_DIR" "$DESKTOP_DIR"

echo "Installing binary  → $BIN_DIR/$APP_NAME"
install -m 755 "$BINARY" "$BIN_DIR/$APP_NAME"

echo "Installing icon    → $ICON_DIR/$APP_ID.svg"
cp "assets/$APP_ID.svg" "$ICON_DIR/$APP_ID.svg"

echo "Installing .desktop → $DESKTOP_DIR/$APP_ID.desktop"
sed "s|@PREFIX@|$PREFIX|g" "assets/$APP_ID.desktop.in" \
    > "$DESKTOP_DIR/$APP_ID.desktop"

# refresh caches — failures are non-fatal (some minimal environments lack the tools)
gtk-update-icon-cache "$SHARE_DIR/icons/hicolor" 2>/dev/null || true
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

echo ""
echo "Tethys Log $VERSION installed to $PREFIX."
echo "Launch: tethys-log   or search 'Tethys Log' in your app grid."
