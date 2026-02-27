use iced::futures;
use iced::futures::SinkExt as _;
use iced::stream;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::catalog::Package;

pub(crate) type Sender = futures::channel::mpsc::Sender<InstallProgress>;

#[derive(Debug, Clone)]
pub enum PackageStatus {
    Pending,
    Installing,
    Done,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub enum InstallProgress {
    Started {
        index: usize,
    },
    /// Finalized line (after \n) — appended to terminal log
    Log {
        #[allow(dead_code)]
        index: usize,
        line: String,
    },
    /// Transient line (spinner, progress bar) — replaces the live line
    Activity {
        #[allow(dead_code)]
        index: usize,
        line: String,
    },
    Succeeded {
        index: usize,
    },
    Failed {
        index: usize,
        error: String,
    },
    Completed,
}

/// A parsed line from process stdout, classified by its line ending.
#[derive(Debug, Clone)]
pub(crate) enum LineEvent {
    /// Finalized line (terminated by \n or \r\n).
    Log(String),
    /// Transient line (terminated by bare \r) — overwrites the live line.
    Activity(String),
}

pub fn install_all(
    packages: Vec<Package>,
    dry_run: bool,
    extra_args: Vec<String>,
) -> impl futures::Stream<Item = InstallProgress> + Send {
    stream::channel(100, move |mut sender: Sender| async move {
        for (i, pkg) in packages.iter().enumerate() {
            let _ = sender.send(InstallProgress::Started { index: i }).await;

            if dry_run {
                let cmd_desc = if let Some(ref custom) = pkg.install_command {
                    format!("cmd /C {custom}")
                } else if let Some(ref winget_id) = pkg.winget_id {
                    format!("winget install --id {winget_id} -e")
                } else {
                    "No install method defined".into()
                };

                let _ = sender
                    .send(InstallProgress::Log {
                        index: i,
                        line: format!("[DRY RUN] Would run: {cmd_desc}"),
                    })
                    .await;

                if let Some(ref post) = pkg.post_install {
                    let _ = sender
                        .send(InstallProgress::Log {
                            index: i,
                            line: format!("[DRY RUN] Would run post-install: {post}"),
                        })
                        .await;
                }

                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                let _ = sender.send(InstallProgress::Succeeded { index: i }).await;
                continue;
            }

            let (program, args) = if let Some(ref custom) = pkg.install_command {
                ("cmd".to_string(), vec!["/C".to_string(), custom.clone()])
            } else if let Some(ref winget_id) = pkg.winget_id {
                let mut a = vec![
                    "install".into(),
                    "--id".into(),
                    winget_id.clone(),
                    "-e".into(),
                    "--accept-package-agreements".into(),
                    "--accept-source-agreements".into(),
                ];
                a.extend(extra_args.iter().cloned());
                ("winget".to_string(), a)
            } else {
                let _ = sender
                    .send(InstallProgress::Failed {
                        index: i,
                        error: "No install method defined".into(),
                    })
                    .await;
                continue;
            };

            match run_command(&program, &args, i, &mut sender).await {
                Ok(()) => {
                    if let Some(ref post) = pkg.post_install {
                        let _ = sender
                            .send(InstallProgress::Log {
                                index: i,
                                line: format!("Running post-install: {post}"),
                            })
                            .await;
                        let post_args = vec!["/C".to_string(), post.clone()];
                        if let Err(e) = run_command("cmd", &post_args, i, &mut sender).await {
                            let _ = sender
                                .send(InstallProgress::Log {
                                    index: i,
                                    line: format!("Post-install warning: {e}"),
                                })
                                .await;
                        }
                    }
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

/// Returns true if a line is transient terminal output (spinners, progress bars, etc.)
/// that should overwrite the live line rather than be appended to the log.
fn is_transient(line: &str) -> bool {
    // Spinner characters
    if line.len() <= 2
        && line
            .chars()
            .all(|c| matches!(c, '\\' | '|' | '/' | '-' | '_'))
    {
        return true;
    }

    // Lines with progress bar block characters (▓░▒█)
    if line.contains(['\u{2588}', '\u{2591}', '\u{2592}', '\u{2593}']) {
        return true;
    }

    // Bare percentage lines like "28%" or "100%"
    let no_pct = line.trim_end_matches('%');
    if no_pct != line && no_pct.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }

    // Download progress lines like "1024 KB / 48.2 MB"
    if (line.contains(" KB / ") || line.contains(" MB / ") || line.contains(" GB / "))
        && line.bytes().next().is_some_and(|b| b.is_ascii_digit())
    {
        return true;
    }

    false
}

/// Convert raw bytes to a trimmed string, returning `None` if empty.
fn trimmed_lossy(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

/// Read raw bytes from an `AsyncRead`, parse CR/LF boundaries, strip ANSI
/// escapes, and send each classified `LineEvent` through the provided sender.
/// Shared by both the install and upgrade flows.
pub(crate) async fn read_stdout<T: Send>(
    stdout: impl tokio::io::AsyncRead + Unpin,
    sender: &mut futures::channel::mpsc::Sender<T>,
    mut map_event: impl FnMut(LineEvent) -> T,
) -> Result<(), String> {
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut buf = [0u8; 1024];
    let mut current_line: Vec<u8> = Vec::new();
    let mut in_escape = false;
    let mut after_cr = false;

    loop {
        let n = reader
            .read(&mut buf)
            .await
            .map_err(|e| format!("Read error: {e}"))?;
        if n == 0 {
            break;
        }

        for &b in &buf[..n] {
            if after_cr {
                after_cr = false;
                if b == b'\n' {
                    if let Some(text) = trimmed_lossy(&current_line) {
                        let _ = sender.send(map_event(LineEvent::Log(text))).await;
                    }
                    current_line.clear();
                    continue;
                } else {
                    if let Some(text) = trimmed_lossy(&current_line) {
                        let _ = sender.send(map_event(LineEvent::Activity(text))).await;
                    }
                    current_line.clear();
                }
            }

            if in_escape {
                if b.is_ascii_alphabetic() || b == b'~' {
                    in_escape = false;
                }
                continue;
            }

            match b {
                b'\x1b' => in_escape = true,
                b'\r' => after_cr = true,
                b'\n' => {
                    if let Some(text) = trimmed_lossy(&current_line) {
                        let _ = sender.send(map_event(LineEvent::Log(text))).await;
                    }
                    current_line.clear();
                }
                _ => current_line.push(b),
            }
        }
    }

    if !current_line.is_empty()
        && let Some(text) = trimmed_lossy(&current_line)
    {
        let _ = sender.send(map_event(LineEvent::Log(text))).await;
    }

    Ok(())
}

pub(crate) async fn run_command(
    program: &str,
    args: &[String],
    index: usize,
    sender: &mut Sender,
) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| format!("Failed to spawn: {e}"))?;

    if let Some(stdout) = child.stdout.take() {
        read_stdout(stdout, sender, |event| match event {
            LineEvent::Log(line) if is_transient(&line) => {
                InstallProgress::Activity { index, line }
            }
            LineEvent::Log(line) => InstallProgress::Log { index, line },
            LineEvent::Activity(line) => InstallProgress::Activity { index, line },
        })
        .await?;
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Wait failed: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Exit code: {}", status.code().unwrap_or(-1)))
    }
}
