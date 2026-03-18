# Winget Search Integration Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a dedicated screen for searching the full winget catalog and installing packages from search results.

**Architecture:** New `SearchPackage`/`SearchProgress` types in `upgrade.rs`, streamed `winget search` via existing `run_winget_scan()`, installation via `run_winget_batch()`. Two new `Screen` variants (`WingetSearch`, `WingetSearchInstalling`) with views in `views.rs` and message handlers in `main.rs`.

**Tech Stack:** Rust, Iced 0.14, tokio, winget CLI

**Spec:** `docs/superpowers/specs/2026-03-17-winget-search-design.md`

---

## Chunk 1: Data Model & Search Engine

### Task 1: Add `SearchPackage`, `SearchProgress`, and `BatchItem` impl to `upgrade.rs`

**Files:**
- Modify: `src/upgrade.rs`

- [ ] **Step 1: Add `SearchPackage` struct after the `BatchItem` impls (line ~121)**

```rust
#[derive(Debug, Clone)]
pub struct SearchPackage {
    pub name: String,
    pub winget_id: String,
    pub version: String,
    pub source: String,
    pub name_lower: String,
    pub winget_id_lower: String,
}

impl BatchItem for SearchPackage {
    fn name(&self) -> &str {
        &self.name
    }
    fn winget_id(&self) -> &str {
        &self.winget_id
    }
}
```

- [ ] **Step 2: Add `SearchProgress` enum after `ScanProgress` (line ~129)**

