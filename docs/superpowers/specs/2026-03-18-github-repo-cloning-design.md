# GitHub Repo Cloning — Design Spec

## Overview

Standalone tool accessible from the home screen that lets users authenticate with GitHub via OAuth device flow, browse their repos, select repos with per-repo destination folders, clone them, and optionally run detected bootstrap scripts.

## Motivation

A provisioning tool should set up more than just packages — getting your repos cloned to the right places is part of "my machine is ready." Dotfiles repos, project repos, and config repos all need to land in specific directories. This feature turns provision into a complete dev environment bootstrapper.

## Approach

Minimal clone-only tool (Approach A). Token lives in memory only — no persistence. The feature is designed for one-time use during machine setup.

## Authentication: GitHub Device Flow

1. App sends `POST https://github.com/login/device/code` with a registered client ID and `scope=repo`
2. GitHub returns a `device_code`, `user_code`, `verification_uri`, and polling `interval`
3. Screen displays: "Go to **github.com/login/device** and enter code: **XXXX-XXXX**" with a button that opens the URL in the default browser
4. App polls `POST https://github.com/login/oauth/access_token` every `interval` seconds (minimum 5s) until the user completes auth or the code expires
5. On success, store the access token in `App` state (not persisted to disk)
6. Transition to the repo list screen

The client ID is a public value from a GitHub OAuth App registered for this project. Safe to embed in the binary.

## Screen Flow

Three new `Screen` variants:

### `GitHubLogin`

- Displays the device code prominently and the verification URL
- "Open GitHub" button opens `verification_uri` in the default browser via `open::that()`
- Spinner animation while polling for auth completion
- Error state if the code expires (offer to retry)
- Back button returns to home (`ProfileSelect`)

### `GitHubRepos`

- Fetches repos from `GET https://api.github.com/user/repos?per_page=100&sort=updated` on entry (paginate if user has >100 repos)
- Search/filter bar at top (reuses `self.search` / `self.search_lower`)
- Each repo row shows:
  - Repo name (bold)
  - Public/private badge
  - Description (truncated, muted text)
  - "Select folder" button — opens `rfd::AsyncFileDialog` folder picker
- After folder is picked, repo appears in a **clone queue** section at the bottom of the screen
  - Each queue item shows: repo name + destination path + remove button
- Footer: "Clone all (N)" button when queue is non-empty
- Back button returns to home (discards token)

### `GitHubCloning`

- Reuses `view_progress_screen` pattern with `ProgressState`
- Shows clone progress for each repo in the queue sequentially
- Cloning via `git clone <clone_url> <destination>` using `tokio::process::Command` with `CREATE_NO_WINDOW`
- After each successful clone, scans repo root for bootstrap scripts
- If bootstrap script(s) found: shows prompt "Run `bootstrap.ps1`?" with yes/skip buttons before proceeding to next repo
- Bootstrap script runs via `tokio::process::Command` in the cloned directory
- Done button returns to repo list screen

## Data Model

### Structs (in `src/github.rs`)

```rust
pub struct GitHubRepo {
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub private: bool,
    pub clone_url: String,
    pub html_url: String,
    // precomputed for search filtering
    pub name_lower: String,
    pub desc_lower: String,
}

pub struct CloneItem {
    pub repo: GitHubRepo,
    pub destination: PathBuf,
}

pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}
```

### Progress enums

```rust
pub enum DeviceFlowProgress {
    CodeReady {
        user_code: String,
        verification_uri: String,
    },
    Polling,
    Authenticated {
        token: String,
    },
    Failed {
        error: String,
    },
}

pub enum CloneProgress {
    // Mirrors InstallProgress pattern
    Log { line: String },
    Activity { line: String },
    PackageDone { index: usize, success: bool },
    BootstrapDetected { index: usize, script: String },
    AllDone,
}
```

## API Calls

All HTTP via `reqwest` (rustls). No `json` feature — use `.text().await` + `serde_json::from_str()`.

| Endpoint | Method | Purpose |
|---|---|---|
| `https://github.com/login/device/code` | POST | Request device code |
| `https://github.com/login/oauth/access_token` | POST | Poll for access token |
| `https://api.github.com/user/repos?per_page=100&sort=updated` | GET | List user's repos |

Pagination: follow `Link` header `rel="next"` if present, up to a reasonable cap (500 repos).

## Cloning

- Shell out to `git clone` via `tokio::process::Command` with `.creation_flags(CREATE_NO_WINDOW)`
- Use `--progress` flag so git writes progress to stderr
- Read stderr for progress lines, stdout for errors
- Use `.stderr(Stdio::piped())` and read raw bytes (same pattern as winget process reading in `install.rs`)

## Bootstrap Script Detection

After each successful clone, scan the repo root directory for these filenames (in order):

1. `bootstrap.ps1`
2. `setup.ps1`
3. `install.ps1`
4. `bootstrap.sh`
5. `setup.sh`
6. `install.sh`
7. `Makefile`

If exactly one is found, prompt "Run `<script>`?". If multiple, let the user pick from a list. If none, proceed silently to the next clone.

Scripts run via `tokio::process::Command` in the cloned repo's directory with `CREATE_NO_WINDOW`.

## New File

`src/github.rs` — contains all GitHub-specific types, API calls, device flow stream, clone stream, and bootstrap detection. Follows the same module pattern as `upgrade.rs` and `uninstall.rs`.

## Message Variants

```
GoToGitHubLogin
GitHubDeviceFlowProgress(DeviceFlowProgress)
GitHubReposFetched(Vec<GitHubRepo>)
GitHubSelectFolder(String)          // repo full_name
GitHubFolderPicked(String, PathBuf) // repo full_name, destination
GitHubRemoveFromQueue(String)       // repo full_name
StartGitHubClone
CancelGitHubClone
GitHubCloneProgress(CloneProgress)
GitHubRunBootstrap(usize)           // clone index
GitHubSkipBootstrap
FinishGitHubClone
```

## Home Screen Entry

New `action_card` on `ProfileSelect` screen with `Icon::Github` (from lucide-icons) and label "Clone repos". Positioned after the search card.

## Dependencies

- `open` crate — for opening the verification URL in the default browser. Lightweight, no heavy deps.
- No new major dependencies. `reqwest`, `tokio`, `serde_json`, `rfd` are already in use.

## Out of Scope

- Persisting the OAuth token across sessions
- Cloning from non-GitHub sources (GitLab, Bitbucket)
- SSH clone URLs (HTTPS only, token provides auth)
- Repo creation or push operations
- Org repo browsing (only user's own repos via `/user/repos`)
