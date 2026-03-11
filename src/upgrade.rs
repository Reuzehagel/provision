use iced::futures;
use iced::futures::SinkExt as _;
use iced::stream;
use tokio::process::Command;

use crate::install::{self, BatchItem, InstallProgress, LineEvent};

/// Events emitted by `run_winget_scan` for the caller to map.
enum ScanEvent {
    Activity(String),
    Log(String),
    Failed(String),
}

/// Shared helper: spawn a winget command, read stdout, collect log lines,
/// and return them. Sends events through a single mapper callback.
async fn run_winget_scan<T: Send>(
    args: &[&str],
    sender: &mut futures::channel::mpsc::Sender<T>,
    map: impl Fn(ScanEvent) -> T,
) -> Result<Vec<String>, ()> {
    let child = Command::new("winget")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .creation_flags(install::CREATE_NO_WINDOW)
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            let _ = sender
                .send(map(ScanEvent::Failed(format!(
                    "Failed to spawn winget: {e}"
                ))))
                .await;
            return Err(());
        }
    };

    let mut all_lines: Vec<String> = Vec::new();

    if let Some(stdout) = child.stdout.take() {
        let all_lines = &mut all_lines;
        let result = install::read_stdout(stdout, sender, |event| match event {
            LineEvent::Log(line) => {
                all_lines.push(line.clone());
                map(ScanEvent::Log(line))
            }
            LineEvent::Activity(line) => map(ScanEvent::Activity(line)),
        })
        .await;

        if let Err(e) = result {
            let _ = sender.send(map(ScanEvent::Failed(e))).await;
            return Err(());
        }
    }

    let _ = child.wait().await;
    Ok(all_lines)
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in upcoming uninstall screen
pub struct InstalledPackage {
    pub name: String,
    pub winget_id: String, // original case for winget uninstall --id
    pub version: String,
    pub source: String,
    pub winget_id_lower: String, // precomputed for search & is_installed()
    pub name_lower: String,      // precomputed for search
    pub size_bytes: Option<u64>, // filled async from registry
}

