# Uninstall Support Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a full package uninstall flow — browse installed packages, select, confirm, uninstall with streaming progress, and async registry size lookups.

**Architecture:** Three new screens (UninstallSelect, UninstallReview, Uninstalling) following the existing Elm-style scan-select-execute pattern. New `uninstall.rs` module for execution logic. Enriched `InstalledPackage` struct with original-case IDs and optional sizes. Background registry scan for package sizes.

**Tech Stack:** Rust 1.85+, Iced 0.14, tokio, winget CLI, Windows registry via `windows-sys`

**Spec:** `docs/superpowers/specs/2026-03-11-uninstall-support-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src/upgrade.rs` | Modify | Enrich `InstalledPackage` struct, update `parse_list_table()` to extract all columns and preserve original-case IDs |
| `src/uninstall.rs` | Create | `uninstall_all()` stream, `scan_sizes()` registry lookup |
| `src/main.rs` | Modify | New `Screen` variants, `Message` variants, App state fields, handler methods, view dispatch, subscription |
| `src/views.rs` | Modify | `view_uninstall_select()`, `view_uninstall_review()`, `view_uninstalling()`, home screen "Uninstall" card |
| `src/settings.rs` | Modify | Add `uninstall_args()` method |
| `src/styles.rs` | Modify | Add `danger_button_style()` for red accent buttons |

---

## Chunk 1: Data Layer — InstalledPackage & Parser

### Task 1: Enrich `InstalledPackage` struct

**Files:**
- Modify: `src/upgrade.rs:8-12`

- [ ] **Step 1: Update the `InstalledPackage` struct**

Replace the existing struct at lines 8-12:

```rust
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub name: String,
    pub winget_id: String,         // original case for winget uninstall --id
    pub version: String,
    pub source: String,
    pub winget_id_lower: String,   // precomputed for search & is_installed()
    pub name_lower: String,        // precomputed for search
    pub size_bytes: Option<u64>,   // filled async from registry
}
```

- [ ] **Step 2: Fix compilation errors from struct change**

The `InstalledScanProgress::Completed` variant at line 20 uses `Vec<InstalledPackage>` — that's fine, no change needed.

The dry-run fake data in `scan_installed()` (around line 55-90) creates `InstalledPackage` instances. Update them:

```rust
// Find the existing fake package pushes and update to include all fields.
// Example pattern for each fake package:
InstalledPackage {
    name: "Visual Studio Code".into(),
    winget_id: "Microsoft.VisualStudioCode".into(),
    version: "1.96.2".into(),
    source: "winget".into(),
    winget_id_lower: "microsoft.visualstudiocode".into(),
    name_lower: "visual studio code".into(),
    size_bytes: None,
}
```

Update every fake `InstalledPackage` in the dry-run block to include the new fields.

- [ ] **Step 3: Run `just check`**

Run: `just check`
Expected: May fail — `parse_list_table()` still constructs the old struct, and `main.rs` consumes fields that changed. We fix that next.

### Task 2: Update `parse_list_table()` to extract all columns

**Files:**
- Modify: `src/upgrade.rs:143-189`

- [ ] **Step 1: Update `parse_list_table()` to extract Name, Source, and preserve original-case ID**

Replace the function body (lines 143-189):

```rust
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
```

- [ ] **Step 2: Run `just check`**

Run: `just check`
Expected: Will fail — `main.rs` still uses the old `HashMap<String, String>` for installed. We fix that next.

### Task 3: Update `App` state for enriched installed data

**Files:**
- Modify: `src/main.rs:229-231` (installed field), `src/main.rs:523-542` (scan handler), `src/main.rs:27-31` (is_installed)

- [ ] **Step 1: Change `App::installed` field**

In the `App` struct (around line 229), replace:
```rust
/// Installed packages detected at startup: winget_id (lowercase) -> version
pub(crate) installed: HashMap<String, String>,
```
with:
```rust
/// Full installed package data from winget list (for uninstall screen)
pub(crate) installed_packages: Vec<upgrade::InstalledPackage>,
/// Installed packages: winget_id (lowercase) -> version (for O(1) is_installed lookups)
pub(crate) installed_map: HashMap<String, String>,
```

- [ ] **Step 2: Update the Default/initialization**

In `App::new()` (around line 270), the `installed` field initialization changes:
```rust
installed_packages: Vec::new(),
installed_map: HashMap::new(),
```

- [ ] **Step 3: Update `handle_installed_scan_progress()`**

At lines 523-542, replace the `Completed` arm:
```rust
upgrade::InstalledScanProgress::Completed { packages } => {
    self.installed_map = packages
        .iter()
        .map(|p| (p.winget_id_lower.clone(), p.version.clone()))
        .collect();
    self.installed_packages = packages;
    self.installed_scan_done = true;
    self._installed_scan_handle = None;
}
```

- [ ] **Step 4: Update `is_installed()`**

At lines 27-31, change `self.installed` to `self.installed_map`:
```rust
pub(crate) fn is_installed(&self, pkg: &Package) -> bool {
    pkg.winget_id_lower
        .as_ref()
        .is_some_and(|wid| self.installed_map.contains_key(wid))
}
```

