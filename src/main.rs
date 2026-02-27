mod catalog;
mod install;
mod profile;
mod settings;
mod styles;
mod theme;
mod upgrade;
mod views;

use std::collections::{HashMap, HashSet};

use iced::{Element, Size, Task, Theme, clipboard, keyboard, task};

use catalog::Package;
use install::PackageStatus;
use profile::Profile;
use upgrade::UpgradeablePackage;

impl App {
    /// Check whether a package from the catalog is already installed.
    pub(crate) fn is_installed(&self, pkg: &Package) -> bool {
        pkg.winget_id
            .as_ref()
            .is_some_and(|wid| self.installed.contains_key(&wid.to_lowercase()))
    }
}

#[cfg(not(debug_assertions))]
fn ensure_elevated() {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};

    unsafe {
        if IsUserAnAdmin() != 0 {
            return;
        }

        let exe: Vec<u16> = std::env::current_exe()
            .unwrap_or_default()
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let args: String = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
        let args_w: Vec<u16> = args.encode_utf16().chain(std::iter::once(0)).collect();

        let verb: Vec<u16> = "runas\0".encode_utf16().collect();

        let result = ShellExecuteW(
            HWND::default(),
            verb.as_ptr(),
            exe.as_ptr(),
            args_w.as_ptr(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        );

        // ShellExecuteW returns > 32 on success
        if result as usize > 32 {
            std::process::exit(0);
        }

        // UAC declined — print warning and continue unelevated
        eprintln!("Warning: running without admin privileges. Some packages may fail to install.");
    }
}

#[cfg(debug_assertions)]
fn ensure_elevated() {}

fn main() -> iced::Result {
    ensure_elevated();

    let dry_run = std::env::args().any(|a| a == "--dry");

    iced::application(move || App::new(dry_run), App::update, App::view)
        .subscription(App::subscription)
        .title("Provision")
        .theme(App::theme)
        .window_size(Size::new(900.0, 600.0))
        .font(lucide_icons::LUCIDE_FONT_BYTES)
        .run()
}

/// Tracks progress for a running install or upgrade operation.
#[derive(Default)]
pub(crate) struct ProgressState {
    pub(crate) statuses: Vec<PackageStatus>,
    pub(crate) current: usize,
    pub(crate) log: Vec<String>,
    pub(crate) live_line: String,
    pub(crate) done: bool,
    pub(crate) copy_status: bool,
    pub(crate) _handle: Option<task::Handle>,
}

const LOG_CAP: usize = 200;

impl ProgressState {
    fn start(&mut self, count: usize) {
        self.statuses = vec![PackageStatus::Pending; count];
        self.current = 0;
        self.log.clear();
        self.live_line.clear();
        self.done = false;
        self.copy_status = false;
    }

    fn handle_event(
        &mut self,
        event: &install::InstallProgress,
        get_name: impl Fn(usize) -> String,
    ) {
        match event {
            install::InstallProgress::Started { index } => {
                if let Some(s) = self.statuses.get_mut(*index) {
                    *s = PackageStatus::Installing;
                }
                self.current = *index;
                self.live_line.clear();
                if *index > 0 {
                    self.log.push(String::new());
                }
                self.log.push(format!("--- {} ---", get_name(*index)));
            }
            install::InstallProgress::Log { line, .. } => {
                self.log.push(line.clone());
                self.live_line.clear();
                if self.log.len() > LOG_CAP {
                    self.log.drain(..self.log.len() - LOG_CAP);
                }
            }
            install::InstallProgress::Activity { line, .. } => {
                self.live_line = line.clone();
            }
            install::InstallProgress::Succeeded { index } => {
                if let Some(s) = self.statuses.get_mut(*index) {
                    *s = PackageStatus::Done;
                }
                self.live_line.clear();
            }
            install::InstallProgress::Failed { index, error } => {
                if let Some(s) = self.statuses.get_mut(*index) {
                    *s = PackageStatus::Failed(error.clone());
                }
                self.live_line.clear();
            }
            install::InstallProgress::Completed => {
                self.done = true;
                self._handle = None;
                self.live_line.clear();
            }
        }
    }

    fn cancel(&mut self, label: &str) {
        self._handle = None;
        self.copy_status = false;
        for s in &mut self.statuses {
            if matches!(s, PackageStatus::Installing | PackageStatus::Pending) {
                *s = PackageStatus::Cancelled;
            }
        }
        self.done = true;
        self.live_line.clear();
        self.log.push(String::new());
        self.log.push(format!("--- {label} cancelled ---"));
    }