```rust
#[derive(Debug, Clone)]
pub enum SearchProgress {
    Activity {
        #[allow(dead_code)]
        line: String,
    },
    Completed {
        packages: Vec<SearchPackage>,
    },
    Failed {
        #[allow(dead_code)]
        error: String,
    },
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: success (new types are unused but that's fine — no warnings because fields are `pub`)

- [ ] **Step 4: Commit**

```bash
git add src/upgrade.rs
git commit -m "Add SearchPackage and SearchProgress types for winget search"
```

### Task 2: Add `parse_search_table()` to `upgrade.rs`

**Files:**
- Modify: `src/upgrade.rs`

- [ ] **Step 1: Add `parse_search_table` after `parse_list_table` (line ~275)**

The `winget search` output has columns: `Name`, `Id`, `Version`, optionally `Match`, then `Source`. The parser must detect whether `Match` is present by checking the header line. Reuses existing `safe_slice`, `safe_slice_to_end`, `find_data_start` helpers.

```rust
pub fn parse_search_table(lines: &[String]) -> Vec<SearchPackage> {
    let header_idx = lines
        .iter()
        .position(|l| l.contains("Name") && l.contains("Id") && l.contains("Version"));

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

    // "Match" column is optional — only present for partial matches
    let match_col = header.find("Match");
    let source_col = header.find("Source");

    // Version ends at Match if present, otherwise at Source, otherwise end of line
    let version_end = match_col.or(source_col).unwrap_or(usize::MAX);

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

        let source = if let Some(sc) = source_col {
            if line.len() > sc {
                safe_slice_to_end(line, sc)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if id.is_empty() {
            continue;
        }

        packages.push(SearchPackage {
            name_lower: name.to_lowercase(),
            winget_id_lower: id.to_lowercase(),
            name,
            winget_id: id,
            version,
            source,
        });
    }

    packages
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`

- [ ] **Step 3: Commit**

```bash
git add src/upgrade.rs
git commit -m "Add parse_search_table for winget search output"
```

### Task 3: Add `search_winget()` stream to `upgrade.rs`

**Files:**
- Modify: `src/upgrade.rs`

- [ ] **Step 1: Add `search_winget` after `scan_installed` (line ~215)**

```rust
pub fn search_winget(
    query: String,
    dry_run: bool,
) -> impl futures::Stream<Item = SearchProgress> + Send {
    stream::channel(
        100,
        move |mut sender: futures::channel::mpsc::Sender<SearchProgress>| async move {
            if dry_run {
                let _ = sender
                    .send(SearchProgress::Activity {
                        line: format!("Searching for \"{query}\"..."),
                    })
                    .await;

                tokio::time::sleep(std::time::Duration::from_millis(800)).await;

                let fake = vec![
                    SearchPackage {
                        name: "Notepad++".into(),
                        winget_id: "Notepad++.Notepad++".into(),
                        version: "8.7.1".into(),
                        source: "winget".into(),
                        name_lower: "notepad++".into(),
                        winget_id_lower: "notepad++.notepad++".into(),
                    },
                    SearchPackage {
                        name: "WinSCP".into(),
                        winget_id: "WinSCP.WinSCP".into(),
                        version: "6.3.3".into(),
                        source: "winget".into(),
                        name_lower: "winscp".into(),
                        winget_id_lower: "winscp.winscp".into(),
                    },
                    SearchPackage {
                        name: "KeePass".into(),
                        winget_id: "DominikReichl.KeePass".into(),
                        version: "2.57.1".into(),
                        source: "winget".into(),
                        name_lower: "keepass".into(),
                        winget_id_lower: "dominikreichl.keepass".into(),
                    },
                ];

                let _ = sender
                    .send(SearchProgress::Completed { packages: fake })
                    .await;
                return;
            }

            let Ok(all_lines) = run_winget_scan(
                &["search", &query, "--count", "100", "--accept-source-agreements"],
                &mut sender,
                |e| match e {
                    ScanEvent::Activity(line) | ScanEvent::Log(line) => {
                        SearchProgress::Activity { line }
                    }
                    ScanEvent::Failed(error) => SearchProgress::Failed { error },
                },
            )
            .await
            else {
                return;
            };

            let packages = parse_search_table(&all_lines);
            let _ = sender
                .send(SearchProgress::Completed { packages })
                .await;
        },
    )
}
```

- [ ] **Step 2: Add `search_install_all` thin wrapper at the end of the file (after `upgrade_all`)**

```rust
pub fn search_install_all(
    packages: Vec<SearchPackage>,
    dry_run: bool,
    extra_args: Vec<String>,
) -> impl futures::Stream<Item = InstallProgress> + Send {
    install::run_winget_batch(
        packages,
        "install",
        vec!["--accept-package-agreements", "--accept-source-agreements"],
        dry_run,
        extra_args,
    )
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`

- [ ] **Step 4: Commit**

```bash
git add src/upgrade.rs
git commit -m "Add search_winget stream and search_install_all batch wrapper"
```

## Chunk 2: App State, Messages & Handlers

### Task 4: Add `Screen` variants, `App` state fields, and `Message` variants to `main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `Screen` variants**

In the `Screen` enum (after `Uninstalling`, around line 411), add:

```rust
    WingetSearch,
    WingetSearchInstalling,
```

- [ ] **Step 2: Add state fields to `App` struct**

In the `App` struct (after the `spinner_frame` field, around line 333), add:

```rust
    // Winget search state
    pub(crate) winget_search_query: String,
    pub(crate) winget_search_results: Vec<upgrade::SearchPackage>,
    pub(crate) winget_search_selected: HashSet<String>,
    pub(crate) winget_search_scanning: bool,
    pub(crate) winget_search_error: Option<String>,
    pub(crate) winget_search_queue: Vec<upgrade::SearchPackage>,
    pub(crate) winget_search_install: ProgressState,
    pub(crate) _winget_search_handle: Option<task::Handle>,
```

- [ ] **Step 3: Initialize new fields in `App::new()`**

In the `Self { ... }` block inside `App::new()` (after `spinner_frame: 0`), add:

```rust
                winget_search_query: String::new(),
                winget_search_results: Vec::new(),
                winget_search_selected: HashSet::new(),
                winget_search_scanning: false,
                winget_search_error: None,
                winget_search_queue: Vec::new(),
                winget_search_install: ProgressState::default(),
                _winget_search_handle: None,
```

- [ ] **Step 4: Add `Message` variants**

In the `Message` enum (before `KeyConfirm`), add:

```rust
    GoToWingetSearch,
    WingetSearchQueryChanged(String),
    StartWingetSearch,
    WingetSearchProgress(upgrade::SearchProgress),
    ToggleWingetSearchPackage(String),
    StartWingetSearchInstall,
    CancelWingetSearchInstall,
    WingetSearchInstallProgress(install::InstallProgress),
    FinishWingetSearchInstall,
    SelectAllWingetSearch,
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build`
Expected: warnings about unused variants (that's fine — handlers come next)

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "Add Screen variants, App state, and Message variants for winget search"
```

### Task 5: Add message handlers to `main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add message dispatch arms in `App::update()`**

In the `match message` block inside `update()`, add these arms in the domain handlers section (after the existing uninstall handlers, around line 521):

```rust
            Message::GoToWingetSearch => self.handle_go_to_winget_search(),
            Message::WingetSearchQueryChanged(v) => {
                self.winget_search_query = v;
                Task::none()
            }
            Message::StartWingetSearch => self.handle_start_winget_search(),
            Message::WingetSearchProgress(e) => self.handle_winget_search_progress(e),
            Message::ToggleWingetSearchPackage(id) => {
                if !self.winget_search_selected.remove(&id) {
                    self.winget_search_selected.insert(id);
                }
                Task::none()
            }
            Message::StartWingetSearchInstall => self.handle_start_winget_search_install(),
            Message::CancelWingetSearchInstall => self.handle_cancel_winget_search_install(),
            Message::WingetSearchInstallProgress(e) => {
                self.handle_winget_search_install_progress(e)
            }
            Message::FinishWingetSearchInstall => self.handle_finish_winget_search_install(),
            Message::SelectAllWingetSearch => self.handle_select_all_winget_search(),
```

- [ ] **Step 2: Add handler methods**

Add a new section after the uninstall handlers (after `handle_size_scan_result`, around line 965):

```rust
    // ── Winget search flow ────────────────────────────────────

    fn handle_go_to_winget_search(&mut self) -> Task<Message> {
        self.winget_search_results.clear();
        self.winget_search_selected.clear();
        self.winget_search_error = None;
        self.winget_search_scanning = false;
        self._winget_search_handle = None;
        self.screen = Screen::WingetSearch;
        widget::operation::focus(widget::Id::new(SEARCH_INPUT_ID))
    }

    fn handle_start_winget_search(&mut self) -> Task<Message> {
        let query = self.winget_search_query.trim().to_string();
        if query.is_empty() {
            return Task::none();
        }

        self.winget_search_results.clear();
        self.winget_search_selected.clear();
        self.winget_search_error = None;
        self.winget_search_scanning = true;

        let dry = self.dry_run;
        let (task, handle) = Task::run(
            upgrade::search_winget(query, dry),
            Message::WingetSearchProgress,
        )
        .abortable();

        self._winget_search_handle = Some(handle.abort_on_drop());
        task
    }

    fn handle_winget_search_progress(
        &mut self,
        event: upgrade::SearchProgress,
    ) -> Task<Message> {
        match event {
            upgrade::SearchProgress::Activity { .. } => {}
            upgrade::SearchProgress::Completed { packages } => {
                self.winget_search_results = packages;
                self.winget_search_scanning = false;
                self._winget_search_handle = None;
            }
            upgrade::SearchProgress::Failed { error } => {
                self.winget_search_error = Some(error);
                self.winget_search_scanning = false;
                self._winget_search_handle = None;
            }
        }
        Task::none()
    }

    fn handle_start_winget_search_install(&mut self) -> Task<Message> {
        let queue: Vec<upgrade::SearchPackage> = self
            .winget_search_results
            .iter()
            .filter(|p| self.winget_search_selected.contains(&p.winget_id))
            .cloned()
            .collect();

        if queue.is_empty() {
            return Task::none();
        }

        self.winget_search_install.start(queue.len());
        self.winget_search_queue = queue.clone();
        self.screen = Screen::WingetSearchInstalling;

        let dry = self.dry_run;
        let extra = self.settings.install_args();
        let (task, handle) = Task::run(
            upgrade::search_install_all(queue, dry, extra),
            Message::WingetSearchInstallProgress,
        )
        .abortable();

        self.winget_search_install._handle = Some(handle.abort_on_drop());
        task
    }

    fn handle_cancel_winget_search_install(&mut self) -> Task<Message> {
        self.winget_search_install.cancel("Installation");
        Task::none()
    }

    fn handle_winget_search_install_progress(
        &mut self,
        event: install::InstallProgress,
    ) -> Task<Message> {
        let queue = &self.winget_search_queue;
        self.winget_search_install.handle_event(event, |i| {
            let name = queue.get(i).map(|p| p.name.as_str()).unwrap_or("...");
            format!("Installing {name}")
        });
        Task::none()
    }

    fn handle_finish_winget_search_install(&mut self) -> Task<Message> {
        self.winget_search_queue.clear();
        self.winget_search_install = ProgressState::default();
        self.winget_search_selected.clear();
        self.screen = Screen::WingetSearch;

        // Re-scan installed packages so installed_map stays current
        let (scan_task, handle) = Task::run(
            upgrade::scan_installed(self.dry_run),
            Message::InstalledScanProgress,
        )
        .abortable();
        self.installed_scan_done = false;
        self._installed_scan_handle = Some(handle.abort_on_drop());

        let focus_task = widget::operation::focus(widget::Id::new(SEARCH_INPUT_ID));
        Task::batch([scan_task, focus_task])
    }

    fn handle_select_all_winget_search(&mut self) -> Task<Message> {
        let all_ids: Vec<String> = self
            .winget_search_results
            .iter()
            .map(|p| p.winget_id.clone())
            .collect();
        toggle_set(&mut self.winget_search_selected, all_ids);
        Task::none()
    }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "Add winget search message handlers"
```

### Task 6: Update keyboard handlers, focus, and subscription in `main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update `handle_key_confirm` (line ~1131)**

Add these arms before the `_ => Task::none()` catch-all:

```rust
            Screen::WingetSearch
                if !self.winget_search_scanning
                    && !self.winget_search_query.trim().is_empty()
                    && self.winget_search_results.is_empty() =>
            {
                self.handle_start_winget_search()
            }
            Screen::WingetSearch if !self.winget_search_selected.is_empty() => {
                self.handle_start_winget_search_install()
            }
            Screen::WingetSearchInstalling if self.winget_search_install.done => {
                self.handle_finish_winget_search_install()
            }
```

- [ ] **Step 2: Update `handle_key_escape` (line ~1153)**

Add these arms. In the existing match at lines 1155-1158, add `Screen::WingetSearch` to the line that already handles `PackageSelect | Review | UpdateSelect | Settings`:

```rust
            Screen::WingetSearch => self.handle_go_back(),
            Screen::WingetSearchInstalling if !self.winget_search_install.done => {
                self.handle_cancel_winget_search_install()
            }
            Screen::WingetSearchInstalling if self.winget_search_install.done => {
                self.handle_finish_winget_search_install()
            }
```

- [ ] **Step 3: Update `handle_focus_search` (line ~1167)**

Add `Screen::WingetSearch` to the existing match arm:

```rust
            Screen::PackageSelect | Screen::UpdateSelect | Screen::UninstallSelect | Screen::WingetSearch => {
```

- [ ] **Step 4: Update `handle_go_back` for `WingetSearch`**

Add a new arm in `handle_go_back` **before the `_ =>` catch-all** (line ~709):

```rust
            Screen::WingetSearch => {
                self._winget_search_handle = None;
                self.screen = Screen::ProfileSelect;
            }
```

- [ ] **Step 5: Update `handle_copy_log` (line ~1071)**

In `handle_copy_log`, add a `Screen::WingetSearchInstalling` arm before the `_ =>` catch-all:

```rust
                Screen::WingetSearchInstalling => &mut self.winget_search_install,
```

- [ ] **Step 6: Update `handle_select_all` (line ~980)**

Add a `Screen::WingetSearch` arm so Ctrl+A works on the search screen:

```rust
            Screen::WingetSearch => {
                return self.handle_select_all_winget_search();
            }
```

- [ ] **Step 7: Update spinner subscription (line ~1208)**

Add winget search conditions to the `spinner_active` check:

```rust
        let spinner_active = !self.installed_scan_done
            || matches!(self.screen, Screen::Installing if !self.install.done)
            || matches!(self.screen, Screen::UpdateScanning if !self.update_scan.done)
            || matches!(self.screen, Screen::Updating if !self.upgrade.done)
            || matches!(self.screen, Screen::Uninstalling if !self.uninstall.done)
            || matches!(self.screen, Screen::WingetSearch if self.winget_search_scanning)
            || matches!(self.screen, Screen::WingetSearchInstalling if !self.winget_search_install.done);
```

- [ ] **Step 8: Verify it compiles**

Run: `cargo build`

- [ ] **Step 9: Commit**

```bash
git add src/main.rs
git commit -m "Wire up keyboard, focus, back navigation, and spinner for winget search"
```

## Chunk 3: Views

### Task 7: Add `view_winget_search()` to `views.rs`

**Files:**
- Modify: `src/views.rs`

- [ ] **Step 1: Add import for `SearchPackage`**

At the top of `views.rs`, update the import from `upgrade` (line ~11) to include `SearchPackage`:

```rust
use crate::upgrade::{SearchPackage, UpgradeablePackage};
```

- [ ] **Step 2: Add `view_winget_search` method on `impl App`**

Add this method after `view_uninstalling` (line ~1177), before the closing `}` of `impl App`:

```rust
    pub(crate) fn view_winget_search(&self) -> Element<'_, Message> {
        // Header with back button
        let header = back_header("Search Winget");

        // Search input + button row
        let search_field = text_input("Search winget packages...", &self.winget_search_query)
            .id(iced::widget::Id::new(SEARCH_INPUT_ID))
            .on_input(Message::WingetSearchQueryChanged)
            .on_submit(Message::StartWingetSearch)
            .padding(8)
            .size(14)
            .width(Length::Fill);

        let mut search_btn = button(
            row![
                text(char::from(Icon::Search))
                    .size(14)
                    .font(LUCIDE_FONT),
                text("Search").size(14),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .style(continue_button_style)
        .padding([8, 16]);

        if !self.winget_search_scanning && !self.winget_search_query.trim().is_empty() {
            search_btn = search_btn.on_press(Message::StartWingetSearch);
        }

        let search_row = row![search_field, search_btn]
            .spacing(8)
            .align_y(iced::Alignment::Center);

        // Results area
        let results_content: Element<'_, Message> = if self.winget_search_scanning {
            // Scanning state
            container(
                column![spinner_indicator(
                    self.spinner_frame,
                    "Searching...".into(),
                    MUTED,
                )]
                .align_x(iced::Alignment::Center)
                .width(Length::Fill),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else if let Some(ref error) = self.winget_search_error {
            // Error state
            container(
                column![
                    text(char::from(Icon::CircleX))
                        .size(24)
                        .font(LUCIDE_FONT)
                        .color(STATUS_RED),
                    text(error).size(14).color(STATUS_RED),
                ]
                .spacing(8)
                .align_x(iced::Alignment::Center)
                .width(Length::Fill),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else if self.winget_search_results.is_empty() {
            // Empty state — either no search yet or no results
            let msg = if self.winget_search_query.is_empty() {
                "Type a query and press Enter"
            } else {
                "No results found"
            };
            container(text(msg).size(14).color(MUTED))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else {
            // Results list
            let col_headers = row![
                iced::widget::Space::new().width(30),
                text("Name").size(11).color(MUTED).width(Length::Fill),
                text("Version").size(11).color(MUTED).width(100),
                text("Package ID").size(11).color(MUTED).width(200),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .padding(padding::left(8).right(28));

            let mut pkg_list = column![].spacing(2).width(Length::Fill);

            for pkg in &self.winget_search_results {
                let is_installed = self.installed_map.contains_key(&pkg.winget_id_lower);

                if is_installed {
                    // Show installed badge, no checkbox
                    let badge = text("installed")
                        .size(11)
                        .color(STATUS_GREEN);

                    let pkg_row = row![
                        badge,
                        text(&pkg.name).size(13).color(MUTED).width(Length::Fill),
                        text(&pkg.version)
                            .size(12)
                            .color(MUTED)
                            .width(100),
                        text(&pkg.winget_id)
                            .size(11)
                            .font(iced::Font::MONOSPACE)
                            .color(MUTED)
                            .width(200),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .padding([6, 8]);

                    pkg_list = pkg_list.push(
                        container(pkg_row).width(Length::Fill),
                    );
                } else {
                    let is_checked = self
                        .winget_search_selected
                        .contains(&pkg.winget_id);
                    let id = pkg.winget_id.clone();

                    let cb = checkbox(is_checked)
                        .on_toggle(move |_| {
                            Message::ToggleWingetSearchPackage(id.clone())
                        })
                        .size(14)
                        .style(package_checkbox_style);

                    let pkg_row = row![
                        cb,
                        text(&pkg.name).size(13).width(Length::Fill),
                        text(&pkg.version)
                            .size(12)
                            .color(MUTED_FG)
                            .width(100),
                        text(&pkg.winget_id)
                            .size(11)
                            .font(iced::Font::MONOSPACE)
                            .color(MUTED)
                            .width(200),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .padding([6, 8]);

                    let row_el: Element<'_, Message> = if is_checked {
                        container(pkg_row)
                            .style(|_: &_| container::Style {
                                background: Some(iced::Background::Color(
                                    CARD_BG,
                                )),
                                border: iced::Border {
                                    radius: 6.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .width(Length::Fill)
                            .into()
                    } else {
                        container(pkg_row).width(Length::Fill).into()
                    };

                    pkg_list = pkg_list.push(row_el);
                }
            }

            column![
                col_headers,
                scrollable(pkg_list.padding(padding::right(20)))
                    .height(Length::Fill)
                    .width(Length::Fill),
            ]
            .spacing(6)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        };

        // Footer
        let selected_count = self.winget_search_selected.len();
        let footer_text = text(format!("{selected_count} selected"))
            .size(13)
            .color(MUTED);

        let mut select_all_btn = button(text("Select all").size(13))
            .style(ghost_button_style)
            .padding([6, 12]);
        if !self.winget_search_results.is_empty() {
            select_all_btn = select_all_btn.on_press(Message::SelectAllWingetSearch);
        }

        let mut install_btn = button(
            text(format!("Install selected ({selected_count})")).size(14),
        )
        .style(continue_button_style)
        .padding([8, 20]);
        if selected_count > 0 {
            install_btn = install_btn.on_press(Message::StartWingetSearchInstall);
        }

        let footer = row![
            footer_text,
            iced::widget::Space::new().width(Length::Fill),
            select_all_btn,
            install_btn,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let content = column![header, search_row, results_content, footer]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }

    pub(crate) fn view_winget_search_installing(&self) -> Element<'_, Message> {
        view_progress_screen(
            &self.winget_search_install,
            &ProgressLabels {
                verb: "Installing",
                done_label: "Installation",
                dry_run_warning: "No packages will actually be installed",
            },
            self.winget_search_queue.iter().map(|p| p.name.as_str()),
            self.dry_run,
            Message::CancelWingetSearchInstall,
            Message::FinishWingetSearchInstall,
            self.spinner_frame,
        )
    }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`

- [ ] **Step 4: Commit**

```bash
git add src/views.rs
git commit -m "Add view_winget_search and view_winget_search_installing views"
```

### Task 8: Add view dispatch and ProfileSelect entry card

**Files:**
- Modify: `src/main.rs`
- Modify: `src/views.rs`

- [ ] **Step 1: Add view dispatch arms in `main.rs`**

In `App::view()` (line ~1176), add before the closing brace:

```rust
            Screen::WingetSearch => self.view_winget_search(),
            Screen::WingetSearchInstalling => self.view_winget_search_installing(),
```

- [ ] **Step 2: Add action card to ProfileSelect in `views.rs`**

In `view_profile_select()`, find where the action cards are pushed onto `content` (line ~181-184):

```rust
        let content = content
            .push(update_card)
            .push(uninstall_card)
            .push(settings_card)
            .push(status_row);
```

Add the search card before `settings_card`:

```rust
        let search_card = action_card(
            Icon::Search,
            "Search winget",
            Some(Message::GoToWingetSearch),
        );

        let content = content
            .push(update_card)
            .push(uninstall_card)
            .push(search_card)
            .push(settings_card)
            .push(status_row);
```

- [ ] **Step 3: Verify it compiles and runs**

Run: `cargo build` then `cargo run -- --dry`

Verify:
1. "Search winget" card appears on home screen
2. Clicking it opens the search screen
3. Typing a query and pressing Enter shows fake results
4. Selecting packages and clicking Install works
5. Done button returns to search screen
6. Escape goes back to home
7. Ctrl+K focuses search

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/views.rs
git commit -m "Wire up view dispatch and add Search Winget card to home screen"
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
git commit -m "Fix clippy/fmt issues from winget search integration"
```