- [ ] **Step 5: Fix all remaining references to `self.installed`**

Search for `self.installed` in `main.rs` and `views.rs`. Common occurrences:
- `self.installed.len()` → `self.installed_map.len()` (in views for count display)
- `self.installed.contains_key()` → `self.installed_map.contains_key()`

- [ ] **Step 6: Run `just check`**

Run: `just check`
Expected: PASS — all existing functionality preserved with the renamed fields.

- [ ] **Step 7: Commit**

```bash
git add src/upgrade.rs src/main.rs src/views.rs
git commit -m "Enrich InstalledPackage struct and update parser for uninstall support"
```

---

## Chunk 2: Uninstall Execution — `uninstall.rs` & Settings

### Task 4: Add `uninstall_args()` to `WingetSettings`

**Files:**
- Modify: `src/settings.rs:149-187`

- [ ] **Step 1: Add `uninstall_args()` method**

After the existing `install_args()` method (after line 187), add:

```rust
/// Build extra CLI flags for uninstall commands.
/// Only a subset of install flags apply to uninstall.
pub fn uninstall_args(&self) -> Vec<String> {
    let mut args = Vec::new();

    match self.install_mode {
        InstallMode::Silent => args.push("--silent".into()),
        InstallMode::Interactive => args.push("--interactive".into()),
    }

    if self.force {
        args.push("--force".into());
    }

    if self.disable_interactivity {
        args.push("--disable-interactivity".into());
    }

    args
}
```

- [ ] **Step 2: Run `just check`**

Run: `just check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/settings.rs
git commit -m "Add uninstall_args() for uninstall-specific winget flags"
```

### Task 5: Create `uninstall.rs` with `uninstall_all()`

**Files:**
- Create: `src/uninstall.rs`
- Modify: `src/main.rs` (add `mod uninstall;`)

- [ ] **Step 1: Create `src/uninstall.rs`**

```rust
use iced::futures;
use iced::futures::SinkExt as _;
use iced::stream;

use crate::install::{self, InstallProgress, Sender};
use crate::upgrade::InstalledPackage;

pub fn uninstall_all(
    packages: Vec<InstalledPackage>,
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
                            "[DRY RUN] Would run: winget uninstall --id {} -e",
                            pkg.winget_id
                        ),
                    })
                    .await;

                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                let _ = sender.send(InstallProgress::Succeeded { index: i }).await;
                continue;
            }

            let mut args: Vec<String> = vec![
                "uninstall".into(),
                "--id".into(),
                pkg.winget_id.clone(),
                "-e".into(),
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
```

- [ ] **Step 2: Add `mod uninstall;` to `main.rs`**

Near the top of `main.rs`, alongside the other module declarations (around line 12-17), add:
```rust
mod uninstall;
```

- [ ] **Step 3: Run `just check`**

Run: `just check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/uninstall.rs src/main.rs
git commit -m "Add uninstall module with uninstall_all() execution stream"
```

### Task 6: Add `scan_sizes()` registry lookup

**Files:**
- Modify: `src/uninstall.rs`

- [ ] **Step 1: Add `scan_sizes()` function**

Add to `src/uninstall.rs`:

```rust
use crate::upgrade::InstalledPackage;

/// Scan Windows registry for installed package sizes.
/// Returns (winget_id_lower, size_in_bytes) pairs.
pub async fn scan_sizes(packages: Vec<InstalledPackage>) -> Vec<(String, u64)> {
    tokio::task::spawn_blocking(move || scan_sizes_blocking(&packages))
        .await
        .unwrap_or_default()
}

fn scan_sizes_blocking(packages: &[InstalledPackage]) -> Vec<(String, u64)> {
    use windows_sys::Win32::System::Registry::*;

    let mut results = Vec::new();
    let mut matched: std::collections::HashSet<String> = std::collections::HashSet::new();

    let reg_paths: &[(&str, HKEY)] = &[
        (
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            HKEY_LOCAL_MACHINE,
        ),
        (
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            HKEY_CURRENT_USER,
        ),
        (
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            HKEY_LOCAL_MACHINE,
        ),
    ];

    for &(subkey, hive) in reg_paths {
        if let Some(entries) = read_uninstall_key(hive, subkey) {
            for (display_name, size_kb) in entries {
                let display_lower = display_name.to_lowercase();
                // Try to match against known packages
                for pkg in packages {
                    if matched.contains(&pkg.winget_id_lower) {
                        continue; // already matched from another hive
                    }
                    // Exact name match (case-insensitive)
                    if display_lower == pkg.name_lower {
                        results.push((pkg.winget_id_lower.clone(), size_kb * 1024));
                        matched.insert(pkg.winget_id_lower.clone());
                        break;
                    }
                    // ID segment match: check if display name contains the
                    // last segment of the winget ID (e.g. "Git" from "Git.Git")
                    let id_segment = pkg.winget_id.rsplit('.').next().unwrap_or("");
                    if !id_segment.is_empty()
                        && id_segment.len() >= 3
                        && display_lower.contains(&id_segment.to_lowercase())
                    {
                        results.push((pkg.winget_id_lower.clone(), size_kb * 1024));
                        matched.insert(pkg.winget_id_lower.clone());
                        break;
                    }
                }
            }
        }
    }

    results
}

/// Read DisplayName and EstimatedSize from all subkeys of a registry Uninstall key.
fn read_uninstall_key(hive: HKEY, subkey: &str) -> Option<Vec<(String, u64)>> {
    use windows_sys::Win32::System::Registry::*;

    let subkey_wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let mut hkey: HKEY = 0;

    let status = unsafe {
        RegOpenKeyExW(
            hive,
            subkey_wide.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        )
    };
    if status != 0 {
        return None;
    }

    let mut entries = Vec::new();
    let mut index = 0u32;

    loop {
        let mut name_buf = [0u16; 256];
        let mut name_len = name_buf.len() as u32;

        let status = unsafe {
            RegEnumKeyExW(
                hkey,
                index,
                name_buf.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        if status != 0 {
            break;
        }

        let subkey_name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
        let full_path = format!("{subkey}\\{subkey_name}");
        let full_wide: Vec<u16> = full_path.encode_utf16().chain(std::iter::once(0)).collect();

        let mut entry_key: HKEY = 0;
        let open_status = unsafe {
            RegOpenKeyExW(hive, full_wide.as_ptr(), 0, KEY_READ, &mut entry_key)
        };

        if open_status == 0 {
            let display_name = read_reg_string(entry_key, "DisplayName");
            let size = read_reg_dword(entry_key, "EstimatedSize");
            unsafe { RegCloseKey(entry_key) };

            if let (Some(name), Some(size_kb)) = (display_name, size) {
                if !name.is_empty() && size_kb > 0 {
                    entries.push((name, size_kb as u64));
                }
            }
        }

        index += 1;
    }

    unsafe { RegCloseKey(hkey) };
    Some(entries)
}

fn read_reg_string(hkey: isize, value_name: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::*;

    let name_wide: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut buf = [0u16; 512];
    let mut buf_size = (buf.len() * 2) as u32;
    let mut reg_type = 0u32;

    let status = unsafe {
        RegQueryValueExW(
            hkey,
            name_wide.as_ptr(),
            std::ptr::null_mut(),
            &mut reg_type,
            buf.as_mut_ptr().cast(),
            &mut buf_size,
        )
    };

    if status != 0 || reg_type != REG_SZ {
        return None;
    }

    let len = (buf_size as usize / 2).saturating_sub(1); // exclude null terminator
    Some(String::from_utf16_lossy(&buf[..len]))
}

fn read_reg_dword(hkey: isize, value_name: &str) -> Option<u32> {
    use windows_sys::Win32::System::Registry::*;

    let name_wide: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut value: u32 = 0;
    let mut buf_size = std::mem::size_of::<u32>() as u32;
    let mut reg_type = 0u32;

    let status = unsafe {
        RegQueryValueExW(
            hkey,
            name_wide.as_ptr(),
            std::ptr::null_mut(),
            &mut reg_type,
            (&mut value as *mut u32).cast(),
            &mut buf_size,
        )
    };

    if status != 0 || reg_type != REG_DWORD {
        return None;
    }

    Some(value)
}
```

- [ ] **Step 2: Add `Win32_System_Registry` feature to `windows-sys` dependency**

In `Cargo.toml`, update the `windows-sys` dependency:
```toml
windows-sys = { version = "0.61", features = ["Win32_UI_Shell", "Win32_Foundation", "Win32_System_Registry"] }
```

- [ ] **Step 3: Run `just check`**

Run: `just check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/uninstall.rs Cargo.toml Cargo.lock
git commit -m "Add scan_sizes() for async registry-based package size lookup"
```

---

## Chunk 3: App State, Messages & Handlers

### Task 7: Add Screen variants, Messages, and App state

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add Screen variants**

At the `Screen` enum (around line 301-312), add three new variants before the closing brace:

```rust
UninstallSelect,
UninstallReview,
Uninstalling,
```

- [ ] **Step 2: Add Message variants**

At the `Message` enum (around line 314-364), add:

```rust
GoToUninstall,
ToggleUninstallPackage(String),
GoToUninstallReview,
StartUninstall,
CancelUninstall,
UninstallProgress(install::InstallProgress),
FinishUninstallAndReset,
SizeScanResult(Vec<(String, u64)>),
```

- [ ] **Step 3: Add App state fields**

In the `App` struct (around line 217-247), add after the upgrade fields:

```rust
// Uninstall state
pub(crate) uninstall_selected: HashSet<String>,
pub(crate) uninstall_queue: Vec<upgrade::InstalledPackage>,
pub(crate) uninstall: ProgressState,
pub(crate) size_scan_done: bool,
```

- [ ] **Step 4: Initialize the new fields in `App::new()`**

In the struct initialization (around line 264-298), add:

```rust
uninstall_selected: HashSet::new(),
uninstall_queue: Vec::new(),
uninstall: ProgressState::default(),
size_scan_done: false,
```

