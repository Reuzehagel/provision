use std::path::PathBuf;

use iced::futures;
use iced::futures::SinkExt as _;
use iced::stream;
use serde::Deserialize;

use crate::install;

/// Encode key-value pairs as `application/x-www-form-urlencoded`.
fn form_encode(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoded(k), urlencoded(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Minimal percent-encoding for form values (encode everything except unreserved chars).
fn urlencoded(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

/// Client ID from the registered GitHub OAuth App for provision.
/// This is a public value — safe to embed in the binary.
const GITHUB_CLIENT_ID: &str = "Ov23li4Kne9leSVg4CZs";

type Sender = futures::channel::mpsc::Sender<DeviceFlowProgress>;

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
    #[allow(dead_code)]
    pub(crate) description: Option<String>,
    #[allow(dead_code)]
    pub(crate) private: bool,
    pub(crate) clone_url: String,
    #[allow(dead_code)]
    pub(crate) html_url: String,
    // Precomputed for search filtering
    #[allow(dead_code)]
    pub(crate) name_lower: String,
    #[allow(dead_code)]
    pub(crate) desc_lower: String,
}

impl From<RepoResponse> for GitHubRepo {
    fn from(r: RepoResponse) -> Self {
        let name_lower = r.name.to_lowercase();
        let desc_lower = r.description.as_deref().unwrap_or("").to_lowercase();
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
    #[allow(dead_code)]
    pub(crate) repo_name: String,
    pub(crate) repo_path: PathBuf,
    #[allow(dead_code)]
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

// ── Device Flow ────────────────────────────────────────────

pub(crate) fn device_flow(dry_run: bool) -> impl futures::Stream<Item = DeviceFlowProgress> + Send {
    stream::channel(100, move |mut sender: Sender| async move {
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

        let client = reqwest::Client::new();

        // Step 1: Request device code
        let resp = match client
            .post("https://github.com/login/device/code")
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_encode(&[
                ("client_id", GITHUB_CLIENT_ID),
                ("scope", "repo"),
            ]))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let _ = sender
                    .send(DeviceFlowProgress::Failed {
                        error: format!("Request failed: {e}"),
                    })
                    .await;
                return;
            }
        };

        let body = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                let _ = sender
                    .send(DeviceFlowProgress::Failed {
                        error: format!("Read body failed: {e}"),
                    })
                    .await;
                return;
            }
        };

        let device_code_resp: DeviceCodeResponse = match serde_json::from_str(&body) {
            Ok(d) => d,
            Err(e) => {
                let _ = sender
                    .send(DeviceFlowProgress::Failed {
                        error: format!("Parse failed: {e}"),
                    })
                    .await;
                return;
            }
        };

        let _ = sender
            .send(DeviceFlowProgress::CodeReady {
                user_code: device_code_resp.user_code.clone(),
                verification_uri: device_code_resp.verification_uri.clone(),
            })
            .await;

        // Step 2: Poll for token
        let mut interval = device_code_resp.interval;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

            let poll_resp = match client
                .post("https://github.com/login/oauth/access_token")
                .header("Accept", "application/json")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(form_encode(&[
                    ("client_id", GITHUB_CLIENT_ID),
                    ("device_code", device_code_resp.device_code.as_str()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ]))
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let _ = sender
                        .send(DeviceFlowProgress::Failed {
                            error: format!("Poll request failed: {e}"),
                        })
                        .await;
                    return;
                }
            };

            let poll_body = match poll_resp.text().await {
                Ok(t) => t,
                Err(e) => {
                    let _ = sender
                        .send(DeviceFlowProgress::Failed {
                            error: format!("Poll read failed: {e}"),
                        })
                        .await;
                    return;
                }
            };

            let token_resp: TokenResponse = match serde_json::from_str(&poll_body) {
                Ok(t) => t,
                Err(e) => {
                    let _ = sender
                        .send(DeviceFlowProgress::Failed {
                            error: format!("Poll parse failed: {e}"),
                        })
                        .await;
                    return;
                }
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
                    interval += 5;
                    continue;
                }
                Some(err) => {
                    let _ = sender
                        .send(DeviceFlowProgress::Failed {
                            error: err.to_string(),
                        })
                        .await;
                    return;
                }
                None => {
                    let _ = sender
                        .send(DeviceFlowProgress::Failed {
                            error: "Unexpected response: no token and no error".into(),
                        })
                        .await;
                    return;
                }
            }
        }
    })
}

