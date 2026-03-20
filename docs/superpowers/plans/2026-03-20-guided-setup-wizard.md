# Guided Setup Wizard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add pre-install warnings and post-install checklists for packages that need manual steps (browser downloads, terminal restarts, reboots).

**Architecture:** Add a `SetupKind` enum to `catalog.rs` for classifying packages. Reorder the install queue in `handle_start_install()`. Add inline badges + summary line to the review screen. Replace the shared `view_progress_screen()` call in `view_installing()` with a custom layout that shows a post-install checklist when done.

**Tech Stack:** Rust, Iced 0.14, existing color constants and badge styles from `styles.rs`.

**Spec:** `docs/superpowers/specs/2026-03-20-guided-setup-wizard-design.md`

---

### Task 1: Add `SetupKind` enum and `setup_kind()` method to `catalog.rs`

**Files:**
- Modify: `src/catalog.rs:8-56` (after `CatalogSource`, alongside `Package` impl)

- [ ] **Step 1: Add the `SetupKind` enum after `CatalogSource`**

Add this after line 25 in `catalog.rs` (after the `CatalogSource` impl block):

```rust
/// Classifies packages by what manual steps the user needs after install.
/// Variant order is load-bearing — derives `Ord` by declaration position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SetupKind {
    /// Normal winget install, no guidance needed.
    Silent,
    /// PATH changes — user needs to restart terminal (Bun, uv).
    TerminalRestart,
    /// Opens a browser URL — user must run the downloaded installer (Rust, Topping).
    BrowserDownload,
    /// System reboot required (WSL, RemoveWindowsAI).
    Reboot,
}
```

- [ ] **Step 2: Add `setup_kind()` and `setup_instruction()` methods to `impl Package`**

Add these methods inside the existing `impl Package` block (after `is_browser_download()`):

```rust
/// Classify this package by what post-install action the user needs.
pub fn setup_kind(&self) -> SetupKind {
    // Id-based overrides (can't be detected from install_command alone)
    match self.id.as_str() {
        "wsl" | "remove-windows-ai" => return SetupKind::Reboot,
        _ => {}
    }
    match self.install_command.as_deref() {
        Some(cmd) if cmd.starts_with("start http") => SetupKind::BrowserDownload,
        Some(_) => SetupKind::TerminalRestart,
        None => SetupKind::Silent,
    }
}

/// User-facing checklist instruction for this package, if it needs manual steps.
pub fn setup_instruction(&self) -> Option<&'static str> {
    match self.id.as_str() {
        "bun" => Some("Restart your terminal for Bun to be available on PATH"),
        "uv" => Some("Restart your terminal for uv to be available on PATH"),
        "rust" => Some("Run rustup-init.exe \u{2014} follow the prompts and select defaults"),
        "topping" => Some("Run the Topping installer you downloaded"),
        "wsl" => Some("Reboot, then open Ubuntu to set up your Linux username and password"),
        "remove-windows-ai" => Some("Reboot for changes to take effect"),
        _ => None,
    }
}
```

- [ ] **Step 3: Run `just check`**

```bash
just check
```

Expected: Build succeeds, clippy passes, fmt passes. The new code is not yet used — clippy may warn about dead code; this is expected and will resolve when the enum is used in Task 3.

- [ ] **Step 4: Commit**

```bash
git add src/catalog.rs
git commit -m "feat: add SetupKind enum and setup_kind()/setup_instruction() methods"
```

---

### Task 2: Add badge styles for `TerminalRestart` and `Reboot` badges to `styles.rs`

**Files:**
- Modify: `src/styles.rs:326-358` (badge styles section)

The `browser_badge_style` (blue-tinted) already exists for `BrowserDownload`. We need two more badge styles.

- [ ] **Step 1: Add `reboot_badge_style` and `restart_badge_style` after `browser_badge_style`**

Add after `browser_badge_style` (after line 358 in `styles.rs`):