- [ ] **Step 5: Run `just check`**

Run: `just check`
Expected: Will warn about unused variants but should compile. We add handlers next.

### Task 8: Add handler methods for uninstall flow

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add uninstall handler methods**

Add these methods to the `impl App` block, near the existing upgrade handlers:

```rust
fn handle_go_to_uninstall(&mut self) -> Task<Message> {
    self.search.clear();
    self.uninstall_selected.clear();
    self.screen = Screen::UninstallSelect;
    Task::none()
}

fn handle_toggle_uninstall_package(&mut self, winget_id_lower: String) -> Task<Message> {
    if !self.uninstall_selected.remove(&winget_id_lower) {
        self.uninstall_selected.insert(winget_id_lower);
    }
    Task::none()
}

fn handle_go_to_uninstall_review(&mut self) -> Task<Message> {
    self.screen = Screen::UninstallReview;
    Task::none()
}

fn handle_start_uninstall(&mut self) -> Task<Message> {
    let queue: Vec<upgrade::InstalledPackage> = self
        .installed_packages
        .iter()
        .filter(|p| self.uninstall_selected.contains(&p.winget_id_lower))
        .cloned()
        .collect();

    if queue.is_empty() {
        return Task::none();
    }

    self.uninstall.start(queue.len());
    self.uninstall_queue = queue.clone();
    self.screen = Screen::Uninstalling;

    let dry = self.dry_run;
    let extra = self.settings.uninstall_args();
    let (task, handle) = Task::run(
        uninstall::uninstall_all(queue, dry, extra),
        Message::UninstallProgress,
    )
    .abortable();

    self.uninstall._handle = Some(handle.abort_on_drop());
    task
}

fn handle_cancel_uninstall(&mut self) -> Task<Message> {
    self.uninstall.cancel("Uninstall");
    Task::none()
}

fn handle_uninstall_progress(
    &mut self,
    event: install::InstallProgress,
) -> Task<Message> {
    let queue = &self.uninstall_queue;
    self.uninstall.handle_event(&event, |i| {
        let name = queue.get(i).map(|p| p.name.as_str()).unwrap_or("...");
        format!("Removing {name}")
    });
    Task::none()
}

fn handle_finish_uninstall_and_reset(&mut self) -> Task<Message> {
    self.search.clear();
    self.uninstall_selected.clear();
    self.uninstall_queue.clear();
    self.uninstall = ProgressState::default();
    self.screen = Screen::ProfileSelect;

    // Re-scan installed packages
    let (task, handle) = Task::run(
        upgrade::scan_installed(self.dry_run),
        Message::InstalledScanProgress,
    )
    .abortable();
    self.installed_scan_done = false;
    self._installed_scan_handle = Some(handle.abort_on_drop());
    task
}

fn handle_size_scan_result(&mut self, sizes: Vec<(String, u64)>) -> Task<Message> {
    for (id_lower, size) in sizes {
        if let Some(pkg) = self
            .installed_packages
            .iter_mut()
            .find(|p| p.winget_id_lower == id_lower)
        {
            pkg.size_bytes = Some(size);
        }
    }
    self.size_scan_done = true;
    Task::none()
}
```

- [ ] **Step 2: Wire messages in the `update()` dispatcher**

In the `update()` method (the big match on `Message`), add arms for each new message:

```rust
Message::GoToUninstall => self.handle_go_to_uninstall(),
Message::ToggleUninstallPackage(id) => self.handle_toggle_uninstall_package(id),
Message::GoToUninstallReview => self.handle_go_to_uninstall_review(),
Message::StartUninstall => self.handle_start_uninstall(),
Message::CancelUninstall => self.handle_cancel_uninstall(),
Message::UninstallProgress(event) => self.handle_uninstall_progress(event),
Message::FinishUninstallAndReset => self.handle_finish_uninstall_and_reset(),
Message::SizeScanResult(sizes) => self.handle_size_scan_result(sizes),
```

- [ ] **Step 3: Update `handle_go_back()` for uninstall screens**

Add arms to the `handle_go_back()` match (around lines 560-582):

```rust
Screen::UninstallSelect => {
    self.screen = Screen::ProfileSelect;
}
Screen::UninstallReview => {
    self.screen = Screen::UninstallSelect;
}
```

- [ ] **Step 4: Update `handle_installed_scan_progress()` to kick off size scan**

In the `Completed` arm of `handle_installed_scan_progress()`, after storing packages, kick off the size scan:

```rust
upgrade::InstalledScanProgress::Completed { packages } => {
    self.installed_map = packages
        .iter()
        .map(|p| (p.winget_id_lower.clone(), p.version.clone()))
        .collect();
    self.installed_packages = packages;
    self.installed_scan_done = true;
    self._installed_scan_handle = None;

    // Kick off background size scan
    let pkgs = self.installed_packages.clone();
    return Task::perform(
        uninstall::scan_sizes(pkgs),
        Message::SizeScanResult,
    );
}
```

- [ ] **Step 5: Update `view()` dispatch**

