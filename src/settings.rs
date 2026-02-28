use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ── Settings tab ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    Winget,
    Changelog,
}

// ── Install mode ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallMode {
    Silent,
    Interactive,
}

impl InstallMode {
    pub const ALL: [Self; 2] = [Self::Silent, Self::Interactive];
}

impl fmt::Display for InstallMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Silent => write!(f, "Silent"),
            Self::Interactive => write!(f, "Interactive"),
        }
    }
}

// ── Install scope ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallScope {
    User,
    Machine,
}

impl fmt::Display for InstallScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User => write!(f, "User"),
            Self::Machine => write!(f, "Machine"),
        }
    }
}

/// Newtype so `pick_list` can display "Default" for `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionalScope(pub Option<InstallScope>);

impl OptionalScope {
    pub const ALL: [Self; 3] = [
        Self(None),
        Self(Some(InstallScope::User)),
        Self(Some(InstallScope::Machine)),
    ];
}

impl fmt::Display for OptionalScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            None => write!(f, "Default"),
            Some(s) => write!(f, "{s}"),
        }
    }
}

// ── Architecture ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Architecture {
    #[serde(rename = "x86")]
    X86,
    #[serde(rename = "x64")]
    X64,
    #[serde(rename = "arm64")]
    Arm64,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::X86 => write!(f, "x86"),
            Self::X64 => write!(f, "x64"),
            Self::Arm64 => write!(f, "arm64"),
        }
    }
}

/// Newtype so `pick_list` can display "Default" for `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionalArchitecture(pub Option<Architecture>);

impl OptionalArchitecture {
    pub const ALL: [Self; 4] = [
        Self(None),
        Self(Some(Architecture::X86)),
        Self(Some(Architecture::X64)),
        Self(Some(Architecture::Arm64)),
    ];
}

impl fmt::Display for OptionalArchitecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            None => write!(f, "Default"),
            Some(a) => write!(f, "{a}"),
        }
    }
}

// ── Winget settings ──────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct WingetSettings {
    pub install_mode: InstallMode,
    pub scope: Option<InstallScope>,
    pub architecture: Option<Architecture>,
    pub force: bool,
    pub include_unknown: bool,
    pub ignore_security_hash: bool,
    pub disable_interactivity: bool,
    pub install_location: String,
}

impl Default for WingetSettings {
    fn default() -> Self {
        Self {
            install_mode: InstallMode::Silent,
            scope: None,
            architecture: None,
            force: false,
            include_unknown: true,
            ignore_security_hash: false,
            disable_interactivity: false,
            install_location: String::new(),
        }
    }
}

impl WingetSettings {
    /// Build extra CLI flags for install/upgrade commands.
    pub fn install_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        match self.install_mode {
            InstallMode::Silent => args.push("--silent".into()),
            InstallMode::Interactive => args.push("--interactive".into()),
        }

        if let Some(scope) = &self.scope {
            args.push("--scope".into());
            args.push(scope.to_string().to_lowercase());
        }

        if let Some(arch) = &self.architecture {
            args.push("--architecture".into());
            args.push(arch.to_string());
        }

        if self.force {
            args.push("--force".into());
        }

        if self.ignore_security_hash {
            args.push("--ignore-security-hash".into());
        }

        if self.disable_interactivity {
            args.push("--disable-interactivity".into());
        }

        if !self.install_location.is_empty() {
            args.push("--location".into());
            args.push(self.install_location.clone());
        }

        args
    }
}

// ── Persistence ─────────────────────────────────────────────────

fn settings_path() -> Option<PathBuf> {
    crate::catalog::dirs_cache_dir()
        .ok()
        .map(|d| d.join("settings.toml"))
}

/// Load settings from disk. Returns `Default` on any failure.
pub fn load_settings() -> WingetSettings {
    let Some(path) = settings_path() else {
        return WingetSettings::default();
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return WingetSettings::default();
    };
    toml::from_str(&raw).unwrap_or_default()
}

/// Serialize settings to a TOML string for async persistence.
pub fn serialize_settings(settings: &WingetSettings) -> Option<String> {
    toml::to_string_pretty(settings).ok()
}

/// Write a pre-serialized settings string to disk (best-effort).
pub async fn save_settings(content: String) {
    let Some(path) = settings_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let _ = tokio::fs::write(&path, content).await;
}
