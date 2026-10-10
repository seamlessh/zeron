#!/usr/bin/env bash
# Build and run an isolated macOS development bundle of the Seamlessh Zeron
# candidate. It must never share identity, data, ports, accounts or network
# services with a real Zeron or Seamlessh installation, so every isolation
# setting is fixed here rather than taken from the caller's environment.
# Set ZERON_DEV_BUILD_ONLY=1 to prepare the signed bundle without launching it.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
command -v cargo >/dev/null 2>&1 || PATH="$HOME/.cargo/bin:$PATH"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
DEV_ROOT="$ROOT/target/macos-dev"
APP="$DEV_ROOT/Zeron Candidate.app"
CONTENTS="$APP/Contents"

# A caller-chosen bundle id could collide with real Zeron TCC and
# LaunchServices state, so the override is ignored.
BUNDLE_ID="app.seamlessh.zeron-candidate"
if [[ -n "${ZERON_DEV_BUNDLE_ID:-}" ]]; then
  echo "warning: ignoring ZERON_DEV_BUNDLE_ID; the candidate always uses $BUNDLE_ID" >&2
fi

die() {
  echo "error: $*" >&2
  exit 1
}

# realpath of a path that may not exist yet, so symlinks cannot smuggle the
# data directory into a real installation's directory.
resolve() {
  python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
}

REAL_HOME="$(resolve "$HOME")"
DATA_DIR="$(resolve "${ZERON_DEV_DATA_DIR:-$DEV_ROOT/data}")"
for protected in "$REAL_HOME/.zeron" "$REAL_HOME/.seamlessh"; do
  case "$DATA_DIR/" in
    "$protected"/*) die "data directory $DATA_DIR is inside $protected" ;;
  esac
done

# 27654 is Zeron's default engine port, 49777 upstream's dev port and 27641
# its OAuth callback port. Binding any of them could attach to, or be
# reached by, a real installation.
IPC_PORT="${ZERON_DEV_IPC_PORT:-47621}"
CALLBACK_PORT=47622
case "$IPC_PORT" in
  27654 | 49777 | 27641 | "$CALLBACK_PORT") die "IPC port $IPC_PORT is reserved" ;;
esac
[[ "$IPC_PORT" =~ ^[0-9]+$ ]] || die "IPC port $IPC_PORT is not a number"

# Synthetic HOME and a system-only PATH keep the engine from finding the
# developer's real agent CLIs, credentials and shell configuration.
DEV_HOME="$DEV_ROOT/home"
DEV_PATH="/usr/bin:/bin:/usr/sbin:/sbin"

# Port 9 (discard) on loopback is closed, so edge rooms, sync and release
# checks fail locally instead of reaching edge.zeron.sh. An empty WorkOS
# client id with no edge token selects development auth.
APP_ENV=(
  "ZERON_DATA_DIR=$DATA_DIR"
  "ZERON_IPC_PORT=$IPC_PORT"
  "ZERON_CALLBACK_PORT=$CALLBACK_PORT"
  "ZERON_EDGE_URL=http://127.0.0.1:9"
  "ZERON_AUTO_UPDATE=0"
  "ZERON_WORKOS_CLIENT_ID="
  "ZERON_NO_LOGIN_SHELL=1"
  "HOME=$DEV_HOME"
  "PATH=$DEV_PATH"
)

if pgrep -f -x "$CONTENTS/MacOS/zeron" >/dev/null 2>&1; then
  die "Zeron Candidate is already running. Quit it before rebuilding the signed bundle."
fi

cd "$ROOT"
cargo build --locked -p zeron

mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources" "$DATA_DIR" "$DEV_HOME"

# Without this file the engine checks npm, GitHub and Homebrew for agent CLI
# releases at boot. Every policy is off, so nothing is probed or installed.
python3 - "$DATA_DIR/harness-update-prefs.json" <<'PY'
import json, sys
harnesses = ["claude-code", "codex", "cursor", "devin", "grok", "hermes",
             "pi", "opencode", "antigravity"]
with open(sys.argv[1], "w") as f:
    json.dump({"policies": {h: "off" for h in harnesses}}, f, indent=2)
PY

TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
install -m 755 "$TARGET_DIR/debug/zeron" "$CONTENTS/MacOS/zeron"
PLIST="$CONTENTS/Info.plist"
sed "s/__VERSION__/$VERSION/g" "$ROOT/dist/macos/Info-dev.plist" >"$PLIST"
plutil -replace CFBundleIdentifier -string "$BUNDLE_ID" "$PLIST"
plutil -replace CFBundleName -string "Zeron Candidate" "$PLIST"
plutil -replace CFBundleDisplayName -string "Zeron Candidate" "$PLIST"
# The bundle carries the same environment as the launch, so macOS privacy's
# "Quit & Reopen" restarts the isolated instance rather than a default one.
plutil -replace LSEnvironment -json '{}' "$PLIST"
for pair in "${APP_ENV[@]}"; do
  plutil -replace "LSEnvironment.${pair%%=*}" -string "${pair#*=}" "$PLIST"
done

for forbidden in edge.zeron.sh api.workos.com client_01KWD0EAKZKD50YCQJNYSRE4BY; do
  if grep -q "$forbidden" "$PLIST"; then
    die "bundle plist references production endpoint or id $forbidden"
  fi
done

mkdir -p "$CONTENTS/Resources/licenses"
cp "$ROOT/crates/voice/NOTICE.md" "$CONTENTS/Resources/licenses"/parakeet-v3.txt

if [[ ! -f "$CONTENTS/Resources/zeron.icns" ]]; then
  ICONSET="$DEV_ROOT/zeron-dev.iconset"
  mkdir -p "$ICONSET"
  for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$ROOT/dist/macos/icon-1024.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
    retina=$((size * 2))
    sips -z "$retina" "$retina" "$ROOT/dist/macos/icon-1024.png" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
  done
  iconutil -c icns "$ICONSET" -o "$CONTENTS/Resources/zeron.icns"
fi

# A real Apple Development identity gives TCC a stable signing requirement
# across rebuilds. Set ZERON_DEV_CODESIGN_IDENTITY explicitly when more than
# one identity is installed; otherwise fall back to an ad-hoc signature.
IDENTITY="${ZERON_DEV_CODESIGN_IDENTITY:-}"
if [[ -z "$IDENTITY" ]]; then
  IDENTITY="$(security find-identity -v -p codesigning 2>/dev/null | sed -n 's/.*"\(Apple Development:[^"]*\)".*/\1/p' | head -1)"