```rust
pub fn reboot_badge_style(_theme: &Theme) -> container::Style {
    badge_base(
        Color::from_rgba(
            STATUS_RED.r,
            STATUS_RED.g,
            STATUS_RED.b,
            0.15,
        ),
        Color::from_rgba(
            STATUS_RED.r,
            STATUS_RED.g,
            STATUS_RED.b,
            0.3,
        ),
    )
}

pub fn restart_badge_style(_theme: &Theme) -> container::Style {
    badge_base(
        Color::from_rgba(
            STATUS_BLUE.r,
            STATUS_BLUE.g,
            STATUS_BLUE.b,
            0.15,
        ),
        Color::from_rgba(
            STATUS_BLUE.r,
            STATUS_BLUE.g,
            STATUS_BLUE.b,
            0.3,
        ),
    )
}
```

- [ ] **Step 2: Run `just check`**

```bash
just check
```

Expected: Build succeeds. New styles are unused for now — dead code warnings may appear but will resolve in Task 3. If clippy fails on warnings, add `#[allow(dead_code)]` temporarily.

- [ ] **Step 3: Commit**

```bash
git add src/styles.rs
git commit -m "feat: add reboot and terminal-restart badge styles"
```

---

### Task 3: Add SetupKind badges to the Review screen and Package Select screen

**Files:**
- Modify: `src/views.rs:436-585` (`view_review`) and `src/views.rs:2222-2262` (`package_row`)

- [ ] **Step 1: Update `package_row()` to show SetupKind badges**

In `package_row()` (around line 2245 in `views.rs`), replace the existing `is_browser` badge block:

```rust
    if is_browser {
        let badge_content = row![
            text(char::from(Icon::ExternalLink))
                .size(9)
                .font(LUCIDE_FONT)
                .color(STATUS_BLUE),
            text("Opens browser").size(10).color(STATUS_BLUE),
        ]
        .spacing(3)
        .align_y(iced::Alignment::Center);
        let badge = container(badge_content)
            .style(browser_badge_style)
            .padding([1, 6]);
        pkg_row = pkg_row.push(badge);
    }
```

With a match on `setup_kind()`:

```rust
    match pkg.setup_kind() {
        catalog::SetupKind::BrowserDownload => {
            let badge_content = row![
                text(char::from(Icon::ExternalLink))
                    .size(9)
                    .font(LUCIDE_FONT)
                    .color(STATUS_BLUE),
                text("manual download").size(10).color(STATUS_BLUE),
            ]
            .spacing(3)
            .align_y(iced::Alignment::Center);
            let badge = container(badge_content)
                .style(browser_badge_style)
                .padding([1, 6]);
            pkg_row = pkg_row.push(badge);
        }
        catalog::SetupKind::Reboot => {
            let badge = container(text("reboot").size(10).color(STATUS_RED))
                .style(reboot_badge_style)
                .padding([1, 6]);
            pkg_row = pkg_row.push(badge);
        }
        catalog::SetupKind::TerminalRestart => {
            let badge = container(text("terminal restart").size(10).color(STATUS_BLUE))
                .style(restart_badge_style)
                .padding([1, 6]);
            pkg_row = pkg_row.push(badge);
        }
        catalog::SetupKind::Silent => {}
    }
```

Also remove the now-unused `let is_browser = pkg.is_browser_download();` line (around line 2235).

- [ ] **Step 2: Update `view_review()` to show SetupKind badges inline**

In `view_review()` (around line 476), replace the existing `is_browser` handling. The current code (lines 476-500) handles `is_browser_download()` for the method widget column. Replace:

```rust
                let is_browser = pkg.is_browser_download();

                let method_widget: Element<'_, Message> = if is_browser {
                    row![
                        text(char::from(Icon::ExternalLink))
                            .size(10)
                            .font(LUCIDE_FONT)
                            .color(STATUS_BLUE),
                        text("opens browser").size(11).color(STATUS_BLUE),
                    ]
                    .spacing(4)
                    .align_y(iced::Alignment::Center)
                    .into()
                } else {
                    let method = match (&pkg.install_command, &pkg.winget_id) {
                        (Some(cmd), _) => cmd.clone(),
                        (_, Some(wid)) => wid.clone(),
                        _ => "unknown".into(),
                    };
                    text(method)
                        .size(11)
                        .font(iced::Font::MONOSPACE)
                        .color(MUTED)
                        .into()
                };
```

With:

```rust
                let method_widget: Element<'_, Message> = match pkg.setup_kind() {
                    catalog::SetupKind::BrowserDownload => {
                        row![
                            text(char::from(Icon::ExternalLink))
                                .size(10)
                                .font(LUCIDE_FONT)
                                .color(STATUS_BLUE),
                            text("opens browser").size(11).color(STATUS_BLUE),
                        ]
                        .spacing(4)
                        .align_y(iced::Alignment::Center)
                        .into()
                    }
                    _ => {
                        let method = match (&pkg.install_command, &pkg.winget_id) {
                            (Some(cmd), _) => cmd.clone(),
                            (_, Some(wid)) => wid.clone(),
                            _ => "unknown".into(),
                        };
                        text(method)
                            .size(11)
                            .font(iced::Font::MONOSPACE)
                            .color(MUTED)
                            .into()
                    }
                };
```

Then add a SetupKind badge to the name row (after the "Already installed" badge, around line 511):

```rust
                match pkg.setup_kind() {
                    catalog::SetupKind::BrowserDownload => {
                        let badge = container(text("manual download").size(10).color(STATUS_AMBER))
                            .style(warning_badge_style)
                            .padding([2, 6]);
                        name_row = name_row.push(badge);
                    }
                    catalog::SetupKind::Reboot => {
                        let badge = container(text("reboot").size(10).color(STATUS_RED))
                            .style(reboot_badge_style)
                            .padding([2, 6]);
                        name_row = name_row.push(badge);
                    }
                    catalog::SetupKind::TerminalRestart => {
                        let badge = container(text("terminal restart").size(10).color(STATUS_BLUE))
                            .style(restart_badge_style)
                            .padding([2, 6]);
                        name_row = name_row.push(badge);
                    }
                    catalog::SetupKind::Silent => {}
                }
```

- [ ] **Step 3: Replace WSL warning with adaptive summary line in `view_review()`**

Replace lines 570-576:

```rust
        if queue.iter().any(|p| p.id == catalog::WSL_PACKAGE_ID) {
            content = content.push(status_indicator(
                Icon::TriangleAlert,
                "WSL installation may require a system restart to take effect.".into(),
                STATUS_AMBER,
            ));
        }
```

With:

```rust
        let special_count = queue
            .iter()
            .filter(|p| p.setup_kind() != catalog::SetupKind::Silent)
            .count();
        if special_count > 0 {
            let has_reboot = queue
                .iter()
                .any(|p| p.setup_kind() == catalog::SetupKind::Reboot);
            let has_restart = queue
                .iter()
                .any(|p| p.setup_kind() == catalog::SetupKind::TerminalRestart);
            let has_download = queue
                .iter()
                .any(|p| p.setup_kind() == catalog::SetupKind::BrowserDownload);

            let summary = if special_count == 1 {
                if has_reboot {
                    "1 package requires a system reboot \u{2014} installed last.".into()
                } else if has_download {
                    "1 package opens a browser download \u{2014} installed last.".into()
                } else {
                    "1 package needs a terminal restart \u{2014} installed last.".into()
                }
            } else if has_reboot && !has_restart && !has_download {
                format!("{special_count} packages require a system reboot \u{2014} installed last.")
            } else if has_restart && !has_reboot && !has_download {
                format!(
                    "{special_count} packages need a terminal restart \u{2014} installed last."
                )
            } else {
                format!(
                    "{special_count} packages need manual steps \u{2014} they'll be installed last."
                )
            };

            content = content.push(status_indicator(
                Icon::TriangleAlert,
                summary,
                STATUS_AMBER,
            ));
        }
```

- [ ] **Step 4: Add necessary imports to `views.rs`**

At the top of `views.rs`, ensure the new badge styles are imported. Find the existing imports from `styles` and add `reboot_badge_style` and `restart_badge_style`.

- [ ] **Step 5: Run `just check`**

```bash
just check
```

