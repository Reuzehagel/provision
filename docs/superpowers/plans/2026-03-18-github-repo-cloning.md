# GitHub Repo Cloning Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a standalone GitHub repo cloning tool accessible from the home screen — authenticate via OAuth device flow, browse repos, select destinations, clone, and optionally run detected bootstrap scripts.

**Architecture:** New `src/github.rs` module with all GitHub types, API calls, and streaming functions. Four new `Screen` variants (`GitHubLogin`, `GitHubRepos`, `GitHubCloning`, `GitHubBootstrap`) with view methods in `views.rs` and handlers in `main.rs`. Clone stream emits `InstallProgress` events to reuse the existing `ProgressState` and `view_progress_screen` infrastructure. New `open` crate dependency for launching the browser.

**Tech Stack:** Rust, Iced 0.14, tokio, reqwest, serde_json, rfd, open, git CLI

**Spec:** `docs/superpowers/specs/2026-03-18-github-repo-cloning-design.md`

---

## Chunk 1: Dependencies & GitHub Module (Types + API)

### Task 1: Add `open` crate dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add `open` to dependencies**

In `Cargo.toml`, add after the `sysinfo` line (line 20):

```toml
open = "5"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: success

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "Add open crate for launching browser URLs"
```

### Task 2: Create `src/github.rs` with types and serde structs

**Files:**
- Create: `src/github.rs`
- Modify: `src/main.rs` (add `mod github;` at line 1)

- [ ] **Step 1: Create `src/github.rs` with all types**

```rust
use std::path::PathBuf;

use serde::Deserialize;

use crate::install;

/// Client ID from the registered GitHub OAuth App for provision.
/// This is a public value — safe to embed in the binary.
const GITHUB_CLIENT_ID: &str = "PLACEHOLDER_CLIENT_ID";

// ── Serde structs for GitHub API responses ──────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct DeviceCodeResponse {
    pub(crate) device_code: String,
    pub(crate) user_code: String,
    pub(crate) verification_uri: String,
    pub(crate) interval: u64,
    #[allow(dead_code)]
    pub(crate) expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenResponse {
    pub(crate) access_token: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoResponse {
    name: String,
    full_name: String,
    description: Option<String>,
    private: bool,
    clone_url: String,
    html_url: String,
}

// ── App-facing types ────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct GitHubRepo {
    pub(crate) name: String,
    pub(crate) full_name: String,
    pub(crate) description: Option<String>,
    pub(crate) private: bool,
    pub(crate) clone_url: String,
    #[allow(dead_code)]
    pub(crate) html_url: String,
    // Precomputed for search filtering
    pub(crate) name_lower: String,
    pub(crate) desc_lower: String,
}

impl From<RepoResponse> for GitHubRepo {
    fn from(r: RepoResponse) -> Self {
        let name_lower = r.name.to_lowercase();
        let desc_lower = r
            .description
            .as_deref()
            .unwrap_or("")
            .to_lowercase();
        Self {
            name: r.name,
            full_name: r.full_name,
            description: r.description,
            private: r.private,
            clone_url: r.clone_url,
            html_url: r.html_url,
            name_lower,
            desc_lower,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CloneItem {
    pub(crate) repo: GitHubRepo,
    pub(crate) destination: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct BootstrapItem {
    pub(crate) repo_name: String,
    pub(crate) repo_path: PathBuf,
    pub(crate) scripts: Vec<String>,
    pub(crate) status: BootstrapStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BootstrapStatus {
    Pending,
    Running,
    Done,
    Skipped,
    Failed(String),
}

// ── Progress enums ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) enum DeviceFlowProgress {
    CodeReady {
        user_code: String,
        verification_uri: String,
    },
    Authenticated {
        #[allow(dead_code)]
        token: String,
    },
    Failed {
        #[allow(dead_code)]
        error: String,
    },
}

/// Bootstrap script names to detect, in priority order.
const BOOTSTRAP_SCRIPTS: &[&str] = &[
    "bootstrap.ps1",
    "setup.ps1",
    "install.ps1",
    "bootstrap.sh",
    "setup.sh",
    "install.sh",
    "Makefile",
];
```

- [ ] **Step 2: Add `mod github;` to `main.rs`**

In `src/main.rs`, add after `mod catalog;` (around line 1, in the module declarations):

```rust
mod github;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: success (types are unused but that's fine — all are `pub(crate)`)

- [ ] **Step 4: Commit**

```bash
git add src/github.rs src/main.rs
git commit -m "Add github module with types for device flow, repos, cloning, and bootstrap"
```

### Task 3: Add API functions and streams to `src/github.rs`

**Files:**
- Modify: `src/github.rs`

- [ ] **Step 1: Add the device flow stream function**

Append to `src/github.rs`:

```rust
use iced::futures;
use iced::futures::SinkExt as _;
use iced::stream;

type Sender = futures::channel::mpsc::Sender<DeviceFlowProgress>;

