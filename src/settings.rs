use std::fmt;

// ── Install mode ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86,
    X64,
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