Expected: Build succeeds. The `is_browser_download()` method in `catalog.rs` and `WSL_PACKAGE_ID` constant may now be unused — if clippy warns, remove them (they're fully replaced by `setup_kind()`).

- [ ] **Step 6: Commit**

```bash
git add src/views.rs src/catalog.rs
git commit -m "feat: add SetupKind badges to review and package select screens"
```

---

### Task 4: Reorder install queue in `handle_start_install()`

**Files:**
- Modify: `src/main.rs:911-933` (`handle_start_install`)

- [ ] **Step 1: Add queue reordering logic**

Replace the current `handle_start_install()`:

```rust
    fn handle_start_install(&mut self) -> Task<Message> {
        let queue: Vec<Package> = self
            .catalog
            .iter()
            .filter(|p| self.selected.contains(&p.id))
            .cloned()
            .collect();

        self.install.start(queue.len());
        self.install_queue = queue.clone();
        self.screen = Screen::Installing;

        let dry = self.dry_run;
        let extra = self.settings.install_args();
        let (task, handle) = Task::run(
            install::install_all(queue, dry, extra),
            Message::InstallProgress,
        )
        .abortable();

        self.install._handle = Some(handle.abort_on_drop());
        task
    }
```

With:

```rust
    fn handle_start_install(&mut self) -> Task<Message> {
        let mut queue: Vec<Package> = self
            .catalog
            .iter()
            .filter(|p| self.selected.contains(&p.id))
            .cloned()
            .collect();

        // Reorder: silent packages first, then by SetupKind weight
        queue.sort_by_key(|p| p.setup_kind());

        let has_special = queue
            .iter()
            .any(|p| p.setup_kind() != catalog::SetupKind::Silent);
        let all_special = queue
            .iter()
            .all(|p| p.setup_kind() != catalog::SetupKind::Silent);

        self.install.start(queue.len());
        self.install_queue = queue.clone();
        self.screen = Screen::Installing;

        // Log a note about reordering if there are special packages mixed with silent ones
        if has_special && !all_special {
            self.install
                .log
                .push("Manual-step packages queued last.".into());
            self.install.log.push(String::new());
        }

        let dry = self.dry_run;
        let extra = self.settings.install_args();
        let (task, handle) = Task::run(
            install::install_all(queue, dry, extra),
            Message::InstallProgress,
        )
        .abortable();

        self.install._handle = Some(handle.abort_on_drop());
        task
    }
```

- [ ] **Step 2: Run `just check`**

```bash
just check
```

Expected: Build succeeds. The sort is stable so silent packages keep their original relative order, and special packages sort by weight within their group.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: reorder install queue — manual-step packages last"
```

---

### Task 5: Add post-install checklist to the Installing screen

**Files:**
- Modify: `src/views.rs:587-601` (`view_installing`)
- Modify: `src/main.rs` (add `checklist_checked` field to `App`, add `ToggleChecklist` message)

- [ ] **Step 1: Add checklist state to `App` and new `Message` variant**

In `src/main.rs`, add a new field to the `App` struct initialization (around line 403, after `install_queue`):

```rust
                checklist_checked: HashSet::new(),
```

Find the `App` struct definition (search for `install_queue: Vec<Package>`) and add the field declaration:

```rust
    checklist_checked: HashSet<String>,
```

This uses `String` keys: package IDs for per-package checkboxes (BrowserDownload), and group keys `"_terminal_restart"` / `"_reboot"` for grouped checkboxes.

Add a new `Message` variant (after `FinishAndReset` around line 480):

```rust
    ToggleChecklist(String),
```

Add the handler in the `update()` match (after the `FinishAndReset` arm):

```rust
            Message::ToggleChecklist(key) => {
                if !self.checklist_checked.remove(&key) {
                    self.checklist_checked.insert(key);
                }
                Task::none()
            }
```

Clear the checklist state in `handle_finish_and_reset()` (add before `self.screen = Screen::ProfileSelect`):

```rust
        self.checklist_checked.clear();
```

Also clear it in `handle_start_install()` (add before `self.install.start(queue.len())`):

```rust
        self.checklist_checked.clear();
```

- [ ] **Step 2: Replace `view_installing()` with custom layout including checklist**

Replace the current `view_installing()` (lines 587-601):

```rust
    pub(crate) fn view_installing(&self) -> Element<'_, Message> {
        view_progress_screen(
            &self.install,
            &ProgressLabels {
                verb: "Installing",
                done_label: "Installation",
                dry_run_warning: "No packages will actually be installed",
            },
            self.install_queue.iter().map(|p| p.name.as_str()),
            self.dry_run,
            Message::CancelInstall,
            Message::FinishAndReset,
            self.spinner_frame,
        )
    }