/// Run the GitHub OAuth device flow. Emits CodeReady, then polls until Authenticated or Failed.
pub(crate) fn device_flow(
    dry_run: bool,
) -> impl futures::Stream<Item = DeviceFlowProgress> + Send {
    stream::channel(10, move |mut sender: Sender| async move {
        if dry_run {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let _ = sender
                .send(DeviceFlowProgress::CodeReady {
                    user_code: "FAKE-CODE".into(),
                    verification_uri: "https://github.com/login/device".into(),
                })
                .await;

            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let _ = sender
                .send(DeviceFlowProgress::Authenticated {
                    token: "fake-token-for-dry-run".into(),
                })
                .await;
            return;
        }

        // 1. Request device code
        let client = reqwest::Client::new();
        let res = client
            .post("https://github.com/login/device/code")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", GITHUB_CLIENT_ID),
                ("scope", "repo"),
            ])
            .send()
            .await;

        let body = match res {
            Ok(r) => match r.text().await {
                Ok(t) => t,
                Err(e) => {
                    let _ = sender
                        .send(DeviceFlowProgress::Failed {
                            error: format!("Read error: {e}"),
                        })
                        .await;
                    return;
                }
            },
            Err(e) => {
                let _ = sender
                    .send(DeviceFlowProgress::Failed {
                        error: format!("Request failed: {e}"),
                    })
                    .await;
                return;
            }
        };

        let device: DeviceCodeResponse = match serde_json::from_str(&body) {
            Ok(d) => d,
            Err(e) => {
                let _ = sender
                    .send(DeviceFlowProgress::Failed {
                        error: format!("Parse error: {e}"),
                    })
                    .await;
                return;
            }
        };

        let _ = sender
            .send(DeviceFlowProgress::CodeReady {
                user_code: device.user_code.clone(),
                verification_uri: device.verification_uri.clone(),
            })
            .await;

        // 2. Poll for token
        let interval = std::cmp::max(device.interval, 5);
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

            let poll_res = client
                .post("https://github.com/login/oauth/access_token")
                .header("Accept", "application/json")
                .form(&[
                    ("client_id", GITHUB_CLIENT_ID),
                    ("device_code", device.device_code.as_str()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ])
                .send()
                .await;

            let poll_body = match poll_res {
                Ok(r) => match r.text().await {
                    Ok(t) => t,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            let token_resp: TokenResponse = match serde_json::from_str(&poll_body) {
                Ok(t) => t,
                Err(_) => continue,
            };

            if let Some(token) = token_resp.access_token {
                let _ = sender
                    .send(DeviceFlowProgress::Authenticated { token })
                    .await;
                return;
            }

            match token_resp.error.as_deref() {
                Some("authorization_pending") => continue,
                Some("slow_down") => {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
                Some(e) => {
                    let _ = sender
                        .send(DeviceFlowProgress::Failed {
                            error: e.to_string(),
                        })
                        .await;
                    return;
                }
                None => continue,
            }
        }
    })
}
```

- [ ] **Step 2: Add the fetch repos function**

Append to `src/github.rs`:

```rust
/// Fetch all repos for the authenticated user.
pub(crate) async fn fetch_repos(
    token: &str,
    dry_run: bool,
) -> Result<Vec<GitHubRepo>, String> {
    if dry_run {
        return Ok(vec![
            GitHubRepo {
                name: "dotfiles".into(),
                full_name: "user/dotfiles".into(),
                description: Some("My dotfiles and config".into()),
                private: false,
                clone_url: "https://github.com/user/dotfiles.git".into(),
                html_url: "https://github.com/user/dotfiles".into(),
                name_lower: "dotfiles".into(),
                desc_lower: "my dotfiles and config".into(),
            },
            GitHubRepo {
                name: "my-project".into(),
                full_name: "user/my-project".into(),
                description: Some("A cool project".into()),
                private: true,
                clone_url: "https://github.com/user/my-project.git".into(),
                html_url: "https://github.com/user/my-project".into(),
                name_lower: "my-project".into(),
                desc_lower: "a cool project".into(),
            },
            GitHubRepo {
                name: "provision".into(),
                full_name: "user/provision".into(),
                description: Some("Windows provisioning tool".into()),
                private: false,
                clone_url: "https://github.com/user/provision.git".into(),
                html_url: "https://github.com/user/provision".into(),
                name_lower: "provision".into(),
                desc_lower: "windows provisioning tool".into(),
            },
        ]);
    }

    let client = reqwest::Client::new();
    let mut all_repos = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!(
            "https://api.github.com/user/repos?per_page=100&sort=updated&page={page}"
        );
        let res = client
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "provision")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let body = res.text().await.map_err(|e| format!("Read error: {e}"))?;
        let batch: Vec<RepoResponse> =
            serde_json::from_str(&body).map_err(|e| format!("Parse error: {e}"))?;

        let count = batch.len();
        all_repos.extend(batch.into_iter().map(GitHubRepo::from));

        if count < 100 || all_repos.len() >= 500 {
            break;
        }
        page += 1;
    }

    Ok(all_repos)
}
```

- [ ] **Step 3: Add the clone stream function**

Append to `src/github.rs`:

```rust
use std::os::windows::process::CommandExt;
use tokio::process::Command;

