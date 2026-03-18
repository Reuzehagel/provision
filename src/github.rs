use std::path::PathBuf;

use serde::Deserialize;

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