Add the new screen variants to the `view()` match (around line 934-945):

```rust
Screen::UninstallSelect => self.view_uninstall_select(),
Screen::UninstallReview => self.view_uninstall_review(),
Screen::Uninstalling => self.view_uninstalling(),
```

- [ ] **Step 6: Update `subscription()` for spinner during uninstall**

In the `spinner_active` condition (around line 963), add:

```rust
|| matches!(self.screen, Screen::Uninstalling if !self.uninstall.done)
```

- [ ] **Step 7: Update `SelectAll` handler for uninstall screen**

In the `SelectAll` message handler, add a case for `UninstallSelect`:

```rust
Screen::UninstallSelect => {
    let search = self.search.to_lowercase();
    let filtered: Vec<String> = self
        .installed_packages
        .iter()
        .filter(|p| {
            search.is_empty()
                || p.name_lower.contains(&search)
                || p.winget_id_lower.contains(&search)
        })
        .map(|p| p.winget_id_lower.clone())
        .collect();
    toggle_set(&mut self.uninstall_selected, filtered);
    Task::none()
}
```

- [ ] **Step 8: Update `KeyConfirm` handler**

Add uninstall screen behavior in the `KeyConfirm` handler:

```rust
Screen::UninstallSelect if !self.uninstall_selected.is_empty() => {
    self.handle_go_to_uninstall_review()
}
Screen::UninstallReview => self.handle_start_uninstall(),
Screen::Uninstalling if self.uninstall.done => {
    self.handle_finish_uninstall_and_reset()
}
```

- [ ] **Step 9: Update `KeyEscape` handler**

Add uninstall screen behavior:

```rust
Screen::UninstallSelect | Screen::UninstallReview => self.handle_go_back(),
Screen::Uninstalling if !self.uninstall.done => self.handle_cancel_uninstall(),
```

- [ ] **Step 10: Update `CopyLog` and `ClearCopyStatus` handlers for uninstall**

In the `handle_copy_log()` method (around line 843), update the screen match to route `Uninstalling`:

```rust
let state = match self.screen {
    Screen::Updating => &self.upgrade,
    Screen::Uninstalling => &self.uninstall,
    _ => &self.install,
};
```

And for setting `copy_status`:
```rust
match self.screen {
    Screen::Updating => self.upgrade.copy_status = true,
    Screen::Uninstalling => self.uninstall.copy_status = true,
    _ => self.install.copy_status = true,
}
```

In the `ClearCopyStatus` handler (around line 455), add:
```rust
self.uninstall.copy_status = false;
```

- [ ] **Step 11: Run `just check`**

Run: `just check`
Expected: Will fail — view methods don't exist yet. That's Task 9-11.

- [ ] **Step 12: Commit (if it compiles; otherwise commit with views)**

If it doesn't compile due to missing view methods, add stub methods that return placeholder elements first:

```rust
// In views.rs, add temporary stubs:
pub(crate) fn view_uninstall_select(&self) -> Element<'_, Message> {
    text("Uninstall select").into()
}
pub(crate) fn view_uninstall_review(&self) -> Element<'_, Message> {
    text("Uninstall review").into()
}
pub(crate) fn view_uninstalling(&self) -> Element<'_, Message> {
    text("Uninstalling").into()
}
```

```bash
git add src/main.rs src/views.rs
git commit -m "Add uninstall screen variants, messages, and handler logic"
```

---

## Chunk 4: View Layer

### Task 9: Add danger button style

**Files:**
- Modify: `src/styles.rs`

- [ ] **Step 1: Add `danger_button_style` function**

Add alongside the existing button style functions:

```rust
/// Red-accent button for destructive actions (uninstall).
pub fn danger_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text_color, border_color) = match status {
        button::Status::Active => (STATUS_RED, TEXT, STATUS_RED),
        button::Status::Hovered => (
            Color::from_rgb(0xdc as f32 / 255.0, 0x26 as f32 / 255.0, 0x26 as f32 / 255.0),
            TEXT,
            Color::from_rgb(0xdc as f32 / 255.0, 0x26 as f32 / 255.0, 0x26 as f32 / 255.0),
        ),
        _ => (STATUS_RED, TEXT, STATUS_RED),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}
```

- [ ] **Step 2: Run `just check`**

