#!/usr/bin/env bash
# Prepare the exact GitHub release artifacts for Butler. macOS mounts the DMG
# natively; other hosts use 7-Zip and restore the app executable's mode.
set -euo pipefail

fail() {
    echo "prepare-itch-upload: $*" >&2
    exit 1
}

if [[ $# -ne 3 ]]; then
    fail "usage: $0 <tag> <download-directory> <output-directory>"
fi

TAG="$1"
DOWNLOAD_DIR="$2"
OUTPUT_DIR="$3"

[[ "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "tag must have the form vX.Y.Z: $TAG"
[[ -d "$DOWNLOAD_DIR" ]] || fail "download directory does not exist: $DOWNLOAD_DIR"
[[ ! -e "$OUTPUT_DIR" ]] || fail "output path already exists: $OUTPUT_DIR"
command -v python3 >/dev/null || fail "python3 is required"
command -v shasum >/dev/null || fail "shasum is required"
command -v unzip >/dev/null || fail "unzip is required"
if command -v hdiutil >/dev/null && command -v ditto >/dev/null; then
    MACOS_EXTRACTOR="native"
elif command -v 7z >/dev/null; then
    MACOS_EXTRACTOR="7zip"
else
    fail "macOS DMG extraction requires hdiutil plus ditto, or 7z"
fi

DOWNLOAD_DIR="$(cd "$DOWNLOAD_DIR" && pwd)"
OUTPUT_PARENT="$(dirname "$OUTPUT_DIR")"
mkdir -p "$OUTPUT_PARENT"
OUTPUT_PARENT="$(cd "$OUTPUT_PARENT" && pwd)"
OUTPUT_DIR="$OUTPUT_PARENT/$(basename "$OUTPUT_DIR")"

PREFIX="nova-protocol_${TAG}"
WINDOWS_ARCHIVE="$DOWNLOAD_DIR/${PREFIX}_windows.zip"
MACOS_ARCHIVE="$DOWNLOAD_DIR/${PREFIX}_macOS.dmg"
LINUX_ARCHIVE="$DOWNLOAD_DIR/${PREFIX}_linux.tar.gz"
WEB_ARCHIVE="$DOWNLOAD_DIR/${PREFIX}_web.zip"
ARCHIVES=("$WINDOWS_ARCHIVE" "$MACOS_ARCHIVE" "$LINUX_ARCHIVE" "$WEB_ARCHIVE")

for archive in "${ARCHIVES[@]}"; do
    [[ -f "$archive" ]] || fail "missing release artifact: $(basename "$archive")"
done

WORK_DIR="$(mktemp -d "$OUTPUT_PARENT/.itch-upload.XXXXXX")"
MOUNT_DIR=""
MOUNTED=0
cleanup() {
    if [[ "$MOUNTED" -eq 1 ]]; then
        hdiutil detach "$MOUNT_DIR" >/dev/null 2>&1 || true
    fi
    [[ -z "$MOUNT_DIR" ]] || rm -rf "$MOUNT_DIR"
    [[ -z "$WORK_DIR" ]] || rm -rf "$WORK_DIR"
}
trap cleanup EXIT

mkdir -p "$WORK_DIR/unpacked/windows" "$WORK_DIR/unpacked/linux" "$WORK_DIR/unpacked/web"
unzip -q "$WINDOWS_ARCHIVE" -d "$WORK_DIR/unpacked/windows"
tar -xzf "$LINUX_ARCHIVE" -C "$WORK_DIR/unpacked/linux"
unzip -q "$WEB_ARCHIVE" -d "$WORK_DIR/unpacked/web"

find_one() {
    local root="$1"
    local kind="$2"
    local name="$3"
    local matches
    local count

    if [[ "$kind" == "file" ]]; then
        matches="$(find "$root" -type f -name "$name" -print)"
    else
        matches="$(find "$root" -type d -name "$name" -prune -print)"
    fi
    count="$(printf '%s\n' "$matches" | awk 'NF { count += 1 } END { print count + 0 }')"
    [[ "$count" -eq 1 ]] || fail "expected one $name under $root, found $count"
    printf '%s\n' "$matches"
}

require_payload_dirs() {
    local root="$1"
    [[ -f "$root/assets/base/base.bundle.ron" ]] || \
        fail "payload assets are missing or nested incorrectly: $root/assets"
    [[ -f "$root/credits/CREDITS.md" ]] || \
        fail "payload credits are missing or nested incorrectly: $root/credits"
}

WINDOWS_EXE="$(find_one "$WORK_DIR/unpacked/windows" file nova-protocol.exe)"
WINDOWS_ROOT="$(dirname "$WINDOWS_EXE")"
require_payload_dirs "$WINDOWS_ROOT"
mkdir "$WORK_DIR/windows"
cp -R "$WINDOWS_ROOT/." "$WORK_DIR/windows/"

LINUX_EXE="$(find_one "$WORK_DIR/unpacked/linux" file nova-protocol)"
[[ -x "$LINUX_EXE" ]] || fail "Linux executable lost its executable permission: $LINUX_EXE"
LINUX_ROOT="$(dirname "$LINUX_EXE")"
require_payload_dirs "$LINUX_ROOT"
mkdir "$WORK_DIR/linux"
cp -R "$LINUX_ROOT/." "$WORK_DIR/linux/"

WEB_INDEX="$(find_one "$WORK_DIR/unpacked/web" file index.html)"
WEB_ROOT="$(dirname "$WEB_INDEX")"
find "$WEB_ROOT" -maxdepth 1 -type f -name '*.wasm' -print -quit | grep -q . || \
    fail "web payload has no root-level wasm file: $WEB_ROOT"
require_payload_dirs "$WEB_ROOT"
mkdir "$WORK_DIR/html5"
cp -R "$WEB_ROOT/." "$WORK_DIR/html5/"

if [[ "$MACOS_EXTRACTOR" == "native" ]]; then
    MOUNT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/nova-itch-dmg.XXXXXX")"
    hdiutil attach -readonly -nobrowse -mountpoint "$MOUNT_DIR" "$MACOS_ARCHIVE" >/dev/null
    MOUNTED=1
    MACOS_APP="$(find_one "$MOUNT_DIR" directory NovaProtocol.app)"
else
    mkdir "$WORK_DIR/unpacked/macos"
    7z x -y "-o$WORK_DIR/unpacked/macos" "$MACOS_ARCHIVE" >/dev/null
    MACOS_APP="$(find_one "$WORK_DIR/unpacked/macos" directory NovaProtocol.app)"
fi

MACOS_PLIST="$MACOS_APP/Contents/Info.plist"
[[ -f "$MACOS_PLIST" ]] || fail "macOS app has no Info.plist: $MACOS_APP"
BUNDLE_EXECUTABLE="$(MACOS_PLIST="$MACOS_PLIST" python3 - <<'PY'
import os
import plistlib
from pathlib import Path

with Path(os.environ["MACOS_PLIST"]).open("rb") as source:
    value = plistlib.load(source).get("CFBundleExecutable", "")
print(value)
PY
)"
[[ "$BUNDLE_EXECUTABLE" == "nova-protocol" ]] || \
    fail "macOS app declares $BUNDLE_EXECUTABLE, expected nova-protocol"
MACOS_EXECUTABLE="$MACOS_APP/Contents/MacOS/$BUNDLE_EXECUTABLE"
[[ -f "$MACOS_EXECUTABLE" ]] || \
    fail "macOS app is missing its declared executable: Contents/MacOS/$BUNDLE_EXECUTABLE"
if [[ "$MACOS_EXTRACTOR" == "7zip" ]]; then
    find "$MACOS_APP" -type d -exec chmod 755 {} +
    find "$MACOS_APP" -type f -exec chmod 644 {} +
    chmod 755 "$MACOS_EXECUTABLE"
fi
[[ -x "$MACOS_EXECUTABLE" ]] || fail "macOS app executable has the wrong mode: $MACOS_EXECUTABLE"
require_payload_dirs "$MACOS_APP/Contents/MacOS"
mkdir -p "$WORK_DIR/osx/NovaProtocol.app"
if [[ "$MACOS_EXTRACTOR" == "native" ]]; then
    ditto "$MACOS_APP" "$WORK_DIR/osx/NovaProtocol.app"
    hdiutil detach "$MOUNT_DIR" >/dev/null
    MOUNTED=0
    rm -rf "$MOUNT_DIR"
    MOUNT_DIR=""
else
    cp -R "$MACOS_APP/." "$WORK_DIR/osx/NovaProtocol.app/"
fi

(
    cd "$DOWNLOAD_DIR"
    shasum -a 256 \
        "$(basename "$WINDOWS_ARCHIVE")" \
        "$(basename "$MACOS_ARCHIVE")" \
        "$(basename "$LINUX_ARCHIVE")" \
        "$(basename "$WEB_ARCHIVE")"
) >"$WORK_DIR/release-sha256.txt"

STAGE_ROOT="$WORK_DIR" python3 - <<'PY' >"$WORK_DIR/upload-inventory.txt"
from __future__ import annotations

import hashlib
import os
from pathlib import Path

root = Path(os.environ["STAGE_ROOT"])
for channel in ("windows", "osx", "linux", "html5"):
    channel_root = root / channel
    for path in sorted(channel_root.rglob("*")):
        relative = path.relative_to(root)
        if path.is_symlink():
            print(f"SYMLINK\t{relative}\t{os.readlink(path)}")
        elif path.is_file():
            digest = hashlib.sha256()
            with path.open("rb") as source:
                for chunk in iter(lambda: source.read(1024 * 1024), b""):
                    digest.update(chunk)
            print(f"FILE\t{relative}\t{path.stat().st_size}\t{digest.hexdigest()}")
PY

rm -rf "$WORK_DIR/unpacked"
mv "$WORK_DIR" "$OUTPUT_DIR"
WORK_DIR=""
trap - EXIT

echo "Prepared Butler payloads in $OUTPUT_DIR"
