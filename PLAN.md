# Organiser — Plan

Desktop app for planning and fast overview of open tasks. This document covers
the **first page**: the unified inbox of assigned issues + my open PRs across
GitHub and GitLab, each with live state.

Note: "Git hook" in the request means a **connector/integration** that talks to
the GitHub/GitLab APIs — not a `.git/hooks` script. Overview tools poll the
provider REST/GraphQL APIs; there is no push mechanism from the host to a
desktop app.

## Stack

- **Shell:** Tauri (Rust core + system webview). Small binary, secrets stay in Rust.
- **UI:** React + TypeScript + Vite.
- **Secrets:** OS keychain via `keyring` crate (Rust side). Tokens never touch the webview or disk in plaintext.
- **HTTP:** `reqwest` in Rust. All provider calls happen in Rust commands, not the browser — avoids CORS, keeps tokens server-side of the IPC boundary.

Why Rust does the fetching: the token stays in the Rust process, the webview only
ever receives already-shaped task data.

## Providers & auth

Three host kinds, one config list. Each account = `{ id, kind, base_url, label, auth }`.
**Hosts are user-configured at runtime and persisted** — add an account once and it's
there on every launch. The account list (`{ id, kind, base_url, label }` — **no token**)
is saved to a JSON file in the Tauri app-config dir; the token goes to the OS keychain
under `organiser:{account_id}`. On launch, read the list, look up each token by id.
github.tools.sap is just an example GHE base_url, not a built-in.

| Kind | base_url example | API |
|------|------------------|-----|
| github.com | api.github.com (fixed for this kind) | GitHub REST v3 / GraphQL v4 |
| github-enterprise | github.tools.sap/api/v3 (user-entered) | same, custom base |
| gitlab (self-hosted or .com) | gitlab.example.com/api/v4 (user-entered) | GitLab REST v4 |

Auth per account: **PAT only for v1**. Paste token, stored in keychain under
`organiser:{account_id}`. OAuth device flow is deferred (needs a registered client_id
per host) — same keychain storage when it lands. `// ponytail: PAT ships first; add device flow per-host once a client_id exists.`

## The one screen (first page)

A single list, "My Work", two sections:

1. **Assigned Issues** — issues where I'm the assignee, state = open. (GitHub, GHE, GitLab)
2. **My Pull/Merge Requests** — PRs/MRs I authored, open, with **review state**.
3. **Project items assigned to me** — GitHub only. Items on Projects v2 boards whose Assignees field includes me, **including board-only draft items** (not yet real issues).

Each row: `[provider icon] title · repo/project · #number · state-badge · updated-ago`.
Grouped by account, filterable by provider, sortable by updated. Click → opens in browser.

A project item that *is* a real issue already surfaced via section 1 is de-duplicated
by `id` — shown once, tagged with its project. Draft project items have no repo/number
and their link opens the board.

### State model (the important part)

Normalise every provider's status into one enum the UI renders as a badge:

```
Open | Draft | InProgress | ChangesRequested | Approved | Merged | Closed
```

Derivation:

**GitHub PR** — from `reviewDecision` (GraphQL) + `isDraft` + `state`:
- draft → Draft
- REVIEW_REQUIRED / no decision → Open
- CHANGES_REQUESTED → ChangesRequested
- APPROVED → Approved
- MERGED → Merged, CLOSED → Closed

**GitLab MR** — no single field; derive:
- `draft: true` → Draft
- `merge_status` / `state == merged` → Merged
- approvals: query `/approvals` → if `approved: true` → Approved
- open reviewers with unresolved threads / requested changes → ChangesRequested
- else → Open
- `// ponytail: GitLab needs 2 calls (MR + approvals). Batch later if the list is slow; single-user volume won't be.`

**Issues** — Open / Closed (+ optional `InProgress` if a label like `in progress` / `doing` is present, configurable). Keep it dumb first.

**GitHub project items** — a project item's state is its board **Status** field (e.g. `Todo` / `In Progress` / `Done`), which is free-form per board. Map by name where obvious (`in progress`→InProgress, `done`→Closed) and fall back to Open. Draft items likewise. `// ponytail: Status column names are per-board; ship the obvious mapping, make it configurable only if boards disagree.`

## Fetching

Per account, on demand + on a timer (default 5 min, `setInterval` in UI calling a Rust command; no background daemon).

- GitHub: one GraphQL query per account gets assigned issues + authored PRs + reviewDecision in a single round trip. Prefer GraphQL over `/search` to get review state without N+1.
- GitHub project items: GraphQL `viewer` → `projectsV2` → `items`, reading each item's `content` (Issue/PR/DraftIssue), its Assignees, and its Status field value. Filter to items where I'm an assignee. `// ponytail: page items lazily; a single user's boards are small.`
- GitHub Enterprise: same queries, different base_url.
- GitLab: REST — `GET /issues?assignee_username=me&state=opened`, `GET /merge_requests?author_id=me&state=opened`, then `/approvals` per MR for approved-state. (No project-board equivalent in scope.)

Rust exposes:
```
list_accounts() -> [Account]           # from config file, no secrets
add_account(kind, base_url, label, token)   # write config file + keychain
remove_account(id)                     # drop from config file + keychain
fetch_work(account_id) -> WorkList      # issues + prs, normalised
fetch_all() -> [WorkList]               # fan out, join
```

Errors surface per-account (bad token, host unreachable) as a row-level banner, not a global failure — one dead account doesn't blank the screen.

## Data shapes (TS side)

```ts
type TaskState = 'open'|'draft'|'in_progress'|'changes_requested'|'approved'|'merged'|'closed'
type WorkItem = {
  id: string; kind: 'issue'|'pr'|'project_item';
  title: string; url: string; updatedAt: string;
  state: TaskState; account: string;
  repo?: string; number?: number;   // absent for board-only draft items
  project?: string;                 // set when it came from a project board
}
```

No local DB for v1 — hold in memory, refetch on launch. `// ponytail: cache to disk only when offline-open becomes a real need.`

## Build order

1. Scaffold Tauri + React + TS (`npm create tauri-app`).
2. Account config UI + **persisted account list (JSON in app-config dir)** + keychain add/remove/list (PAT only). Accounts survive restart; verify by relaunching.
3. GitHub GraphQL fetch (assigned issues + authored PRs + **project items assigned to me, incl. drafts**) → normalised WorkList, de-duplicated by id → render the list with badges.
4. GitHub Enterprise (same code, user-entered base_url) — verify against github.tools.sap.
5. GitLab REST fetch + approvals → same WorkList.
6. Timer refresh, per-account error rows, provider filter/sort.
7. (Later) OAuth device flow, disk cache, second page.

## Explicitly skipped for v1

- Real `.git` hooks — not how host→client updates work. Add webhooks only if a server component ever exists.
- Background daemon / native notifications — polling while the window is open covers overview. Add when you want alerts with the app closed.
- Local database — in-memory. Add on offline need.
- Writing back (closing issues, approving) — read-only overview first.