Run: `just check`
Expected: PASS (may warn about unused function — that's fine until views use it)

### Task 10: Add home screen "Uninstall packages" card

**Files:**
- Modify: `src/views.rs` (in `view_profile_select()`)

- [ ] **Step 1: Add uninstall card to home screen**

In `view_profile_select()`, after the "Check for updates" card (around line 108), add a similar card for uninstall:

```rust
// Uninstall card — same pattern as update_card
let uninstall_icon = text(char::from(Icon::Trash2))
    .size(15)
    .font(LUCIDE_FONT)
    .color(MUTED);
let uninstall_text = text("Uninstall packages").size(14).color(MUTED_FG);
let uninstall_chevron = text(char::from(Icon::ChevronRight))
    .size(14)
    .font(LUCIDE_FONT)
    .color(MUTED);

let uninstall_content = row![
    uninstall_icon,
    uninstall_text,
    iced::widget::Space::new().width(Length::Fill),
    uninstall_chevron,
]
.spacing(12)
.align_y(iced::Alignment::Center)
.padding([14, 16])
.width(Length::Fill);

let uninstall_card = if self.installed_scan_done && !self.installed_packages.is_empty() {
    button(uninstall_content)
        .on_press(Message::GoToUninstall)
        .width(Length::Fill)
        .style(update_card_style)
} else {
    button(uninstall_content)
        .width(Length::Fill)
        .style(update_card_style)
};
```

Then add `uninstall_card` to the layout column alongside the existing `update_card`.

- [ ] **Step 2: Run `just check`**

Run: `just check`
Expected: PASS

### Task 11: Implement `view_uninstall_select()`

**Files:**
- Modify: `src/views.rs`

- [ ] **Step 1: Add size formatting helper**

Add a standalone helper function:

```rust
fn format_size(bytes: Option<u64>) -> String {
    match bytes {
        None => "\u{2014}".into(), // em dash
        Some(b) if b < 1024 => format!("{b} B"),
        Some(b) if b < 1024 * 1024 => format!("{:.0} KB", b as f64 / 1024.0),
        Some(b) if b < 1024 * 1024 * 1024 => format!("{:.0} MB", b as f64 / (1024.0 * 1024.0)),
        Some(b) => format!("{:.1} GB", b as f64 / (1024.0 * 1024.0 * 1024.0)),
    }
}
```

- [ ] **Step 2: Implement `view_uninstall_select()`**

Replace the stub with the full implementation. This follows the `view_update_select()` pattern but uses a table layout:

```rust
pub(crate) fn view_uninstall_select(&self) -> Element<'_, Message> {
    use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input};
    use iced::{padding, Length};
    use crate::styles::*;

    let search = self.search.to_lowercase();

    let mut filtered: Vec<&upgrade::InstalledPackage> = self
        .installed_packages
        .iter()
        .filter(|p| {
            search.is_empty()
                || p.name_lower.contains(&search)
                || p.winget_id_lower.contains(&search)
        })
        .collect();

    // Sort alphabetically by name (precomputed lowercase), fallback to winget_id
    filtered.sort_by(|a, b| {
        let a_key = if a.name_lower.is_empty() { &a.winget_id_lower } else { &a.name_lower };
        let b_key = if b.name_lower.is_empty() { &b.winget_id_lower } else { &b.name_lower };
        a_key.cmp(b_key)
    });

    // Header with back button and search
    let back_btn = button(
        text(char::from(lucide_icons::Icon::ChevronLeft))
            .size(16)
            .font(LUCIDE_FONT),
    )
    .on_press(Message::GoBack)
    .style(ghost_button_style);

    let heading = text("Uninstall packages").size(16).color(TEXT);

    let search_input = text_input("Search installed...", &self.search)
        .on_input(Message::SearchChanged)
        .size(13)
        .width(200);

    let header = row![
        back_btn,
        heading,
        iced::widget::Space::new().width(Length::Fill),
        search_input,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .padding([16, 20]);

    // Column headers
    let col_headers = row![
        iced::widget::Space::new().width(38),
        text("Name").size(11).color(MUTED),
        iced::widget::Space::new().width(Length::Fill),
        container(text("Version").size(11).color(MUTED)).width(100),
        container(text("Size").size(11).color(MUTED)).width(80),
        container(text("Package ID").size(11).color(MUTED)).width(160),
    ]
    .spacing(0)
    .align_y(iced::Alignment::Center)
    .padding([8, 20]);

    // Package rows
    let mut pkg_list = column![].spacing(1);

    for pkg in &filtered {
        let is_checked = self.uninstall_selected.contains(&pkg.winget_id_lower);
        let id = pkg.winget_id_lower.clone();

        let cb = checkbox(is_checked)
            .on_toggle(move |_| Message::ToggleUninstallPackage(id.clone()))
            .size(14)
            .style(package_checkbox_style);

        let display_name = if pkg.name.is_empty() { &pkg.winget_id } else { &pkg.name };

        let row_content = row![
            container(cb).width(38).center_y(Length::Fill),
            text(display_name).size(13).color(TEXT).width(Length::Fill),
            container(text(&pkg.version).size(12).color(MUTED_FG)).width(100),
            container(text(format_size(pkg.size_bytes)).size(12).color(MUTED_FG)).width(80),
            container(text(&pkg.winget_id).size(11).color(MUTED)).width(160),
        ]
        .spacing(0)
        .align_y(iced::Alignment::Center)
        .padding([8, 0]);

        let row_bg = if is_checked {
            container(row_content).style(|_: &_| container::Style {
                background: Some(iced::Background::Color(CARD_BG)),
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
        } else {
            container(row_content)
        };

        pkg_list = pkg_list.push(row_bg);
    }

    let subtitle = if search.is_empty() {
        format!("{} installed packages", filtered.len())
    } else {
        format!("{} of {} installed packages", filtered.len(), self.installed_packages.len())
    };

    let scrollable_list = scrollable(
        pkg_list
            .width(Length::Fill)
            .padding(padding::right(20)),
    );

    // Footer
    let selected_count = self.uninstall_selected.len();
    let selected_size: u64 = self
        .installed_packages
        .iter()
        .filter(|p| self.uninstall_selected.contains(&p.winget_id_lower))
        .filter_map(|p| p.size_bytes)
        .sum();

    let footer_label = if selected_size > 0 {
        format!("{selected_count} selected \u{00b7} ~{}", format_size(Some(selected_size)))
    } else {
        format!("{selected_count} selected")
    };

    let footer_text = text(footer_label).size(12).color(MUTED_FG);

    let review_btn = if selected_count > 0 {
        button(
            row![
                text("Review uninstall").size(13),
                text(char::from(lucide_icons::Icon::ChevronRight))
                    .size(13)
                    .font(LUCIDE_FONT),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::GoToUninstallReview)
        .style(danger_button_style)
    } else {
        button(text("Review uninstall").size(13))
            .style(danger_button_style)
    };

    let footer = row![
        footer_text,
        iced::widget::Space::new().width(Length::Fill),
        review_btn,
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center)
    .padding([12, 20]);

    // Compose
    let content = column![
        header,
        container(text(subtitle).size(12).color(MUTED))
            .padding([4, 20]),
        col_headers,
        scrollable_list,
        container(footer).style(|_: &_| container::Style {
            border: Border {
                color: BORDER,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
    .spacing(0)
    .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
```

- [ ] **Step 3: Run `just check`**

Run: `just check`
Expected: PASS (with the remaining view stubs still in place)

- [ ] **Step 4: Commit**

```bash
git add src/views.rs src/styles.rs
git commit -m "Add UninstallSelect screen with table layout and search"
```

### Task 12: Implement `view_uninstall_review()`

**Files:**
- Modify: `src/views.rs`

- [ ] **Step 1: Implement `view_uninstall_review()`**

Replace the stub:

```rust
pub(crate) fn view_uninstall_review(&self) -> Element<'_, Message> {
    use iced::widget::{button, column, container, row, scrollable, text};
    use iced::{padding, Length};
    use crate::styles::*;

    // Header
    let back_btn = button(
        text(char::from(lucide_icons::Icon::ChevronLeft))
            .size(16)
            .font(LUCIDE_FONT),
    )
    .on_press(Message::GoBack)
    .style(ghost_button_style);

    let heading = text("Confirm uninstall").size(16).color(TEXT);

    let header = row![back_btn, heading]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding([16, 20]);

    // Warning banner
    let warning_icon = text(char::from(lucide_icons::Icon::TriangleAlert))
        .size(16)
        .font(LUCIDE_FONT)
        .color(STATUS_AMBER);

    let warning_content = column![
        text("This action cannot be undone")
            .size(13)
            .color(STATUS_AMBER),
        text("These packages will be permanently removed. You can reinstall them later through winget.")
            .size(12)
            .color(MUTED_FG),
    ]
    .spacing(4);

    let warning_banner = container(
        row![warning_icon, warning_content]
            .spacing(10)
            .align_y(iced::Alignment::Start),
    )
    .padding(12)
    .width(Length::Fill)
    .style(|_: &_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(
            0x45 as f32 / 255.0,
            0x1a as f32 / 255.0,
            0x03 as f32 / 255.0,
        ))),
        border: Border {
            color: Color::from_rgb(
                0x92 as f32 / 255.0,
                0x40 as f32 / 255.0,
                0x0e as f32 / 255.0,
            ),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    });

    // Package list
    let selected_packages: Vec<&upgrade::InstalledPackage> = self
        .installed_packages
        .iter()
        .filter(|p| self.uninstall_selected.contains(&p.winget_id_lower))
        .collect();

    let mut pkg_list = column![].spacing(2);

    for pkg in &selected_packages {
        let x_icon = text(char::from(lucide_icons::Icon::X))
            .size(14)
            .font(LUCIDE_FONT)
            .color(STATUS_RED);

        let display_name = if pkg.name.is_empty() { &pkg.winget_id } else { &pkg.name };
        let detail = format!(
            "{} \u{00b7} v{} \u{00b7} {}",
            pkg.winget_id,
            pkg.version,
            format_size(pkg.size_bytes),
        );

        let row_content = row![
            x_icon,
            column![
                text(display_name).size(13).color(TEXT),
                text(detail).size(11).color(MUTED),
            ]
            .spacing(2),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .padding([10, 12]);

        pkg_list = pkg_list.push(
            container(row_content)
                .width(Length::Fill)
                .style(|_: &_| container::Style {
                    background: Some(iced::Background::Color(CARD_BG)),
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }

    // Total size
    let total_size: u64 = selected_packages
        .iter()
        .filter_map(|p| p.size_bytes)
        .sum();

    let size_text = if total_size > 0 {
        format!(
            "{} packages to uninstall \u{00b7} Estimated ~{} will be freed",
            selected_packages.len(),
            format_size(Some(total_size)),
        )
    } else {
        format!("{} packages to uninstall", selected_packages.len())
    };

    let scrollable_content = scrollable(
        column![pkg_list]
            .spacing(8)
            .width(Length::Fill)
            .padding(padding::right(20)),
    );

    // Footer
    let edit_btn = button(
        row![
            text(char::from(lucide_icons::Icon::ChevronLeft))
                .size(13)
                .font(LUCIDE_FONT),
            text("Edit").size(13),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::GoBack)
    .style(ghost_button_style);

    let uninstall_btn = button(
        text(format!("Uninstall {} packages", selected_packages.len())).size(13),
    )
    .on_press(Message::StartUninstall)
    .style(danger_button_style);

    let footer = row![
        edit_btn,
        iced::widget::Space::new().width(Length::Fill),
        uninstall_btn,
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center)
    .padding([12, 20]);

    // Compose
    let content = column![
        header,
        container(warning_banner).padding([0, 20]),
        container(text(size_text).size(12).color(MUTED)).padding([12, 20]),
        container(scrollable_content).padding([0, 20]).height(Length::Fill),
        container(footer).style(|_: &_| container::Style {
            border: Border {
                color: BORDER,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
    .spacing(0)
    .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
```

- [ ] **Step 2: Run `just check`**

Run: `just check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/views.rs
git commit -m "Add UninstallReview screen with warning banner and package list"
```

### Task 13: Implement `view_uninstalling()`

**Files:**
- Modify: `src/views.rs`

- [ ] **Step 1: Implement `view_uninstalling()`**

Replace the stub. This reuses `view_progress_screen()`:

```rust
pub(crate) fn view_uninstalling(&self) -> Element<'_, Message> {
    view_progress_screen(
        &self.uninstall,
        &ProgressLabels {
            verb: "Uninstalling",
            done_label: "Uninstall",
            dry_run_warning: "No packages will actually be uninstalled",
        },
        self.uninstall_queue.iter().map(|p| p.name.as_str()),
        self.dry_run,
        Message::CancelUninstall,
        Message::FinishUninstallAndReset,
        self.spinner_frame,
    )
}
```

- [ ] **Step 2: Run `just check`**

Run: `just check`
Expected: PASS — everything should compile now.

- [ ] **Step 3: Commit**

```bash
git add src/views.rs
git commit -m "Add Uninstalling progress screen reusing view_progress_screen()"
```

---

## Chunk 5: Integration & Polish

### Task 14: Manual testing in dry-run mode

- [ ] **Step 1: Run the app in dry-run mode**

Run: `cargo run -- --dry`

- [ ] **Step 2: Verify the home screen**

- "Uninstall packages" card appears alongside "Check for updates"
- Card becomes clickable after installed scan completes (spinner → checkmark)

- [ ] **Step 3: Test uninstall selection flow**

- Click "Uninstall packages"
- Verify table layout with Name, Version, Size, Package ID columns
- Search works (filters by name and package ID)
- Select/deselect packages with checkboxes
- Ctrl+A selects all visible packages
- Footer shows count and size

- [ ] **Step 4: Test review screen**

- Click "Review uninstall"
- Warning banner appears
- Package list shows selected packages with details
- Total freed size shown (if sizes available)
- "Edit" goes back preserving selection
- "Uninstall N packages" starts uninstall

- [ ] **Step 5: Test uninstall progress**

- Progress screen shows with "Uninstalling" heading
- Dry-run completes quickly
- Status shows "Uninstall Complete"
- "Done" returns to home screen
- Installed scan restarts (spinner visible briefly)

- [ ] **Step 6: Test keyboard shortcuts**

- Enter advances through screens
- Escape goes back
- Ctrl+A toggles selection

- [ ] **Step 7: Test cancellation**

- Start uninstall, press Cancel
- Remaining packages show "Cancelled"

### Task 15: Fix any issues found during testing

- [ ] **Step 1: Fix any compilation, layout, or logic issues discovered**

Common things to watch for:
- Missing imports (Icon variants, style functions)
- Layout issues with column widths
- Scrollbar overlap (needs `padding::right(20)`)
- Missing `use` statements in views
- Clippy warnings

- [ ] **Step 2: Run `just check`**

Run: `just check`
Expected: PASS with no warnings

- [ ] **Step 3: Commit fixes**

```bash
git add -u
git commit -m "Fix issues found during uninstall feature testing"
```

### Task 16: Update CLAUDE.md architecture section

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Add `uninstall.rs` to the Architecture section**

After the `src/upgrade.rs` entry, add:
```
- **`src/uninstall.rs`** — Uninstall engine. `uninstall_all()` streams per-package uninstalls via `winget uninstall`. `scan_sizes()` async registry lookup for installed package sizes from `HKLM/HKCU\...\Uninstall` keys with `EstimatedSize` DWORD.
```

- [ ] **Step 2: Update Screen enum description**

Update the screen flow description to include `UninstallSelect`, `UninstallReview`, `Uninstalling`.

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "Update CLAUDE.md with uninstall module documentation"
```
