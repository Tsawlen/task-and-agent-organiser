#!/usr/bin/env bash
set -euo pipefail

APP_NAME="organiser"
REPO_DIR="$(cd "$(dirname "$0")" && pwd)"

green()  { printf '\033[0;32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[0;33m%s\033[0m\n' "$*"; }
red()    { printf '\033[0;31m%s\033[0m\n' "$*"; exit 1; }
step()   { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }

# ── Rust ────────────────────────────────────────────────────────────────────
step "Checking Rust"
if ! command -v rustup &>/dev/null; then
  yellow "rustup not found — installing..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
  source "$HOME/.cargo/env"
else
  green "rustup $(rustup --version 2>&1 | head -1)"
fi

export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v cargo &>/dev/null; then
  red "cargo not found after rustup install — open a new shell and re-run."
fi
green "cargo $(cargo --version)"

# ── Node.js ──────────────────────────────────────────────────────────────────
step "Checking Node.js"
if ! command -v node &>/dev/null; then
  red "Node.js not found. Install it from https://nodejs.org (v20+) and re-run."
fi
NODE_MAJOR=$(node -e 'process.stdout.write(String(process.versions.node.split(".")[0]))')
if [ "$NODE_MAJOR" -lt 20 ]; then
  red "Node.js $NODE_MAJOR found but v20+ is required."
fi
green "node $(node --version)"

# ── Xcode CLI tools (macOS) ──────────────────────────────────────────────────
step "Checking Xcode command-line tools"
if ! xcode-select -p &>/dev/null; then
  yellow "Installing Xcode command-line tools..."
  xcode-select --install
  echo "Re-run this script after the Xcode tools installation completes."
  exit 0
fi
green "Xcode CLT: $(xcode-select -p)"

# ── npm install ───────────────────────────────────────────────────────────────
step "Installing npm dependencies"
cd "$REPO_DIR"
npm install

# ── Build ─────────────────────────────────────────────────────────────────────
step "Building $APP_NAME (this takes a few minutes on first run)"
npm run tauri build

# ── Install to /Applications ──────────────────────────────────────────────────
step "Installing to /Applications"
BUNDLE_DIR="$REPO_DIR/src-tauri/target/release/bundle/macos"
APP_BUNDLE=$(find "$BUNDLE_DIR" -maxdepth 1 -name "*.app" | head -1)

if [ -z "$APP_BUNDLE" ]; then
  red "Could not find .app bundle in $BUNDLE_DIR"
fi

DEST="/Applications/$(basename "$APP_BUNDLE")"
if [ -d "$DEST" ]; then
  yellow "Removing existing $DEST"
  rm -rf "$DEST"
fi

cp -r "$APP_BUNDLE" /Applications/
green "Installed: $DEST"

printf '\n\033[1;32m✓ Done! Launch %s from /Applications or Spotlight.\033[0m\n' "$APP_NAME"