/// Clone a batch of repos as a stream, emitting InstallProgress events
/// so the existing ProgressState infrastructure can be reused.
pub(crate) fn clone_all(
    items: Vec<CloneItem>,
    token: String,
    dry_run: bool,
) -> impl futures::Stream<Item = install::InstallProgress> + Send {
    stream::channel(
        100,
        move |mut sender: futures::channel::mpsc::Sender<install::InstallProgress>| async move {
            for (i, item) in items.iter().enumerate() {
                let _ = sender
                    .send(install::InstallProgress::Started { index: i })
                    .await;

                if dry_run {
                    let _ = sender
                        .send(install::InstallProgress::Log {
                            index: i,
                            line: format!(
                                "[DRY RUN] git clone {} → {}",
                                item.repo.full_name,
                                item.destination.display()
                            ),
                        })
                        .await;
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                    let _ = sender
                        .send(install::InstallProgress::Succeeded { index: i })
                        .await;
                    continue;
                }

                // Inject token into clone URL for private repo access
                let auth_url = item
                    .repo
                    .clone_url
                    .replace("https://", &format!("https://oauth2:{token}@"));

                let dest = item.destination.to_string_lossy().to_string();

                let result = Command::new("git")
                    .args(["clone", "--progress", &auth_url, &dest])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .creation_flags(install::CREATE_NO_WINDOW)
                    .spawn();

                match result {
                    Ok(mut child) => {
                        // git writes progress to stderr
                        if let Some(stderr) = child.stderr.take() {
                            let _ = install::read_stdout(stderr, &mut sender, |event| {
                                match event {
                                    install::LineEvent::Log(line) => {
                                        install::InstallProgress::Log { index: i, line }
                                    }
                                    install::LineEvent::Activity(line) => {
                                        install::InstallProgress::Activity { index: i, line }
                                    }
                                }
                            })
                            .await;
                        }

                        match child.wait().await {
                            Ok(status) if status.success() => {
                                let _ = sender
                                    .send(install::InstallProgress::Succeeded { index: i })
                                    .await;
                            }
                            Ok(status) => {
                                let _ = sender
                                    .send(install::InstallProgress::Failed {
                                        index: i,
                                        error: format!(
                                            "git clone exited with code {}",
                                            status.code().unwrap_or(-1)
                                        ),
                                    })
                                    .await;
                            }
                            Err(e) => {
                                let _ = sender
                                    .send(install::InstallProgress::Failed {
                                        index: i,
                                        error: format!("Wait failed: {e}"),
                                    })
                                    .await;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = sender
                            .send(install::InstallProgress::Failed {
                                index: i,
                                error: format!("Failed to spawn git: {e}"),
                            })
                            .await;
                    }
                }
            }
            let _ = sender
                .send(install::InstallProgress::Completed)
                .await;
        },
    )
}
```

- [ ] **Step 4: Add bootstrap detection and runner functions**

Append to `src/github.rs`:

```rust
/// Scan cloned repos for bootstrap scripts, returning items that have at least one.
pub(crate) fn detect_bootstrap_scripts(items: &[CloneItem]) -> Vec<BootstrapItem> {
    items
        .iter()
        .filter_map(|item| {
            let found: Vec<String> = BOOTSTRAP_SCRIPTS
                .iter()
                .filter(|s| item.destination.join(s).exists())
                .map(|s| (*s).to_string())
                .collect();

            if found.is_empty() {
                None
            } else {
                Some(BootstrapItem {
                    repo_name: item.repo.name.clone(),
                    repo_path: item.destination.clone(),
                    scripts: found,
                    status: BootstrapStatus::Pending,
                })
            }
        })
        .collect()
}

/// Run a bootstrap script in the given repo directory.
/// Returns Ok(output) on success, Err(message) on failure.
pub(crate) async fn run_bootstrap(
    repo_path: &std::path::Path,
    script: &str,
) -> Result<String, String> {
    let (program, args): (&str, Vec<String>) = if script.ends_with(".ps1") {
        (
            "powershell.exe",
            vec![
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                script.into(),
            ],
        )
    } else if script.ends_with(".sh") {
        ("bash", vec![script.into()])
    } else if script == "Makefile" {
        ("make", vec![])
    } else {
        return Err(format!("Unknown script type: {script}"));
    };

    let output = Command::new(program)
        .args(&args)
        .current_dir(repo_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(install::CREATE_NO_WINDOW)
        .output()
        .await
        .map_err(|e| format!("Failed to run {script}: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(format!("{stdout}{stderr}"))
    } else {
        Err(format!(
            "Exit code {}: {stderr}",
            output.status.code().unwrap_or(-1)
        ))
    }
}

/// Open a URL in the default browser.
pub(crate) fn open_url(url: &str) {
    let _ = open::that(url);
}
```

- [ ] **Step 5: Make `read_stdout` and `LineEvent` public in `install.rs`**

In `src/install.rs`, change the visibility of `read_stdout` (line 199) and `LineEvent` (line 50) from the current visibility to `pub(crate)`:

`LineEvent` (around line 50-54): ensure it is `pub(crate) enum LineEvent`.
`read_stdout` (around line 199): ensure it is `pub(crate) async fn read_stdout`.

These are already `pub(crate)` if they were used by `upgrade.rs` — verify and adjust if needed.

- [ ] **Step 6: Verify it compiles**

Run: `cargo build`

- [ ] **Step 7: Commit**

```bash
git add src/github.rs src/main.rs src/install.rs
git commit -m "Add GitHub API functions: device flow, fetch repos, clone stream, bootstrap detection"
```

## Chunk 2: App State, Messages & Handlers

### Task 4: Add Screen variants, App state fields, and Message variants

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `Screen` variants**

In the `Screen` enum (after `WingetSearchInstalling`, line 430), add:

```rust
    GitHubLogin,
    GitHubRepos,
    GitHubCloning,
    GitHubBootstrap,
```

- [ ] **Step 2: Add state fields to `App` struct**

In the `App` struct (after `_winget_search_handle`, line 342), add:

```rust
    // GitHub clone state
    pub(crate) github_token: Option<String>,
    pub(crate) github_user_code: Option<String>,
    pub(crate) github_verification_uri: Option<String>,
    pub(crate) github_polling: bool,
    pub(crate) github_auth_error: Option<String>,
    pub(crate) github_repos: Vec<github::GitHubRepo>,
    pub(crate) github_repos_loading: bool,
    pub(crate) github_clone_queue: Vec<github::CloneItem>,
    pub(crate) github_clone: ProgressState,
    pub(crate) github_bootstrap_items: Vec<github::BootstrapItem>,
    pub(crate) _github_device_flow_handle: Option<task::Handle>,
```

- [ ] **Step 3: Initialize new fields in `App::new()`**

In the `Self { ... }` block (after `_winget_search_handle: None,` line 408), add:

```rust
                github_token: None,
                github_user_code: None,
                github_verification_uri: None,
                github_polling: false,
                github_auth_error: None,
                github_repos: Vec::new(),
                github_repos_loading: false,
                github_clone_queue: Vec::new(),
                github_clone: ProgressState::default(),
                github_bootstrap_items: Vec::new(),
                _github_device_flow_handle: None,
```

- [ ] **Step 4: Add `Message` variants**

In the `Message` enum (before `KeyConfirm`, around line 494), add:

```rust
    GoToGitHubLogin,
    GitHubDeviceFlowProgress(github::DeviceFlowProgress),
    GitHubReposFetched(Result<Vec<github::GitHubRepo>, String>),
    GitHubSelectFolder(String),
    GitHubFolderPicked(String, std::path::PathBuf),
    GitHubRemoveFromQueue(String),
    StartGitHubClone,
    CancelGitHubClone,
    GitHubCloneProgress(install::InstallProgress),
    FinishGitHubClone,
    GitHubRunBootstrap(usize, String),
    GitHubSkipBootstrap(usize),
    GitHubBootstrapDone(usize, Result<String, String>),
    FinishGitHubBootstrap,
    OpenGitHubUrl,
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build`
Expected: warnings about unused variants (handlers come next)

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "Add Screen variants, App state, and Message variants for GitHub cloning"
```

### Task 5: Add message dispatch and handler methods

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add message dispatch arms in `update()`**

In the `match message` block, after the winget search dispatch arms (after line 570 `Message::SelectAllWingetSearch`), add:

```rust
            Message::GoToGitHubLogin => self.handle_go_to_github_login(),
            Message::GitHubDeviceFlowProgress(e) => self.handle_github_device_flow_progress(e),
            Message::GitHubReposFetched(r) => self.handle_github_repos_fetched(r),
            Message::GitHubSelectFolder(full_name) => {
                self.handle_github_select_folder(full_name)
            }
            Message::GitHubFolderPicked(full_name, path) => {
                self.github_clone_queue.push(github::CloneItem {
                    repo: self
                        .github_repos
                        .iter()
                        .find(|r| r.full_name == full_name)
                        .cloned()
                        .expect("repo must exist"),
                    destination: path,
                });
                Task::none()
            }
            Message::GitHubRemoveFromQueue(full_name) => {
                self.github_clone_queue.retain(|item| item.repo.full_name != full_name);
                Task::none()
            }
            Message::StartGitHubClone => self.handle_start_github_clone(),
            Message::CancelGitHubClone => self.handle_cancel_github_clone(),
            Message::GitHubCloneProgress(e) => self.handle_github_clone_progress(e),
            Message::FinishGitHubClone => self.handle_finish_github_clone(),
            Message::GitHubRunBootstrap(idx, script) => {
                self.handle_github_run_bootstrap(idx, script)
            }
            Message::GitHubSkipBootstrap(idx) => {
                if let Some(item) = self.github_bootstrap_items.get_mut(idx) {
                    item.status = github::BootstrapStatus::Skipped;
                }
                Task::none()
            }
            Message::GitHubBootstrapDone(idx, result) => {
                if let Some(item) = self.github_bootstrap_items.get_mut(idx) {
                    match result {
                        Ok(_) => item.status = github::BootstrapStatus::Done,
                        Err(e) => item.status = github::BootstrapStatus::Failed(e),
                    }
                }
                Task::none()
            }
            Message::FinishGitHubBootstrap => {
                self.github_bootstrap_items.clear();
                self.screen = Screen::GitHubRepos;
                Task::none()
            }
            Message::OpenGitHubUrl => {
                if let Some(ref uri) = self.github_verification_uri {
                    github::open_url(uri);
                }
                Task::none()
            }
```

- [ ] **Step 2: Add handler methods**

Add a new section after the winget search handlers (after `handle_select_all_winget_search`, around line 1143):

```rust
    // ── GitHub clone flow ────────────────────────────────────

    fn handle_go_to_github_login(&mut self) -> Task<Message> {
        self.github_token = None;
        self.github_user_code = None;
        self.github_verification_uri = None;
        self.github_auth_error = None;
        self.github_polling = true;
        self.github_repos.clear();
        self.github_clone_queue.clear();
        self.clear_search();
        self.screen = Screen::GitHubLogin;

        let dry = self.dry_run;
        let (task, handle) =
            Task::run(github::device_flow(dry), Message::GitHubDeviceFlowProgress).abortable();
        self._github_device_flow_handle = Some(handle.abort_on_drop());
        task
    }

    fn handle_github_device_flow_progress(
        &mut self,
        event: github::DeviceFlowProgress,
    ) -> Task<Message> {
        match event {
            github::DeviceFlowProgress::CodeReady {
                user_code,
                verification_uri,
            } => {
                self.github_user_code = Some(user_code);
                self.github_verification_uri = Some(verification_uri);
            }
            github::DeviceFlowProgress::Authenticated { token } => {
                self.github_token = Some(token.clone());
                self.github_polling = false;
                self._github_device_flow_handle = None;
                self.github_repos_loading = true;
                self.screen = Screen::GitHubRepos;

                let dry = self.dry_run;
                return Task::perform(
                    async move { github::fetch_repos(&token, dry).await },
                    Message::GitHubReposFetched,
                );
            }
            github::DeviceFlowProgress::Failed { error } => {
                self.github_auth_error = Some(error);
                self.github_polling = false;
                self._github_device_flow_handle = None;
            }
        }
        Task::none()
    }

    fn handle_github_repos_fetched(
        &mut self,
        result: Result<Vec<github::GitHubRepo>, String>,
    ) -> Task<Message> {
        self.github_repos_loading = false;
        match result {
            Ok(repos) => {
                self.github_repos = repos;
            }
            Err(e) => {
                self.github_auth_error = Some(e);
            }
        }
        Task::none()
    }

    fn handle_github_select_folder(&mut self, full_name: String) -> Task<Message> {
        Task::perform(
            async {
                let folder = rfd::AsyncFileDialog::new()
                    .set_title("Select clone destination")
                    .pick_folder()
                    .await;
                (full_name, folder)
            },
            |(full_name, folder)| {
                if let Some(handle) = folder {
                    Message::GitHubFolderPicked(full_name, handle.path().to_path_buf())
                } else {
                    Message::KeyIgnored
                }
            },
        )
    }

    fn handle_start_github_clone(&mut self) -> Task<Message> {
        if self.github_clone_queue.is_empty() {
            return Task::none();
        }

        let queue = self.github_clone_queue.clone();
        self.github_clone.start(queue.len());
        self.screen = Screen::GitHubCloning;

        let token = self.github_token.clone().unwrap_or_default();
        let dry = self.dry_run;
        let (task, handle) = Task::run(
            github::clone_all(queue, token, dry),
            Message::GitHubCloneProgress,
        )
        .abortable();

        self.github_clone._handle = Some(handle.abort_on_drop());
        task
    }

    fn handle_cancel_github_clone(&mut self) -> Task<Message> {
        self.github_clone.cancel("Clone");
        Task::none()
    }

    fn handle_github_clone_progress(&mut self, event: install::InstallProgress) -> Task<Message> {
        let queue = &self.github_clone_queue;
        self.github_clone.handle_event(event, |i| {
            let name = queue.get(i).map(|item| item.repo.name.as_str()).unwrap_or("...");
            format!("Cloning {name}")
        });
        Task::none()
    }

    fn handle_finish_github_clone(&mut self) -> Task<Message> {
        // Detect bootstrap scripts in cloned repos
        let bootstrap_items = github::detect_bootstrap_scripts(&self.github_clone_queue);

        self.github_clone = ProgressState::default();

        if bootstrap_items.is_empty() {
            self.github_clone_queue.clear();
            self.screen = Screen::GitHubRepos;
        } else {
            self.github_bootstrap_items = bootstrap_items;
            self.github_clone_queue.clear();
            self.screen = Screen::GitHubBootstrap;
        }
        Task::none()
    }

    fn handle_github_run_bootstrap(
        &mut self,
        idx: usize,
        script: String,
    ) -> Task<Message> {
        if let Some(item) = self.github_bootstrap_items.get_mut(idx) {
            item.status = github::BootstrapStatus::Running;
            let path = item.repo_path.clone();
            return Task::perform(
                async move { github::run_bootstrap(&path, &script).await },
                move |result| Message::GitHubBootstrapDone(idx, result),
            );
        }
        Task::none()
    }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "Add GitHub clone message dispatch and handler methods"
```

### Task 6: Wire up keyboard handlers, back navigation, spinner, and copy log

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update `handle_go_back` (around line 736)**

Add before the `_ =>` catch-all (line 762):

```rust
            Screen::GitHubLogin => {
                self._github_device_flow_handle = None;
                self.github_polling = false;
                self.screen = Screen::ProfileSelect;
            }
            Screen::GitHubRepos => {
                self.github_token = None;
                self.github_repos.clear();
                self.github_clone_queue.clear();
                self.screen = Screen::ProfileSelect;
            }
```

- [ ] **Step 2: Update `handle_key_confirm` (around line 1313)**

Add before the `_ => Task::none()` catch-all (line 1344):

```rust
            Screen::GitHubRepos if !self.github_clone_queue.is_empty() => {
                self.handle_start_github_clone()
            }
            Screen::GitHubCloning if self.github_clone.done => {
                self.handle_finish_github_clone()
            }
```

- [ ] **Step 3: Update `handle_key_escape` (around line 1348)**

Add arms for GitHub screens:

```rust
            Screen::GitHubLogin | Screen::GitHubRepos => self.handle_go_back(),
            Screen::GitHubCloning if !self.github_clone.done => {
                self.handle_cancel_github_clone()
            }
            Screen::GitHubCloning if self.github_clone.done => {
                self.handle_finish_github_clone()
            }
            Screen::GitHubBootstrap => {
                self.github_bootstrap_items.clear();
                self.screen = Screen::GitHubRepos;
                Task::none()
            }
```

- [ ] **Step 4: Update `handle_focus_search` (around line 1369)**

Add `Screen::GitHubRepos` to the match arm:

```rust
            Screen::PackageSelect
            | Screen::UpdateSelect
            | Screen::UninstallSelect
            | Screen::WingetSearch
            | Screen::GitHubRepos => widget::operation::focus(widget::Id::new(SEARCH_INPUT_ID)),
```

- [ ] **Step 5: Update `handle_copy_log` (around line 1252)**

Add a `Screen::GitHubCloning` arm before `_ =>`:

```rust
                Screen::GitHubCloning => &mut self.github_clone,
```

- [ ] **Step 6: Update `ClearCopyStatus` handler (around line 623)**

Add clearing for github clone:

```rust
                self.github_clone.copy_status = false;
```

- [ ] **Step 7: Update spinner subscription (around line 1413)**

Add GitHub conditions:

```rust
            || matches!(self.screen, Screen::GitHubLogin if self.github_polling)
            || matches!(self.screen, Screen::GitHubRepos if self.github_repos_loading)
            || matches!(self.screen, Screen::GitHubCloning if !self.github_clone.done)
            || matches!(self.screen, Screen::GitHubBootstrap if self.github_bootstrap_items.iter().any(|i| i.status == github::BootstrapStatus::Running))
```

- [ ] **Step 8: Verify it compiles**

Run: `cargo build`

- [ ] **Step 9: Commit**

```bash
git add src/main.rs
git commit -m "Wire up keyboard, back nav, spinner, and copy log for GitHub clone screens"
```

## Chunk 3: Views

### Task 7: Add GitHub view methods to `views.rs`

**Files:**
- Modify: `src/views.rs`

- [ ] **Step 1: Add import for GitHub types**

At the top of `views.rs`, add:

```rust
use crate::github::{BootstrapStatus, GitHubRepo};
```

- [ ] **Step 2: Add `view_github_login` method on `impl App`**

Add after the winget search view methods (after `view_winget_search_installing`):

```rust
    pub(crate) fn view_github_login(&self) -> Element<'_, Message> {
        let header = back_header("Clone repos");

        let content: Element<'_, Message> = if let Some(ref error) = self.github_auth_error {
            // Error state
            let retry_btn = button(text("Try again").size(14))
                .style(continue_button_style)
                .padding([8, 20])
                .on_press(Message::GoToGitHubLogin);

            container(
                column![
                    text(char::from(Icon::CircleX))
                        .size(32)
                        .font(LUCIDE_FONT)
                        .color(STATUS_RED),
                    text("Authentication failed").size(16),
                    text(error).size(13).color(MUTED),
                    retry_btn,
                ]
                .spacing(12)
                .align_x(iced::Alignment::Center)
                .width(Length::Fill),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else if let Some(ref code) = self.github_user_code {
            // Code ready — show code + open button
            let code_display = container(
                text(code).size(28).font(iced::Font::MONOSPACE),
            )
            .padding([16, 32])
            .style(|_: &_| container::Style {
                background: Some(iced::Background::Color(CARD_BG)),
                border: iced::Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: BORDER,
                },
                ..Default::default()
            });

            let open_btn = button(
                row![
                    text(char::from(Icon::ExternalLink))
                        .size(14)
                        .font(LUCIDE_FONT),
                    text("Open GitHub").size(14),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .style(continue_button_style)
            .padding([10, 24])
            .on_press(Message::OpenGitHubUrl);

            let spinner = spinner_indicator(
                self.spinner_frame,
                "Waiting for authorization...".into(),
                MUTED,
            );

            container(
                column![
                    text("Sign in to GitHub").size(18),
                    text("Enter this code at github.com/login/device")
                        .size(13)
                        .color(MUTED),
                    code_display,
                    open_btn,
                    spinner,
                ]
                .spacing(16)
                .align_x(iced::Alignment::Center)
                .width(Length::Fill),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            // Initial loading
            container(spinner_indicator(
                self.spinner_frame,
                "Connecting to GitHub...".into(),
                MUTED,
            ))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        };

        let layout = column![header, content]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }
```

- [ ] **Step 3: Add `view_github_repos` method**

```rust
    pub(crate) fn view_github_repos(&self) -> Element<'_, Message> {
        let header = back_header("Your repositories");

        // Search bar
        let search_field = text_input("Filter repos...", &self.search)
            .id(iced::widget::Id::new(SEARCH_INPUT_ID))
            .on_input(Message::SearchChanged)
            .padding(8)
            .size(14)
            .width(Length::Fill);

        let search_row = row![search_field].width(Length::Fill);

        // Repo list
        let results_content: Element<'_, Message> = if self.github_repos_loading {
            container(spinner_indicator(
                self.spinner_frame,
                "Loading repositories...".into(),
                MUTED,
            ))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else if self.github_repos.is_empty() {
            let msg = if let Some(ref e) = self.github_auth_error {
                e.as_str()
            } else {
                "No repositories found"
            };
            container(text(msg).size(14).color(MUTED))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else {
            let sl = self.search_lower.as_str();
            let queued_names: std::collections::HashSet<&str> = self
                .github_clone_queue
                .iter()
                .map(|item| item.repo.full_name.as_str())
                .collect();

            let mut repo_list = column![].spacing(2).width(Length::Fill);

            for repo in &self.github_repos {
                if !sl.is_empty()
                    && !repo.name_lower.contains(sl)
                    && !repo.desc_lower.contains(sl)
                {
                    continue;
                }

                let is_queued = queued_names.contains(repo.full_name.as_str());

                let visibility_badge = if repo.private {
                    text("private").size(10).color(STATUS_AMBER)
                } else {
                    text("public").size(10).color(MUTED)
                };

                let desc = text(
                    repo.description
                        .as_deref()
                        .unwrap_or("")
                )
                .size(12)
                .color(MUTED);

                let name_col = column![
                    row![
                        text(&repo.name).size(14),
                        visibility_badge,
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                    desc,
                ]
                .spacing(2)
                .width(Length::Fill);

                let action: Element<'_, Message> = if is_queued {
                    text(char::from(Icon::Check))
                        .size(14)
                        .font(LUCIDE_FONT)
                        .color(STATUS_GREEN)
                        .into()
                } else {
                    let full_name = repo.full_name.clone();
                    button(
                        text("Select folder").size(12),
                    )
                    .style(ghost_button_style)
                    .padding([4, 10])
                    .on_press(Message::GitHubSelectFolder(full_name))
                    .into()
                };

                let repo_row = row![name_col, action]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .padding([8, 12]);

                let row_el: Element<'_, Message> = if is_queued {
                    container(repo_row)
                        .style(|_: &_| container::Style {
                            background: Some(iced::Background::Color(CARD_BG)),
                            border: iced::Border {
                                radius: 6.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .width(Length::Fill)
                        .into()
                } else {
                    container(repo_row).width(Length::Fill).into()
                };

                repo_list = repo_list.push(row_el);
            }

            scrollable(repo_list.padding(iced::Padding::from([0, 20, 0, 0])))
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        };

        // Clone queue
        let queue_section: Element<'_, Message> = if self.github_clone_queue.is_empty() {
            iced::widget::Space::new().height(0).into()
        } else {
            let mut queue_col = column![
                text("Clone queue").size(13).color(MUTED),
            ]
            .spacing(4)
            .width(Length::Fill);

            for item in &self.github_clone_queue {
                let full_name = item.repo.full_name.clone();
                let remove_btn = button(
                    text(char::from(Icon::X)).size(12).font(LUCIDE_FONT),
                )
                .style(ghost_button_style)
                .padding([2, 6])
                .on_press(Message::GitHubRemoveFromQueue(full_name));

                let queue_row = row![
                    text(&item.repo.name).size(13),
                    text(char::from(Icon::ArrowRight))
                        .size(11)
                        .font(LUCIDE_FONT)
                        .color(MUTED),
                    text(item.destination.display().to_string())
                        .size(12)
                        .font(iced::Font::MONOSPACE)
                        .color(MUTED_FG),
                    iced::widget::Space::new().width(Length::Fill),
                    remove_btn,
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center);

                queue_col = queue_col.push(queue_row);
            }

            queue_col.into()
        };

        // Footer
        let queue_count = self.github_clone_queue.len();
        let mut clone_btn = button(
            text(format!("Clone all ({queue_count})")).size(14),
        )
        .style(continue_button_style)
        .padding([8, 20]);

        if queue_count > 0 {
            clone_btn = clone_btn.on_press(Message::StartGitHubClone);
        }

        let footer = row![
            iced::widget::Space::new().width(Length::Fill),
            clone_btn,
        ]
        .align_y(iced::Alignment::Center);

        let layout = column![header, search_row, results_content, queue_section, footer]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }
```

- [ ] **Step 4: Add `view_github_cloning` method**

```rust
    pub(crate) fn view_github_cloning(&self) -> Element<'_, Message> {
        view_progress_screen(
            &self.github_clone,
            &ProgressLabels {
                verb: "Cloning",
                done_label: "Clone",
                dry_run_warning: "No repos will actually be cloned",
            },
            self.github_clone_queue
                .iter()
                .map(|item| item.repo.name.as_str()),
            self.dry_run,
            Message::CancelGitHubClone,
            Message::FinishGitHubClone,
            self.spinner_frame,
        )
    }
```

- [ ] **Step 5: Add `view_github_bootstrap` method**

```rust
    pub(crate) fn view_github_bootstrap(&self) -> Element<'_, Message> {
        let header = text("Setup scripts detected").size(18);
        let subtitle = text("These repos have bootstrap scripts that can set things up for you.")
            .size(13)
            .color(MUTED);

        let mut list = column![].spacing(8).width(Length::Fill);

        for (idx, item) in self.github_bootstrap_items.iter().enumerate() {
            let status_indicator: Element<'_, Message> = match &item.status {
                BootstrapStatus::Pending => {
                    iced::widget::Space::new().width(0).into()
                }
                BootstrapStatus::Running => spinner_indicator(
                    self.spinner_frame,
                    "Running...".into(),
                    STATUS_BLUE,
                ),
                BootstrapStatus::Done => text(char::from(Icon::Check))
                    .size(14)
                    .font(LUCIDE_FONT)
                    .color(STATUS_GREEN)
                    .into(),
                BootstrapStatus::Skipped => text("skipped")
                    .size(12)
                    .color(MUTED)
                    .into(),
                BootstrapStatus::Failed(e) => text(e)
                    .size(12)
                    .color(STATUS_RED)
                    .into(),
            };

            let actions: Element<'_, Message> = if item.status == BootstrapStatus::Pending {
                if item.scripts.len() == 1 {
                    let script = item.scripts[0].clone();
                    let run_btn = button(
                        row![
                            text(char::from(Icon::Play))
                                .size(12)
                                .font(LUCIDE_FONT),
                            text(format!("Run {}", &item.scripts[0])).size(12),
                        ]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                    )
                    .style(continue_button_style)
                    .padding([4, 12])
                    .on_press(Message::GitHubRunBootstrap(idx, script));

                    let skip_btn = button(text("Skip").size(12))
                        .style(ghost_button_style)
                        .padding([4, 10])
                        .on_press(Message::GitHubSkipBootstrap(idx));

                    row![run_btn, skip_btn].spacing(6).into()
                } else {
                    // Multiple scripts — show a button per script
                    let mut btns = row![].spacing(4);
                    for script in &item.scripts {
                        let s = script.clone();
                        btns = btns.push(
                            button(text(script).size(11))
                                .style(ghost_button_style)
                                .padding([4, 8])
                                .on_press(Message::GitHubRunBootstrap(idx, s)),
                        );
                    }
                    let skip_btn = button(text("Skip").size(12))
                        .style(ghost_button_style)
                        .padding([4, 10])
                        .on_press(Message::GitHubSkipBootstrap(idx));
                    btns = btns.push(skip_btn);
                    btns.into()
                }
            } else {
                iced::widget::Space::new().width(0).into()
            };

            let item_row = row![
                column![
                    text(&item.repo_name).size(14),
                    text(item.repo_path.display().to_string())
                        .size(11)
                        .font(iced::Font::MONOSPACE)
                        .color(MUTED),
                ]
                .spacing(2)
                .width(Length::Fill),
                status_indicator,
                actions,
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .padding([10, 12]);

            list = list.push(
                container(item_row)
                    .style(|_: &_| container::Style {
                        background: Some(iced::Background::Color(CARD_BG)),
                        border: iced::Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .width(Length::Fill),
            );
        }

        let all_handled = self.github_bootstrap_items.iter().all(|item| {
            !matches!(item.status, BootstrapStatus::Pending | BootstrapStatus::Running)
        });

        let mut done_btn = button(text("Done").size(14))
            .style(continue_button_style)
            .padding([8, 20]);
        if all_handled {
            done_btn = done_btn.on_press(Message::FinishGitHubBootstrap);
        }

        let footer = row![
            iced::widget::Space::new().width(Length::Fill),
            done_btn,
        ];

        let layout = column![header, subtitle, list, footer]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo build`

- [ ] **Step 7: Commit**

```bash
git add src/views.rs
git commit -m "Add view methods for GitHub login, repos, cloning, and bootstrap screens"
```

### Task 8: Add view dispatch and home screen entry card

**Files:**
- Modify: `src/main.rs`
- Modify: `src/views.rs`

- [ ] **Step 1: Add view dispatch arms in `main.rs`**

In `App::view()` (around line 1393), add after `Screen::WingetSearchInstalling`:

```rust
            Screen::GitHubLogin => self.view_github_login(),
            Screen::GitHubRepos => self.view_github_repos(),
            Screen::GitHubCloning => self.view_github_cloning(),
            Screen::GitHubBootstrap => self.view_github_bootstrap(),
```

- [ ] **Step 2: Add action card to ProfileSelect in `views.rs`**

In `view_profile_select()`, find the section where action cards are pushed (around line 181-192). Add the GitHub card:

```rust
        let github_card = action_card(
            Icon::Github,
            "Clone repos",
            Some(Message::GoToGitHubLogin),
        );
```

And update the content push chain to include it after `search_card`:

```rust
        let content = content
            .push(update_card)
            .push(uninstall_card)
            .push(search_card)
            .push(github_card)
            .push(settings_card)
            .push(status_row);
```

- [ ] **Step 3: Verify it compiles and runs**

Run: `just check` then `cargo run -- --dry`

Verify:
1. "Clone repos" card appears on home screen
2. Clicking it opens the login screen with fake device code
3. After 3 seconds, transitions to repos screen with fake repos
4. Clicking "Select folder" opens a folder picker
5. After selecting a folder, repo appears in clone queue
6. "Clone all" starts cloning progress
7. Done returns to repos screen
8. Escape goes back to home
9. Ctrl+K focuses search on repos screen

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/views.rs
git commit -m "Add view dispatch and Clone repos card to home screen"
```

### Task 9: Run full check suite

**Files:** (no changes)

- [ ] **Step 1: Run `just check`**

Run: `just check`
Expected: `cargo build` + `cargo clippy` + `cargo fmt --check` all pass

- [ ] **Step 2: Fix any clippy/fmt issues**

If clippy warnings appear, fix them. If fmt fails, run `just fmt`.

- [ ] **Step 3: Final commit if fixes were needed**

```bash
git add -A
git commit -m "Fix clippy/fmt issues from GitHub clone integration"
```