// ── Fetch Repos ────────────────────────────────────────────

pub(crate) async fn fetch_repos(token: &str, dry_run: bool) -> Result<Vec<GitHubRepo>, String> {
    if dry_run {
        return Ok(vec![
            GitHubRepo {
                name: "dotfiles".into(),
                full_name: "user/dotfiles".into(),
                description: Some("Personal dotfiles and configs".into()),
                private: false,
                clone_url: "https://github.com/user/dotfiles.git".into(),
                html_url: "https://github.com/user/dotfiles".into(),
                name_lower: "dotfiles".into(),
                desc_lower: "personal dotfiles and configs".into(),
            },
            GitHubRepo {
                name: "my-project".into(),
                full_name: "user/my-project".into(),
                description: Some("A private project".into()),
                private: true,
                clone_url: "https://github.com/user/my-project.git".into(),
                html_url: "https://github.com/user/my-project".into(),
                name_lower: "my-project".into(),
                desc_lower: "a private project".into(),
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
    let mut all_repos: Vec<GitHubRepo> = Vec::new();
    let mut page = 1u32;

    loop {
        let url =
            format!("https://api.github.com/user/repos?per_page=100&sort=updated&page={page}");

        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "provision")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let body = resp.text().await.map_err(|e| format!("Read failed: {e}"))?;
        let repos: Vec<RepoResponse> =
            serde_json::from_str(&body).map_err(|e| format!("Parse failed: {e}"))?;

        let count = repos.len();
        all_repos.extend(repos.into_iter().map(GitHubRepo::from));

        if count < 100 || all_repos.len() >= 500 {
            break;
        }
        page += 1;
    }

    Ok(all_repos)
}

// ── Clone Stream ───────────────────────────────────────────

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
                                "[DRY RUN] Would clone {} to {}",
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

                // Inject token into clone URL
                let auth_url = item.repo.clone_url.replacen(
                    "https://",
                    &format!("https://oauth2:{token}@"),
                    1,
                );

                let dest = item.destination.to_string_lossy().to_string();

                let mut child = match tokio::process::Command::new("git")
                    .args(["clone", "--progress", &auth_url, &dest])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::piped())
                    .creation_flags(install::CREATE_NO_WINDOW)
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = sender
                            .send(install::InstallProgress::Failed {
                                index: i,
                                error: format!("Failed to spawn git: {e}"),
                            })
                            .await;
                        continue;
                    }
                };

                // Git writes progress to stderr
                if let Some(stderr) = child.stderr.take() {
                    let _ = install::read_stdout(stderr, &mut sender, |event| match event {
                        install::LineEvent::Log(line) => {
                            install::InstallProgress::Log { index: i, line }
                        }
                        install::LineEvent::Activity(line) => {
                            install::InstallProgress::Activity { index: i, line }
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

            let _ = sender.send(install::InstallProgress::Completed).await;
        },
    )
}

// ── Bootstrap Detection & Running ──────────────────────────

pub(crate) fn detect_bootstrap_scripts(items: &[CloneItem]) -> Vec<BootstrapItem> {
    let mut result = Vec::new();
    for item in items {
        let mut found_scripts = Vec::new();
        for &script in BOOTSTRAP_SCRIPTS {
            if item.destination.join(script).exists() {
                found_scripts.push(script.to_string());
            }
        }
        if !found_scripts.is_empty() {
            result.push(BootstrapItem {
                repo_name: item.repo.name.clone(),
                repo_path: item.destination.clone(),
                scripts: found_scripts,
                status: BootstrapStatus::Pending,
            });
        }
    }
    result
}

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
                repo_path.join(script).to_string_lossy().into_owned(),
            ],
        )
    } else if script.ends_with(".sh") {
        (
            "bash",
            vec![repo_path.join(script).to_string_lossy().into_owned()],
        )
    } else if script == "Makefile" {
        ("make", vec![])
    } else {
        return Err(format!("Unknown script type: {script}"));
    };

    let output = tokio::process::Command::new(program)
        .args(&args)
        .current_dir(repo_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(install::CREATE_NO_WINDOW)
        .output()
        .await
        .map_err(|e| format!("Failed to run {script}: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(format!("{stdout}{stderr}"))
    } else {
        Err(format!(
            "Exit code {}: {stderr}",
            output.status.code().unwrap_or(-1)
        ))
    }
}

pub(crate) fn open_url(url: &str) {
    let _ = open::that(url);
}