    pub(crate) fn status_counts(&self) -> (usize, usize, usize) {
        let done = self
            .statuses
            .iter()
            .filter(|s| matches!(s, PackageStatus::Done))
            .count();
        let failed = self
            .statuses
            .iter()
            .filter(|s| matches!(s, PackageStatus::Failed(_)))
            .count();
        let cancelled = self
            .statuses
            .iter()
            .filter(|s| matches!(s, PackageStatus::Cancelled))
            .count();
        (done, failed, cancelled)
    }
}

/// Tracks state for the update-scan flow: scanning, results, and selection.
#[derive(Default)]
pub(crate) struct UpdateScanState {
    pub(crate) log: Vec<String>,
    pub(crate) live_line: String,
    pub(crate) packages: Vec<UpgradeablePackage>,
    pub(crate) selected: HashSet<String>,
    pub(crate) done: bool,
    pub(crate) error: Option<String>,
    pub(crate) _handle: Option<task::Handle>,
}

pub(crate) struct App {
    pub(crate) dry_run: bool,
    pub(crate) selected_profile: Option<Profile>,
    pub(crate) screen: Screen,
    pub(crate) catalog: Vec<Package>,
    pub(crate) selected: HashSet<String>,
    pub(crate) search: String,
    pub(crate) settings: settings::WingetSettings,
    // Install state
    pub(crate) install_queue: Vec<Package>,
    pub(crate) install: ProgressState,
    /// Installed packages detected at startup: winget_id (lowercase) -> version
    pub(crate) installed: HashMap<String, String>,
    pub(crate) installed_scan_done: bool,
    pub(crate) _installed_scan_handle: Option<task::Handle>,
    // Update scan + upgrade state
    pub(crate) update_scan: UpdateScanState,
    pub(crate) upgrade_queue: Vec<UpgradeablePackage>,
    pub(crate) upgrade: ProgressState,
    /// Transient status message for export/import feedback.
    pub(crate) selection_status: Option<String>,
}

impl App {
    fn new(dry_run: bool) -> (Self, Task<Message>) {
        let (scan_task, scan_handle) = Task::run(
            upgrade::scan_installed(dry_run),
            Message::InstalledScanProgress,
        )
        .abortable();

        (
            Self {
                dry_run,
                selected_profile: None,
                screen: Screen::default(),
                catalog: catalog::load_catalog(),
                selected: HashSet::new(),
                search: String::new(),
                settings: settings::WingetSettings::default(),
                install_queue: Vec::new(),
                install: ProgressState::default(),
                installed: HashMap::new(),
                installed_scan_done: false,
                _installed_scan_handle: Some(scan_handle.abort_on_drop()),
                update_scan: UpdateScanState::default(),
                upgrade_queue: Vec::new(),
                upgrade: ProgressState::default(),
                selection_status: None,
            },
            scan_task,
        )
    }
}

