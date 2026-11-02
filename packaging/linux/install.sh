#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ "$(id -u)" -eq 0 ]; then
    BIN_DIR="/usr/local/bin"
    APP_DIR="/usr/local/share/applications"
    ICON_DIR="/usr/local/share/icons/hicolor/256x256/apps"
else
    BIN_DIR="$HOME/.local/bin"
    APP_DIR="$HOME/.local/share/applications"
    ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
fi

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

install -m 755 "$SCRIPT_DIR/zalo-rust" "$BIN_DIR/zalo-rust"
install -m 755 "$SCRIPT_DIR/ctdozlite" "$BIN_DIR/ctdozlite"
install -m 644 "$SCRIPT_DIR/ctdozlite.desktop" "$APP_DIR/ctdozlite.desktop"
install -m 644 "$SCRIPT_DIR/ctdozlite.png" "$ICON_DIR/ctdozlite.png"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi

echo "Đã cài đặt thành công ctdozlite v0.1.4 tại $BIN_DIR/ctdozlite"
echo "Chạy ứng dụng bằng lệnh: ctdozlite (hoặc mở từ menu ứng dụng)."
