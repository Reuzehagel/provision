use iced::futures;
use iced::futures::SinkExt as _;
use iced::stream;
use tokio::process::Command;

use crate::install::{self, InstallProgress, LineEvent, Sender};

#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub winget_id: String,
    pub version: String,
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
                        winget_id: "Git.Git".into(),
                        version: "2.47.0".into(),
                    },
                    InstalledPackage {
                        winget_id: "Mozilla.Firefox".into(),
                        version: "131.0".into(),
                    },
                    InstalledPackage {
                        winget_id: "7zip.7zip".into(),
                        version: "24.08".into(),
                    },
                    InstalledPackage {
                        winget_id: "Microsoft.WindowsTerminal".into(),
                        version: "1.21.0".into(),
                    },
                    InstalledPackage {
                        winget_id: "Microsoft.VisualStudioCode".into(),
                        version: "1.95.0".into(),
                    },
                ];

                let _ = sender
                    .send(InstalledScanProgress::Completed { packages: fake })
                    .await;
                return;
            }

            let child = Command::new("winget")
                .args(["list"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .creation_flags(0x08000000)
                .spawn();

            let mut child = match child {
                Ok(c) => c,
                Err(e) => {
                    let _ = sender
                        .send(InstalledScanProgress::Failed {
                            error: format!("Failed to spawn winget: {e}"),
                        })
                        .await;
                    return;
                }
            };

            let mut all_lines: Vec<String> = Vec::new();

            if let Some(stdout) = child.stdout.take() {
                let all_lines = &mut all_lines;
                let result = install::read_stdout(stdout, &mut sender, |event| match event {
                    LineEvent::Log(line) => {
                        all_lines.push(line.clone());
                        InstalledScanProgress::Activity { line }
                    }
                    LineEvent::Activity(line) => InstalledScanProgress::Activity { line },
                })
                .await;

                if let Err(e) = result {
                    let _ = sender
                        .send(InstalledScanProgress::Failed { error: e })
                        .await;
                    return;
                }
            }

            let _ = child.wait().await;

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

    let version_end = header.find("Source").unwrap_or(usize::MAX);
    let data_start = find_data_start(lines, header_idx);

    let mut packages = Vec::new();

    for line in &lines[data_start..] {
        if line.len() < version_col + 1 {
            continue;
        }

        let id = safe_slice(line, id_col, version_col);
        let version = if version_end < usize::MAX {
            safe_slice(line, version_col, version_end)
        } else {
            safe_slice_to_end(line, version_col)
        };

        if id.is_empty() {
            continue;
        }

        packages.push(InstalledPackage {
            winget_id: id.to_lowercase(),
            version,
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
                    },
                    UpgradeablePackage {
                        name: "Visual Studio Code".into(),
                        winget_id: "Microsoft.VisualStudioCode".into(),
                        current_version: "1.94.0".into(),
                        available_version: "1.95.0".into(),
                        source: "winget".into(),
                    },
                    UpgradeablePackage {
                        name: "Git".into(),
                        winget_id: "Git.Git".into(),
                        current_version: "2.46.0".into(),
                        available_version: "2.47.0".into(),
                        source: "winget".into(),
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

            let mut scan_args = vec!["upgrade"];
            if include_unknown {
                scan_args.push("--include-unknown");
            }

            let child = Command::new("winget")
                .args(&scan_args)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .creation_flags(0x08000000)
                .spawn();

            let mut child = match child {
                Ok(c) => c,
                Err(e) => {
                    let _ = sender
                        .send(ScanProgress::Failed {
                            error: format!("Failed to spawn winget: {e}"),
                        })
                        .await;
                    return;
                }
            };

            let mut all_lines: Vec<String> = Vec::new();

            if let Some(stdout) = child.stdout.take() {
                let all_lines = &mut all_lines;
                let result = install::read_stdout(stdout, &mut sender, |event| match event {
                    LineEvent::Log(line) => {
                        all_lines.push(line.clone());
                        ScanProgress::Log { line }
                    }
                    LineEvent::Activity(line) => ScanProgress::Activity { line },
                })
                .await;

                if let Err(e) = result {
                    let _ = sender.send(ScanProgress::Failed { error: e }).await;
                    return;
                }
            }

            let _ = child.wait().await;

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

        packages.push(UpgradeablePackage {
            name,
            winget_id: id,
            current_version: version,
            available_version: available,
            source,
        });
    }

    packages
}

/// Find the first data row after the header, skipping any separator line (dashes).
fn find_data_start(lines: &[String], header_idx: usize) -> usize {
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
    stream::channel(100, move |mut sender: Sender| async move {
        for (i, pkg) in packages.iter().enumerate() {
            let _ = sender.send(InstallProgress::Started { index: i }).await;

            if dry_run {
                let _ = sender
                    .send(InstallProgress::Log {
                        index: i,
                        line: format!(
                            "[DRY RUN] Would run: winget upgrade --id {} -e",
                            pkg.winget_id
                        ),
                    })
                    .await;

                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                let _ = sender.send(InstallProgress::Succeeded { index: i }).await;
                continue;
            }

            let mut args: Vec<String> = vec![
                "upgrade".into(),
                "--id".into(),
                pkg.winget_id.clone(),
                "-e".into(),
                "--accept-package-agreements".into(),
                "--accept-source-agreements".into(),
            ];
            args.extend(extra_args.iter().cloned());

            match install::run_command("winget", &args, i, &mut sender).await {
                Ok(()) => {
                    let _ = sender.send(InstallProgress::Succeeded { index: i }).await;
                }
                Err(e) => {
                    let _ = sender
                        .send(InstallProgress::Failed { index: i, error: e })
                        .await;
                }
            }
        }
        let _ = sender.send(InstallProgress::Completed).await;
    })
}