fi
if [[ -n "$IDENTITY" ]]; then
  codesign --entitlements "$ROOT/dist/macos/Dictation.entitlements" --force --sign "$IDENTITY" --identifier "$BUNDLE_ID" "$APP"
else
  codesign --entitlements "$ROOT/dist/macos/Dictation.entitlements" --force --sign - --identifier "$BUNDLE_ID" "$APP"
  echo "warning: no Apple Development signing identity found; macOS may ask for permissions again after a rebuild" >&2
fi

if [[ "${ZERON_DEV_BUILD_ONLY:-0}" == "1" ]]; then
  echo "built: $APP" >&2
  exit 0
fi

echo "running Zeron Candidate (bundle $BUNDLE_ID, data $DATA_DIR, IPC $IPC_PORT)" >&2
# LaunchServices must own the process. Launching Contents/MacOS/zeron directly
# makes TCC attribute Screen Recording to the terminal (Warp, Terminal, etc.).
# -W keeps the script attached until the app exits. Runtime logs remain in the
# isolated data directory (`$DATA_DIR/logs/zeron-headed.log`).
OPEN_ENV=()
for pair in "${APP_ENV[@]}"; do
  OPEN_ENV+=(--env "$pair")
done
if [[ -n "${ZERON_OPEN_ROUTE:-}" ]]; then
  OPEN_ENV+=(--env "ZERON_OPEN_ROUTE=$ZERON_OPEN_ROUTE")
fi
exec open -W "${OPEN_ENV[@]}" "$APP" --args "$@"