```

With:

```rust
    pub(crate) fn view_installing(&self) -> Element<'_, Message> {
        let state = &self.install;
        let names: Vec<&str> = self.install_queue.iter().map(|p| p.name.as_str()).collect();
        let total = names.len();
        let (done_count, failed_count, cancelled_count) = state.status_counts();

        // Heading
        let heading_row = if state.done {
            let label = match (self.dry_run, cancelled_count > 0) {
                (true, true) => "Dry Run Cancelled".to_string(),
                (true, false) => "Dry Run Complete".to_string(),
                (false, true) => "Installation Cancelled".to_string(),
                (false, false) => "Installation Complete".to_string(),
            };
            row![text(label).size(20)]
                .spacing(8)
                .align_y(iced::Alignment::Center)
        } else {
            let verb = if self.dry_run {
                "[DRY RUN] Installing".to_string()
            } else {
                "Installing".to_string()
            };
            let count_text = format!(
                "{} of {total} \u{00b7} {}",
                state.current + 1,
                state.elapsed_display()
            );
            row![
                text(verb).size(20),
                text(count_text).size(14).color(MUTED),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
        };

        // Subtitle
        let subtitle: Element<'_, Message> = if state.done {
            let mut counts = row![].spacing(6).align_y(iced::Alignment::Center);
            counts = counts
                .push(
                    text(char::from(Icon::CircleCheck))
                        .size(13)
                        .font(LUCIDE_FONT)
                        .color(STATUS_GREEN),
                )
                .push(text(format!("{done_count} succeeded")).size(13).color(STATUS_GREEN));
            counts = counts
                .push(text("\u{00b7}").size(13).color(MUTED))
                .push(
                    text(char::from(Icon::CircleX))
                        .size(13)
                        .font(LUCIDE_FONT)
                        .color(if failed_count > 0 { STATUS_RED } else { MUTED }),
                )
                .push(
                    text(format!("{failed_count} failed"))
                        .size(13)
                        .color(if failed_count > 0 { STATUS_RED } else { MUTED }),
                );
            if cancelled_count > 0 {
                counts = counts
                    .push(text("\u{00b7}").size(13).color(MUTED))
                    .push(
                        text(char::from(Icon::CircleX))
                            .size(13)
                            .font(LUCIDE_FONT)
                            .color(STATUS_AMBER),
                    )
                    .push(
                        text(format!("{cancelled_count} cancelled"))
                            .size(13)
                            .color(STATUS_AMBER),
                    );
            }
            counts = counts
                .push(text("\u{00b7}").size(13).color(MUTED))
                .push(
                    text(char::from(Icon::Clock))
                        .size(13)
                        .font(LUCIDE_FONT)
                        .color(MUTED),
                )
                .push(text(state.elapsed_display()).size(13).color(MUTED));
            counts.into()
        } else if self.dry_run {
            text("No packages will actually be installed")
                .size(13)
                .color(STATUS_AMBER)
                .into()
        } else {
            let name = names.get(state.current).unwrap_or(&"...");
            text(*name).size(13).color(MUTED).into()
        };

        let completed = (done_count + failed_count + cancelled_count) as f32;
        let progress = progress_bar(0.0..=total as f32, completed);

        // Package list
        let mut pkg_list = column![].spacing(2).width(Length::Fill);
        for (i, name) in names.iter().enumerate() {
            let (icon, color, label): (Element<'_, Message>, _, _) =
                match &state.statuses[i] {
                    PackageStatus::Pending => (
                        text(char::from(Icon::Circle))
                            .size(14)
                            .font(LUCIDE_FONT)
                            .color(MUTED)
                            .into(),
                        MUTED,
                        "Pending".into(),
                    ),
                    PackageStatus::Installing => (
                        text(SPINNER_FRAMES[self.spinner_frame])
                            .size(14)
                            .color(STATUS_BLUE)
                            .into(),
                        STATUS_BLUE,
                        "Installing...".into(),
                    ),
                    PackageStatus::Done => (
                        text(char::from(Icon::CircleCheck))
                            .size(14)
                            .font(LUCIDE_FONT)
                            .color(STATUS_GREEN)
                            .into(),
                        STATUS_GREEN,
                        "Done".into(),
                    ),
                    PackageStatus::Failed(e) => (
                        text(char::from(Icon::CircleX))
                            .size(14)
                            .font(LUCIDE_FONT)
                            .color(STATUS_RED)
                            .into(),
                        STATUS_RED,
                        format!("Failed: {e}"),
                    ),
                    PackageStatus::Cancelled => (
                        text(char::from(Icon::CircleX))
                            .size(14)
                            .font(LUCIDE_FONT)
                            .color(STATUS_AMBER)
                            .into(),
                        STATUS_AMBER,
                        "Cancelled".into(),
                    ),
                };

            let pkg_row = row![
                icon,
                text(*name).size(14),
                iced::widget::Space::new().width(Length::Fill),
                text(label).size(12).color(color),
            ]
            .spacing(8)
            .padding(padding::top(4).bottom(4).right(20))
            .align_y(iced::Alignment::Center);

            pkg_list = pkg_list.push(pkg_row);
        }

        let scrollable_pkgs = scrollable(pkg_list)
            .height(Length::FillPortion(3))
            .width(Length::Fill);

        let log_box = terminal_log_box(&state.log)
            .height(Length::FillPortion(2))
            .width(Length::Fill);

        // Post-install checklist (only when done and there are special packages)
        let checklist: Option<Element<'_, Message>> = if state.done {
            self.build_post_install_checklist()
        } else {
            None
        };

        // Footer
        let mut cancel_btn = button(text("Cancel").size(14))
            .style(cancel_button_style)
            .padding([8, 20]);
        if !state.done {
            cancel_btn = cancel_btn.on_press(Message::CancelInstall);
        }

        let copy_btn: Element<'_, Message> = if state.done {
            let (icon, label) = if state.copy_status {
                (Icon::ClipboardCheck, "Copied!")
            } else {
                (Icon::Clipboard, "Copy log")
            };
            let mut btn = button(
                row![
                    text(char::from(icon)).size(14).font(LUCIDE_FONT),
                    text(label).size(14),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            )
            .style(ghost_button_style)
            .padding([8, 16]);
            if !state.copy_status {
                btn = btn.on_press(Message::CopyLog);
            }
            btn.into()
        } else {
            iced::widget::Space::new().into()
        };

        let mut done_btn = button(text("Done").size(14))
            .style(continue_button_style)
            .padding([8, 20]);
        if state.done {
            done_btn = done_btn.on_press(Message::FinishAndReset);
        }

        let footer = row![
            cancel_btn,
            iced::widget::Space::new().width(Length::Fill),
            copy_btn,
            done_btn,
        ]
        .spacing(8)
        .width(Length::Fill);

        let mut content = column![heading_row, subtitle, progress, scrollable_pkgs, log_box]
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(cl) = checklist {
            content = content.push(cl);
        }

        content = content.push(footer);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }
```

- [ ] **Step 3: Add `build_post_install_checklist()` helper method**

Add this method to `impl App` in `views.rs`:

```rust
    /// Build the post-install checklist grouped by SetupKind.
    /// Returns `None` if no special packages succeeded.
    fn build_post_install_checklist(&self) -> Option<Element<'_, Message>> {
        use catalog::SetupKind;

        let queue = &self.install_queue;
        let statuses = &self.install.statuses;

        // Only show checklist items for packages that actually succeeded
        let succeeded = |i: usize| matches!(statuses.get(i), Some(PackageStatus::Done));

        let download_pkgs: Vec<&Package> = queue
            .iter()
            .enumerate()
            .filter(|(i, p)| p.setup_kind() == SetupKind::BrowserDownload && succeeded(*i))
            .map(|(_, p)| p)
            .collect();
        let restart_pkgs: Vec<&Package> = queue
            .iter()
            .enumerate()
            .filter(|(i, p)| p.setup_kind() == SetupKind::TerminalRestart && succeeded(*i))
            .map(|(_, p)| p)
            .collect();
        let reboot_pkgs: Vec<&Package> = queue
            .iter()
            .enumerate()
            .filter(|(i, p)| p.setup_kind() == SetupKind::Reboot && succeeded(*i))
            .map(|(_, p)| p)
            .collect();

        if download_pkgs.is_empty() && restart_pkgs.is_empty() && reboot_pkgs.is_empty() {
            return None;
        }

        let mut checklist = column![]
            .spacing(8)
            .width(Length::Fill);

        checklist = checklist.push(
            text("Post-install steps")
                .size(14)
                .color(TEXT),
        );

        // Group 1: Browser downloads — one checkbox per package (keyed by package id)
        for pkg in &download_pkgs {
            if let Some(instruction) = pkg.setup_instruction() {
                let key = pkg.id.clone();
                let is_checked = self.checklist_checked.contains(&pkg.id);
                let cb = checkbox(is_checked)
                    .label(format!("{} \u{2014} {instruction}", pkg.name))
                    .on_toggle(move |_| Message::ToggleChecklist(key.clone()))
                    .size(14)
                    .text_size(12)
                    .style(package_checkbox_style);
                checklist = checklist.push(cb);
            }
        }

        // Group 2: Terminal restart — single checkbox (keyed by group name)
        if !restart_pkgs.is_empty() {
            let names: Vec<&str> = restart_pkgs.iter().map(|p| p.name.as_str()).collect();
            let label = format!("Restart your terminal (for {})", names.join(", "));
            let is_checked = self.checklist_checked.contains("_terminal_restart");
            let cb = checkbox(is_checked)
                .label(label)
                .on_toggle(|_| Message::ToggleChecklist("_terminal_restart".into()))
                .size(14)
                .text_size(12)
                .style(package_checkbox_style);
            checklist = checklist.push(cb);
        }

        // Group 3: Reboot — single checkbox (keyed by group name)
        if !reboot_pkgs.is_empty() {
            let names: Vec<&str> = reboot_pkgs.iter().map(|p| p.name.as_str()).collect();
            let label = format!("Reboot your system (for {})", names.join(", "));
            let is_checked = self.checklist_checked.contains("_reboot");
            let cb = checkbox(is_checked)
                .label(label)
                .on_toggle(|_| Message::ToggleChecklist("_reboot".into()))
                .size(14)
                .text_size(12)
                .style(package_checkbox_style);
            checklist = checklist.push(cb);
        }

        Some(checklist.into())
    }