#[derive(Debug, Default)]
pub(crate) enum Screen {
    #[default]
    ProfileSelect,
    PackageSelect,
    Review,
    Installing,
    UpdateScanning,
    UpdateSelect,
    Updating,
    Settings,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    InstalledScanProgress(upgrade::InstalledScanProgress),
    ProfileSelected(Profile),
    GoBack,
    TogglePackage(String),
    SearchChanged(String),
    GoToReview,
    StartInstall,
    CancelInstall,
    InstallProgress(install::InstallProgress),
    FinishAndReset,
    StartUpdateScan,
    CancelUpdateScan,
    UpdateScanProgress(upgrade::ScanProgress),
    ToggleUpgradePackage(String),
    StartUpgrade,
    CancelUpgrade,
    UpgradeProgress(install::InstallProgress),
    FinishUpdateAndReset,
    ToggleCategory(String),
    ExportSelection,
    ExportCompleted(Result<(), String>),
    ImportSelection,
    ImportCompleted(Result<HashSet<String>, String>),
    ClearSelectionStatus,
    CopyLog(Vec<String>),
    ClearCopyStatus,
    OpenSettings,
    SetInstallMode(settings::InstallMode),
    SetScope(settings::OptionalScope),
    SetArchitecture(settings::OptionalArchitecture),
    ToggleForce(bool),
    ToggleIncludeUnknown(bool),
    ToggleIgnoreSecurityHash(bool),
    ToggleDisableInteractivity(bool),
    SetInstallLocation(String),
    KeyConfirm,
    KeyEscape,
    SelectAll,
    KeyIgnored,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InstalledScanProgress(event) => match event {
                upgrade::InstalledScanProgress::Activity { .. } => {}
                upgrade::InstalledScanProgress::Completed { packages } => {
                    for pkg in packages {
                        self.installed.insert(pkg.winget_id, pkg.version);
                    }
                    self.installed_scan_done = true;
                    self._installed_scan_handle = None;
                }
                upgrade::InstalledScanProgress::Failed { .. } => {
                    self.installed_scan_done = true;
                    self._installed_scan_handle = None;
                }
            },
            Message::ProfileSelected(profile) => {
                self.selected_profile = Some(profile);
                let mut selection = catalog::default_selection(&self.catalog, profile);
                if self.installed_scan_done {
                    for pkg in &self.catalog {
                        if self.is_installed(pkg) {
                            selection.remove(&pkg.id);
                        }
                    }
                }
                self.selected = selection;
                self.search.clear();
                self.screen = Screen::PackageSelect;
            }
            Message::GoBack => match self.screen {
                Screen::Review => {
                    self.screen = Screen::PackageSelect;
                }
                Screen::UpdateScanning => {
                    self.update_scan._handle = None;
                    self.screen = Screen::ProfileSelect;
                }
                Screen::UpdateSelect => {
                    self.screen = Screen::ProfileSelect;
                }
                Screen::Settings => {
                    self.screen = Screen::ProfileSelect;
                }
                _ => {
                    self.search.clear();
                    self.screen = Screen::ProfileSelect;
                }
            },
            Message::TogglePackage(id) => {
                if !self.selected.remove(&id) {
                    self.selected.insert(id);
                }
            }
            Message::SearchChanged(value) => {
                self.search = value;
            }
            Message::GoToReview => {
                self.screen = Screen::Review;
            }
            Message::StartInstall => {
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
                return task;
            }
            Message::CancelInstall => {
                self.install.cancel("Installation");
            }
            Message::InstallProgress(event) => {
                let queue = &self.install_queue;
                self.install.handle_event(&event, |i| {
                    let name = queue.get(i).map(|p| p.name.as_str()).unwrap_or("...");
                    format!("Installing {name}")
                });
            }
            Message::FinishAndReset => {
                self.selected_profile = None;
                self.selected.clear();
                self.search.clear();
                self.install_queue.clear();
                self.install = ProgressState::default();
                self.screen = Screen::ProfileSelect;
            }
            Message::StartUpdateScan => {
                self.update_scan = UpdateScanState::default();
                self.search.clear();
                self.screen = Screen::UpdateScanning;

                let dry = self.dry_run;
                let include_unknown = self.settings.include_unknown;
                let (task, handle) = Task::run(
                    upgrade::scan_upgrades(dry, include_unknown),
                    Message::UpdateScanProgress,
                )
                .abortable();

                self.update_scan._handle = Some(handle.abort_on_drop());
                return task;
            }
            Message::CancelUpdateScan => {
                self.update_scan._handle = None;
                self.screen = Screen::ProfileSelect;
            }
            Message::UpdateScanProgress(event) => match event {
                upgrade::ScanProgress::Activity { line } => {
                    self.update_scan.live_line = line;
                }
                upgrade::ScanProgress::Log { line } => {
                    self.update_scan.log.push(line);
                    self.update_scan.live_line.clear();
                    if self.update_scan.log.len() > LOG_CAP {
                        self.update_scan
                            .log
                            .drain(..self.update_scan.log.len() - LOG_CAP);
                    }
                }
                upgrade::ScanProgress::Completed { packages } => {
                    self.update_scan.done = true;
                    self.update_scan.live_line.clear();
                    self.update_scan._handle = None;
                    if packages.is_empty() {
                        self.update_scan.packages = packages;
                    } else {
                        self.update_scan.selected =
                            packages.iter().map(|p| p.winget_id.clone()).collect();
                        self.update_scan.packages = packages;
                        self.screen = Screen::UpdateSelect;
                    }
                }
                upgrade::ScanProgress::Failed { error } => {
                    self.update_scan.done = true;
                    self.update_scan.error = Some(error);
                    self.update_scan.live_line.clear();
                    self.update_scan._handle = None;
                }
            },
            Message::ToggleUpgradePackage(id) => {
                if !self.update_scan.selected.remove(&id) {
                    self.update_scan.selected.insert(id);
                }
            }
            Message::StartUpgrade => {
                let queue: Vec<UpgradeablePackage> = self
                    .update_scan
                    .packages
                    .iter()
                    .filter(|p| self.update_scan.selected.contains(&p.winget_id))
                    .cloned()
                    .collect();

                self.upgrade.start(queue.len());
                self.upgrade_queue = queue.clone();
                self.screen = Screen::Updating;

                let dry = self.dry_run;
                let extra = self.settings.install_args();
                let (task, handle) = Task::run(
                    upgrade::upgrade_all(queue, dry, extra),
                    Message::UpgradeProgress,
                )
                .abortable();

                self.upgrade._handle = Some(handle.abort_on_drop());
                return task;
            }
            Message::CancelUpgrade => {
                self.upgrade.cancel("Upgrade");
            }
            Message::UpgradeProgress(event) => {
                let queue = &self.upgrade_queue;
                self.upgrade.handle_event(&event, |i| {
                    let name = queue.get(i).map(|p| p.name.as_str()).unwrap_or("...");
                    format!("Upgrading {name}")
                });
            }
            Message::FinishUpdateAndReset => {
                self.update_scan = UpdateScanState::default();
                self.upgrade_queue.clear();
                self.upgrade = ProgressState::default();
                self.screen = Screen::ProfileSelect;
            }
            Message::ToggleCategory(cat) => {
                let cat_ids: Vec<String> = self
                    .catalog
                    .iter()
                    .filter(|p| p.category == cat)
                    .map(|p| p.id.clone())
                    .collect();
                let all_selected = cat_ids.iter().all(|id| self.selected.contains(id));
                if all_selected {
                    for id in &cat_ids {
                        self.selected.remove(id);
                    }
                } else {
                    for id in cat_ids {
                        self.selected.insert(id);
                    }
                }
            }
            Message::ExportSelection => {
                let selected = self.selected.clone();
                return Task::perform(
                    catalog::export_selection(selected),
                    Message::ExportCompleted,
                );
            }
            Message::ExportCompleted(result) => {
                match result {
                    Ok(()) => {
                        self.selection_status = Some("Selection exported".into());
                    }
                    Err(msg) if msg.is_empty() => return Task::none(),
                    Err(msg) => {
                        self.selection_status = Some(format!("Export failed: {msg}"));
                    }
                }
                return Task::perform(
                    async {
                        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                    },
                    |_| Message::ClearSelectionStatus,
                );
            }
            Message::ImportSelection => {
                let valid_ids: HashSet<String> =
                    self.catalog.iter().map(|p| p.id.clone()).collect();
                return Task::perform(
                    catalog::import_selection(valid_ids),
                    Message::ImportCompleted,
                );
            }
            Message::ImportCompleted(result) => {
                match result {
                    Ok(ids) => {
                        let count = ids.len();
                        self.selected = ids;
                        self.selection_status = Some(format!("{count} packages imported"));
                    }
                    Err(msg) if msg.is_empty() => return Task::none(),
                    Err(msg) => {
                        self.selection_status = Some(format!("Import failed: {msg}"));
                    }
                }
                return Task::perform(
                    async {
                        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                    },
                    |_| Message::ClearSelectionStatus,
                );
            }
            Message::ClearSelectionStatus => {
                self.selection_status = None;
            }
            Message::CopyLog(lines) => {
                // Build clipboard text: summary header + log lines
                let state = match self.screen {
                    Screen::Updating => &self.upgrade,
                    _ => &self.install,
                };
                let (done, failed, cancelled) = state.status_counts();
                let mut header = format!("{done} succeeded, {failed} failed");
                if cancelled > 0 {
                    header.push_str(&format!(", {cancelled} cancelled"));
                }
                let text = format!("{header}\n\n{}", lines.join("\n"));

                match self.screen {
                    Screen::Updating => self.upgrade.copy_status = true,
                    _ => self.install.copy_status = true,
                }

                return Task::batch([
                    clipboard::write(text),
                    Task::perform(
                        async {
                            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                        },
                        |_| Message::ClearCopyStatus,
                    ),
                ]);
            }
            Message::ClearCopyStatus => {
                self.install.copy_status = false;
                self.upgrade.copy_status = false;
            }
            Message::OpenSettings => {
                self.screen = Screen::Settings;
            }
            Message::SetInstallMode(mode) => {
                self.settings.install_mode = mode;
            }
            Message::SetScope(opt) => {
                self.settings.scope = opt.0;
            }
            Message::SetArchitecture(opt) => {
                self.settings.architecture = opt.0;
            }
            Message::ToggleForce(v) => {
                self.settings.force = v;
            }
            Message::ToggleIncludeUnknown(v) => {
                self.settings.include_unknown = v;
            }
            Message::ToggleIgnoreSecurityHash(v) => {
                self.settings.ignore_security_hash = v;
            }
            Message::ToggleDisableInteractivity(v) => {
                self.settings.disable_interactivity = v;
            }
            Message::SetInstallLocation(v) => {
                self.settings.install_location = v;
            }
            Message::KeyIgnored => {}
            Message::KeyConfirm => {
                return match self.screen {
                    Screen::PackageSelect if !self.selected.is_empty() => {
                        self.update(Message::GoToReview)
                    }
                    Screen::Review => self.update(Message::StartInstall),
                    Screen::Installing if self.install.done => self.update(Message::FinishAndReset),
                    Screen::UpdateScanning if self.update_scan.done => self.update(Message::GoBack),
                    Screen::UpdateSelect if !self.update_scan.selected.is_empty() => {
                        self.update(Message::StartUpgrade)
                    }
                    Screen::Updating if self.upgrade.done => {
                        self.update(Message::FinishUpdateAndReset)
                    }
                    _ => Task::none(),
                };
            }
            Message::KeyEscape => {
                return match self.screen {
                    Screen::PackageSelect
                    | Screen::Review
                    | Screen::UpdateSelect
                    | Screen::Settings => self.update(Message::GoBack),
                    Screen::Installing if !self.install.done => self.update(Message::CancelInstall),
                    Screen::UpdateScanning if !self.update_scan.done => {
                        self.update(Message::CancelUpdateScan)
                    }
                    Screen::Updating if !self.upgrade.done => self.update(Message::CancelUpgrade),
                    _ => Task::none(),
                };
            }
            Message::SelectAll => {
                let search_lower = self.search.to_lowercase();
                match self.screen {
                    Screen::PackageSelect => {
                        let visible_ids: Vec<String> = self
                            .catalog
                            .iter()
                            .filter(|p| {
                                search_lower.is_empty()
                                    || p.name.to_lowercase().contains(&search_lower)
                                    || p.description.to_lowercase().contains(&search_lower)
                            })
                            .map(|p| p.id.clone())
                            .collect();
                        let all_selected = visible_ids.iter().all(|id| self.selected.contains(id));
                        if all_selected {
                            for id in &visible_ids {
                                self.selected.remove(id);
                            }
                        } else {
                            for id in visible_ids {
                                self.selected.insert(id);
                            }
                        }
                    }
                    Screen::UpdateSelect => {
                        let visible_ids: Vec<String> = self
                            .update_scan
                            .packages
                            .iter()
                            .filter(|p| {
                                search_lower.is_empty()
                                    || p.name.to_lowercase().contains(&search_lower)
                                    || p.winget_id.to_lowercase().contains(&search_lower)
                            })
                            .map(|p| p.winget_id.clone())
                            .collect();
                        let all_selected = visible_ids
                            .iter()
                            .all(|id| self.update_scan.selected.contains(id));
                        if all_selected {
                            for id in &visible_ids {
                                self.update_scan.selected.remove(id);
                            }
                        } else {
                            for id in visible_ids {
                                self.update_scan.selected.insert(id);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        match self.screen {
            Screen::ProfileSelect => self.view_profile_select(),
            Screen::PackageSelect => self.view_package_select(),
            Screen::Review => self.view_review(),
            Screen::Installing => self.view_installing(),
            Screen::UpdateScanning => self.view_update_scanning(),
            Screen::UpdateSelect => self.view_update_select(),
            Screen::Updating => self.view_updating(),
            Screen::Settings => self.view_settings(),
        }
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        keyboard::listen().map(|event| match event {
            keyboard::Event::KeyPressed { key, modifiers, .. } => match key.as_ref() {
                keyboard::Key::Named(keyboard::key::Named::Enter) if modifiers.is_empty() => {
                    Message::KeyConfirm
                }
                keyboard::Key::Named(keyboard::key::Named::Escape) if modifiers.is_empty() => {
                    Message::KeyEscape
                }
                keyboard::Key::Character("a") if modifiers.command() => Message::SelectAll,
                _ => Message::KeyIgnored,
            },
            _ => Message::KeyIgnored,
        })
    }

    fn theme(&self) -> Theme {
        theme::default()
    }
}
