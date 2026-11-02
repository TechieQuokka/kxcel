#!/usr/bin/env bash
# Installs the release binaries, desktop entry, and icons for the current user.
#
# On Wayland, desktops find a window's icon through its desktop entry, so this
# is what makes the icon appear in the dock and task switcher.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
bin="$HOME/.local/bin"

cargo build --release --manifest-path "$root/Cargo.toml" -p excel-app -p excel-cli

install -Dm755 "$root/target/release/kxcel" "$bin/kxcel"
install -Dm755 "$root/target/release/kxcel-cli" "$bin/kxcel-cli"
install -Dm644 "$root/packaging/kxcel.desktop" "$data/applications/kxcel.desktop"
install -Dm644 "$root/assets/icon.svg" "$data/icons/hicolor/scalable/apps/kxcel.svg"
for size in 16 32 48 64 128 256 512; do
    install -Dm644 "$root/assets/icons/kxcel-$size.png" "$data/icons/hicolor/${size}x${size}/apps/kxcel.png"
done

update-desktop-database "$data/applications" 2>/dev/null || true
gtk-update-icon-cache -f -t "$data/icons/hicolor" 2>/dev/null || true

echo "Installed kxcel and kxcel-cli to $bin"
case ":$PATH:" in
    *":$bin:"*) ;;
    *) echo "Add $bin to PATH to run them by name" ;;
esac