```

- [ ] **Step 4: Run `just check`**

```bash
just check
```

Expected: Build succeeds, clippy clean, fmt clean.

- [ ] **Step 5: Manual test with `cargo run -- --dry`**

```bash
cargo run -- --dry
```

Test flow:
1. Select Desktop profile
2. Verify badges appear on Bun, Rust, uv, WSL, Topping, RemoveWindowsAI in package select
3. Go to Review — verify badges appear inline and summary line shows at the bottom
4. Click Install — verify "Manual-step packages queued last." appears in log, and silent packages install before special ones
5. When done, verify the post-install checklist appears with the three groups
6. Click checkboxes — verify they toggle
7. Click Done — verify return to home

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/views.rs
git commit -m "feat: post-install checklist on installing screen"
```

---

### Task 6: Clean up dead code

**Files:**
- Modify: `src/catalog.rs` (remove `WSL_PACKAGE_ID` and `is_browser_download()` if unused)
- Modify: `src/views.rs` (remove any leftover `is_browser_download` references)

- [ ] **Step 1: Check for remaining uses of `WSL_PACKAGE_ID` and `is_browser_download()`**

Search the codebase for these. If no remaining references exist outside the new `setup_kind()` method, remove:
- `pub const WSL_PACKAGE_ID: &str = "wsl";` from `catalog.rs`
- `pub fn is_browser_download(&self) -> bool { ... }` from `catalog.rs`

- [ ] **Step 2: Run `just check`**

```bash
just check
```

Expected: Build succeeds with no warnings.

- [ ] **Step 3: Commit**

```bash
git add src/catalog.rs src/views.rs
git commit -m "chore: remove WSL_PACKAGE_ID and is_browser_download, replaced by SetupKind"
```