#[derive(Debug, Clone)]
pub enum InstalledScanProgress {
    Activity {
        #[allow(dead_code)]
        line: String,
    },
    Completed {
        packages: Vec<InstalledPackage>,
    },
    Failed {
        #[allow(dead_code)]
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct UpgradeablePackage {
    pub name: String,
    pub winget_id: String,
    pub current_version: String,
    pub available_version: String,
    #[allow(dead_code)]
    pub source: String,
    /// Precomputed `name.to_lowercase()` for search filtering.
    pub name_lower: String,
    /// Precomputed `winget_id.to_lowercase()` for search filtering.
    pub winget_id_lower: String,
}

impl BatchItem for UpgradeablePackage {
    fn name(&self) -> &str {
        &self.name
    }
    fn winget_id(&self) -> &str {
        &self.winget_id
    }
}

impl BatchItem for InstalledPackage {
    fn name(&self) -> &str {
        &self.name
    }
    fn winget_id(&self) -> &str {
        &self.winget_id
    }
}

#[derive(Debug, Clone)]
pub enum ScanProgress {
    Activity { line: String },
    Log { line: String },
    Completed { packages: Vec<UpgradeablePackage> },
    Failed { error: String },
}

pub fn scan_installed(dry_run: bool) -> impl futures::Stream<Item = InstalledScanProgress> + Send {
    stream::channel(
        100,
        move |mut sender: futures::channel::mpsc::Sender<InstalledScanProgress>| async move {
            if dry_run {
                let _ = sender
                    .send(InstalledScanProgress::Activity {
                        line: "Scanning installed packages...".into(),
                    })
                    .await;

                tokio::time::sleep(std::time::Duration::from_millis(800)).await;

                let fake = vec![
                    InstalledPackage {
                        name: "Git".into(),
                        winget_id: "Git.Git".into(),
                        version: "2.47.0".into(),
                        source: "winget".into(),
                        winget_id_lower: "git.git".into(),
                        name_lower: "git".into(),
                        size_bytes: None,
                    },
                    InstalledPackage {
                        name: "Mozilla Firefox".into(),
                        winget_id: "Mozilla.Firefox".into(),
                        version: "131.0".into(),
                        source: "winget".into(),
                        winget_id_lower: "mozilla.firefox".into(),
                        name_lower: "mozilla firefox".into(),
                        size_bytes: None,
                    },
                    InstalledPackage {
                        name: "7-Zip".into(),
                        winget_id: "7zip.7zip".into(),
                        version: "24.08".into(),
                        source: "winget".into(),
                        winget_id_lower: "7zip.7zip".into(),
                        name_lower: "7-zip".into(),
                        size_bytes: None,
                    },
                    InstalledPackage {
                        name: "Windows Terminal".into(),
                        winget_id: "Microsoft.WindowsTerminal".into(),
                        version: "1.21.0".into(),
                        source: "winget".into(),
                        winget_id_lower: "microsoft.windowsterminal".into(),
                        name_lower: "windows terminal".into(),
                        size_bytes: None,
                    },
                    InstalledPackage {
                        name: "Visual Studio Code".into(),
                        winget_id: "Microsoft.VisualStudioCode".into(),
                        version: "1.95.0".into(),
                        source: "winget".into(),
                        winget_id_lower: "microsoft.visualstudiocode".into(),
                        name_lower: "visual studio code".into(),
                        size_bytes: None,
                    },
                ];

                let _ = sender
                    .send(InstalledScanProgress::Completed { packages: fake })
                    .await;
                return;
            }

            let Ok(all_lines) = run_winget_scan(&["list"], &mut sender, |e| match e {
                ScanEvent::Activity(line) | ScanEvent::Log(line) => {
                    InstalledScanProgress::Activity { line }
                }
                ScanEvent::Failed(error) => InstalledScanProgress::Failed { error },
            })
            .await
            else {
                return;
            };

            let packages = parse_list_table(&all_lines);
            let _ = sender
                .send(InstalledScanProgress::Completed { packages })
                .await;
        },
    )
}

pub fn parse_list_table(lines: &[String]) -> Vec<InstalledPackage> {
    let header_idx = lines
        .iter()
        .position(|l| l.contains("Name") && l.contains("Id") && l.contains("Version"));

    let Some(header_idx) = header_idx else {
        return Vec::new();
    };

    let header = &lines[header_idx];

    let Some(id_col) = header.find("Id") else {
        return Vec::new();
    };
    let Some(version_col) = header.find("Version") else {
        return Vec::new();
    };

    let name_col = header.find("Name").unwrap_or(0);
    let version_end = header.find("Source").unwrap_or(usize::MAX);
    let data_start = find_data_start(lines, header_idx);

    let mut packages = Vec::new();

    for line in &lines[data_start..] {
        if line.len() < version_col + 1 {
            continue;
        }

        let name = safe_slice(line, name_col, id_col);
        let id = safe_slice(line, id_col, version_col);
        let version = if version_end < usize::MAX {
            safe_slice(line, version_col, version_end)
        } else {
            safe_slice_to_end(line, version_col)
        };
        let source = if version_end < usize::MAX && line.len() > version_end {
            safe_slice_to_end(line, version_end)
        } else {
            String::new()
        };

        if id.is_empty() {
            continue;
        }

        packages.push(InstalledPackage {
            name_lower: name.to_lowercase(),
            winget_id_lower: id.to_lowercase(),
            name,
            winget_id: id,
            version,
            source,
            size_bytes: None,
        });
    }

    packages
}

pub fn scan_upgrades(
    dry_run: bool,
    include_unknown: bool,
) -> impl futures::Stream<Item = ScanProgress> + Send {
    stream::channel(
        100,
        move |mut sender: futures::channel::mpsc::Sender<ScanProgress>| async move {
            if dry_run {
                let _ = sender
                    .send(ScanProgress::Log {
                        line: "[DRY RUN] Scanning for upgradeable packages...".into(),
                    })
                    .await;

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                let _ = sender
                    .send(ScanProgress::Activity {
                        line: "Checking sources...".into(),
                    })
                    .await;

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                let fake = vec![
                    UpgradeablePackage {
                        name: "Mozilla Firefox".into(),
                        winget_id: "Mozilla.Firefox".into(),
                        current_version: "130.0".into(),
                        available_version: "131.0".into(),
                        source: "winget".into(),
                        name_lower: "mozilla firefox".into(),
                        winget_id_lower: "mozilla.firefox".into(),
                    },
                    UpgradeablePackage {
                        name: "Visual Studio Code".into(),
                        winget_id: "Microsoft.VisualStudioCode".into(),
                        current_version: "1.94.0".into(),
                        available_version: "1.95.0".into(),
                        source: "winget".into(),
                        name_lower: "visual studio code".into(),
                        winget_id_lower: "microsoft.visualstudiocode".into(),
                    },
                    UpgradeablePackage {
                        name: "Git".into(),
                        winget_id: "Git.Git".into(),
                        current_version: "2.46.0".into(),
                        available_version: "2.47.0".into(),
                        source: "winget".into(),
                        name_lower: "git".into(),
                        winget_id_lower: "git.git".into(),
                    },
                ];

                let _ = sender
                    .send(ScanProgress::Log {
                        line: format!("[DRY RUN] Found {} fake upgradeable packages", fake.len()),
                    })
                    .await;

                let _ = sender
                    .send(ScanProgress::Completed { packages: fake })
                    .await;
                return;
            }

            let mut scan_args: Vec<&str> = vec!["upgrade"];
            if include_unknown {
                scan_args.push("--include-unknown");
            }

            let Ok(all_lines) = run_winget_scan(&scan_args, &mut sender, |e| match e {
                ScanEvent::Activity(line) => ScanProgress::Activity { line },
                ScanEvent::Log(line) => ScanProgress::Log { line },
                ScanEvent::Failed(error) => ScanProgress::Failed { error },
            })
            .await
            else {
                return;
            };

            let packages = parse_upgrade_table(&all_lines);
            let _ = sender.send(ScanProgress::Completed { packages }).await;
        },
    )
}

pub fn parse_upgrade_table(lines: &[String]) -> Vec<UpgradeablePackage> {
    let header_idx = lines.iter().position(|l| {
        l.contains("Name") && l.contains("Id") && l.contains("Version") && l.contains("Available")
    });

    let Some(header_idx) = header_idx else {
        return Vec::new();
    };

    let header = &lines[header_idx];

    let Some(name_col) = header.find("Name") else {
        return Vec::new();
    };
    let Some(id_col) = header.find("Id") else {
        return Vec::new();
    };
    let Some(version_col) = header.find("Version") else {
        return Vec::new();
    };
    let Some(available_col) = header.find("Available") else {
        return Vec::new();
    };
    let source_col = header.find("Source");
    let data_start = find_data_start(lines, header_idx);

    let mut packages = Vec::new();

    for line in &lines[data_start..] {
        if line.contains("upgrades available") || line.contains("upgrade(s) available") {
            continue;
        }

        if line.len() < available_col + 1 {
            continue;
        }

        let name = safe_slice(line, name_col, id_col);
        let id = safe_slice(line, id_col, version_col);
        let version = safe_slice(line, version_col, available_col);
        let (available, source) = if let Some(sc) = source_col {
            (
                safe_slice(line, available_col, sc),
                safe_slice_to_end(line, sc),
            )
        } else {
            (safe_slice_to_end(line, available_col), String::new())
        };

        if id.is_empty() || available.is_empty() {
            continue;
        }

        let name_lower = name.to_lowercase();
        let winget_id_lower = id.to_lowercase();
        packages.push(UpgradeablePackage {
            name,
            winget_id: id,
            current_version: version,
            available_version: available,
            source,
            name_lower,
            winget_id_lower,
        });
    }

    packages
}

/// Find the first data row after the header, skipping any separator line (dashes).
fn find_data_start(lines: &[String], header_idx: usize) -> usize {
    if header_idx + 1 >= lines.len() {
        return lines.len();
    }
    let sep_offset = lines[header_idx + 1..].iter().position(|l| {
        l.starts_with("---") || l.starts_with("───") || l.chars().all(|c| c == '-' || c == ' ')
    });
    match sep_offset {
        Some(offset) => header_idx + 2 + offset,
        None => header_idx + 1,
    }
}

fn snap_forward(s: &str, mut i: usize) -> usize {
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

fn snap_back(s: &str, mut i: usize) -> usize {
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn safe_slice(line: &str, start: usize, end: usize) -> String {
    let start = snap_forward(line, start.min(line.len()));
    let end = snap_back(line, end.min(line.len()));
    if start >= end {
        return String::new();
    }
    line[start..end].trim().to_string()
}

fn safe_slice_to_end(line: &str, start: usize) -> String {
    let start = snap_forward(line, start.min(line.len()));
    if start >= line.len() {
        return String::new();
    }
    line[start..].trim().to_string()
}

pub fn upgrade_all(
    packages: Vec<UpgradeablePackage>,
    dry_run: bool,
    extra_args: Vec<String>,
) -> impl futures::Stream<Item = InstallProgress> + Send {
    install::run_winget_batch(
        packages,
        "upgrade",
        vec!["--accept-package-agreements", "--accept-source-agreements"],
        dry_run,
        extra_args,
    )
}
