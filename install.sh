#!/usr/bin/env bash
# Tethys Log — install / uninstall script.
#
# Usage:
#   ./install.sh                 — build release and install to ~/.local (no sudo, default)
#   ./install.sh --user          — same as above, explicit
#   sudo ./install.sh --system   — build release and install to /usr/local (needs sudo)
#   ./install.sh --uninstall              — remove the ~/.local install
#   sudo ./install.sh --system --uninstall — remove the /usr/local install
#
# Runtime dependencies that must be present before running:
#   GTK 4, GStreamer good+bad plugins, gdk-pixbuf, yt-dlp (optional, for YouTube playback)
#   See README.md for per-distro package names.
set -euo pipefail

APP_ID="com.tethyslog.app"
APP_NAME="tethys-log"
VERSION="0.1.0"

# ── argument parsing ──────────────────────────────────────────────────────────
# User installs to ~/.local are the default and don't need sudo. --system opts
# into a shared /usr/local install, which does need sudo.

SYSTEM_INSTALL=0
UNINSTALL=0
for arg in "$@"; do
    case "$arg" in
        --user)      SYSTEM_INSTALL=0 ;;
        --system)    SYSTEM_INSTALL=1 ;;
        --uninstall) UNINSTALL=1 ;;
        *)
            echo "Unknown option: $arg" >&2
            echo "Usage: $0 [--user|--system] [--uninstall]" >&2
            exit 1
            ;;
    esac
done

if [[ $SYSTEM_INSTALL -eq 1 ]]; then
    PREFIX="/usr/local"
else
    PREFIX="$HOME/.local"
fi

# A plain (non---system) run should never be sudo'd: it silently builds and
# installs as root, which then can't find your user's rustup toolchain and
# fails with a confusing "no default toolchain configured" error that has
# nothing to do with Tethys itself.
if [[ $EUID -eq 0 && $SYSTEM_INSTALL -eq 0 ]]; then
    echo "Error: user installation should not be run with sudo." >&2
    echo "" >&2
    echo "Use:" >&2
    echo "  ./install.sh" >&2
    echo "" >&2
    echo "For a system-wide installation instead:" >&2
    echo "  sudo ./install.sh --system" >&2
    exit 1
fi

BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share"
ICON_DIR="$SHARE_DIR/icons/hicolor/scalable/apps"
DESKTOP_DIR="$SHARE_DIR/applications"
MIME_DIR="$SHARE_DIR/mime/packages"

# ── uninstall path ────────────────────────────────────────────────────────────

if [[ $UNINSTALL -eq 1 ]]; then
    echo "Removing Tethys Log from $PREFIX ..."
    rm -f "$BIN_DIR/$APP_NAME"
    rm -f "$ICON_DIR/$APP_ID.svg"
    rm -f "$DESKTOP_DIR/$APP_ID.desktop"
    rm -f "$MIME_DIR/$APP_ID.mime.xml"
    gtk-update-icon-cache "$SHARE_DIR/icons/hicolor" 2>/dev/null || true
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
    update-mime-database "$SHARE_DIR/mime" 2>/dev/null || true
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

mkdir -p "$BIN_DIR" "$ICON_DIR" "$DESKTOP_DIR" "$MIME_DIR"

echo "Installing binary  → $BIN_DIR/$APP_NAME"
install -m 755 "$BINARY" "$BIN_DIR/$APP_NAME"

echo "Installing icon    → $ICON_DIR/$APP_ID.svg"
cp "assets/$APP_ID.svg" "$ICON_DIR/$APP_ID.svg"

echo "Installing .desktop → $DESKTOP_DIR/$APP_ID.desktop"
sed "s|@PREFIX@|$PREFIX|g" "assets/$APP_ID.desktop.in" \
    > "$DESKTOP_DIR/$APP_ID.desktop"

echo "Installing MIME type → $MIME_DIR/$APP_ID.mime.xml"
cp "assets/$APP_ID.mime.xml" "$MIME_DIR/$APP_ID.mime.xml"

# refresh caches — failures are non-fatal (some minimal environments lack the tools)
gtk-update-icon-cache "$SHARE_DIR/icons/hicolor" 2>/dev/null || true
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
update-mime-database "$SHARE_DIR/mime" 2>/dev/null || true

echo ""
echo "Tethys Log $VERSION installed to $PREFIX."
echo "Launch: tethys-log   or search 'Tethys Log' in your app grid."

# Warn (don't auto-edit shell rc files — that's surprising and easy to get wrong)
# if the install dir isn't already on PATH.
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo ""
    echo "Note: $BIN_DIR is not currently in your PATH, so 'tethys-log' won't"
    echo "run directly from a new shell yet. Add it by putting this line in"
    echo "your shell config (~/.bashrc, ~/.zshrc, etc.) and restarting your shell:"
    echo ""
    echo "  export PATH=\"$BIN_DIR:\$PATH\""
fi
