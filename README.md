# Organiser

A desktop app for tracking your open issues, pull requests, and review requests across GitHub, GitHub Enterprise, and GitLab — plus live visibility into running Claude Code agent sessions.

Built with [Tauri 2](https://tauri.app) + React + TypeScript.

## Features

- **Agents** — live status of Claude Code sessions (working / waiting / done), fed via hooks
- **Pull Requests** — your open PRs with CI status, approval state, and row colouring
- **Issues** — assigned issues and project board items
- **Review Requests** — PRs waiting for your review, filtered to exclude bots
- Per-account org filtering and GHE project board selection
- macOS native notifications on PR state changes
- Tokens stored in the OS keychain — nothing sensitive on disk

## Prerequisites

| Tool | Version |
|------|---------|
| [Rust](https://rustup.rs) | stable (1.77+) |
| Node.js | 20+ |
| npm | 10+ |
| [Tauri CLI prerequisites](https://tauri.app/start/prerequisites/) | macOS: Xcode command-line tools |

## Setup

### Quick install (macOS)

```bash
./install.sh
```

Checks for Rust/Node, installs dependencies, builds, and copies the app to `/Applications`.

### Manual

```bash
# 1. Install dependencies
npm install

# 2. Run in development mode (hot-reload frontend, Rust recompiles on change)
npm run tauri dev

# 3. Build a release bundle
npm run tauri build
```

The first `tauri dev` or `tauri build` will compile the Rust backend — this takes a few minutes on a fresh checkout.

## Adding accounts

1. Open the app and click **Settings** in the sidebar.
2. Click **Add account** and choose the provider type:
   - **GitHub** — personal access token with `repo`, `read:org`, `read:project` scopes
   - **GitHub Enterprise** — same scopes, plus your GHE base URL (e.g. `https://github.example.com`)
   - **GitLab** — personal access token with `read_api` scope, plus your GitLab base URL
3. Tokens are stored in the macOS Keychain — they are never written to disk or committed to the repo.

## Claude Code agent hooks

The app automatically registers the required hooks in `~/.claude/settings.json` on first launch. These hooks POST session events to a local server (`127.0.0.1:27384`) so the Agents page can track session state.

If you prefer to add them manually, the hooks point to:

```
curl -sf -X POST http://127.0.0.1:27384/hook \
  -H 'Content-Type: application/json' -d @-
```

Events used: `PostToolUse`, `SessionStart`, `Stop`, `SessionEnd`.

## Data storage

| What | Where |
|------|-------|
| Account metadata (label, URL, org filters) | `~/Library/Application Support/com.organiser.app/` |
| Tokens | macOS Keychain |
| Claude hook config | `~/.claude/settings.json` (merged, not replaced) |

Nothing sensitive is stored in the repository.

## macOS SDK note

If you hit build errors related to the macOS 26 SDK, the Rust build is pinned to the macOS 15.4 SDK in `src-tauri/.cargo/config.toml`. Ensure Xcode 15.x command-line tools are installed, or update the `SDKROOT` path in that file to match your local SDK.
