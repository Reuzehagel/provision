mod catalog;
mod install;
mod profile;
mod settings;
mod styles;
mod sysinfo;
mod theme;
mod uninstall;
mod upgrade;
mod version;
mod views;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use std::time::{Duration, Instant};

use iced::{Element, Size, Subscription, Task, Theme, clipboard, keyboard, task, time, widget};

use catalog::{CatalogSource, Package};
use install::PackageStatus;
use profile::Profile;
use upgrade::UpgradeablePackage;

pub(crate) const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
pub(crate) const SEARCH_INPUT_ID: &str = "search_input";

impl App {
    fn clear_search(&mut self) {
        self.search.clear();
        self.search_lower.clear();
    }

    /// Check whether a package from the catalog is already installed.
    pub(crate) fn is_installed(&self, pkg: &Package) -> bool {
        pkg.winget_id_lower
            .as_ref()
            .is_some_and(|wid| self.installed_map.contains_key(wid))
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

const ICON_RGBA: &[u8] = include_bytes!("../assets/icon.rgba");

fn main() -> iced::Result {
    ensure_elevated();

    let dry_run = std::env::args().any(|a| a == "--dry");
    let icon = iced::window::icon::from_rgba(ICON_RGBA.to_vec(), 128, 128).ok();

    iced::application(move || App::new(dry_run), App::update, App::view)
        .subscription(App::subscription)
        .title("Provision")
        .theme(App::theme)
        .window_size(Size::new(900.0, 605.0))
        .window(iced::window::Settings {
            icon,
            ..Default::default()
        })
        .font(lucide_icons::LUCIDE_FONT_BYTES)
        .run()
}

const LOG_CAP: usize = 200;

/// Capped log buffer shared by progress screens and the update-scan screen.
/// Maintains a pre-joined `joined` cache to avoid re-joining 200 lines per frame.
/// Uses `RefCell` for the cache so `joined()` works through `&self` (needed by Iced's `view`).
pub(crate) struct LogBuffer {
    pub(crate) lines: Vec<String>,
    pub(crate) live_line: String,
    /// Pre-joined text of `lines` + `live_line`, rebuilt lazily via `joined()`.
    joined_cache: RefCell<String>,
    dirty: std::cell::Cell<bool>,
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            live_line: String::new(),
            joined_cache: RefCell::new(String::new()),
            dirty: std::cell::Cell::new(false),
        }
    }
}

impl LogBuffer {
    pub(crate) fn push(&mut self, line: String) {
        self.lines.push(line);
        self.live_line.clear();
        self.dirty.set(true);
        if self.lines.len() > LOG_CAP {
            self.lines.drain(..self.lines.len() - LOG_CAP);
        }
    }

    pub(crate) fn set_live(&mut self, line: String) {
        self.dirty.set(true);
        self.live_line = line;
    }

    pub(crate) fn clear(&mut self) {
        self.lines.clear();
        self.live_line.clear();
        self.joined_cache.borrow_mut().clear();
        self.dirty.set(false);
    }

    /// Return the full terminal text (lines + live_line), using a cache to
    /// avoid re-joining every frame. Safe to call from `&self` (view methods).
    pub(crate) fn joined(&self) -> std::cell::Ref<'_, String> {
        if self.dirty.get() {
            let mut cache = self.joined_cache.borrow_mut();
            cache.clear();
            for (i, line) in self.lines.iter().enumerate() {
                if i > 0 {
                    cache.push('\n');
                }
                cache.push_str(line);
            }
            if !self.live_line.is_empty() {
                if !cache.is_empty() {
                    cache.push('\n');
                }
                cache.push_str(&self.live_line);
            }
            self.dirty.set(false);
        }
        self.joined_cache.borrow()
    }
}

/// Tracks progress for a running install or upgrade operation.
#[derive(Default)]
pub(crate) struct ProgressState {
    pub(crate) statuses: Vec<PackageStatus>,
    pub(crate) current: usize,
    pub(crate) log: LogBuffer,
    pub(crate) done: bool,
    pub(crate) copy_status: bool,
    pub(crate) started_at: Option<Instant>,
    pub(crate) _handle: Option<task::Handle>,
}

impl ProgressState {
    fn start(&mut self, count: usize) {
        self.statuses = vec![PackageStatus::Pending; count];
        self.current = 0;
        self.log.clear();
        self.done = false;
        self.copy_status = false;
        self.started_at = Some(Instant::now());
    }

    fn handle_event(
        &mut self,
        event: install::InstallProgress,
        get_name: impl Fn(usize) -> String,
    ) {
        match event {
            install::InstallProgress::Started { index } => {
                if let Some(s) = self.statuses.get_mut(index) {
                    *s = PackageStatus::Installing;
                }
                self.current = index;
                self.log.live_line.clear();
                self.log.dirty.set(true);
                if index > 0 {
                    self.log.push(String::new());
                }
                self.log.push(format!("--- {} ---", get_name(index)));
            }
            install::InstallProgress::Log { line, .. } => {
                self.log.push(line);
            }
            install::InstallProgress::Activity { line, .. } => {
                self.log.set_live(line);
            }
            install::InstallProgress::Succeeded { index } => {
                if let Some(s) = self.statuses.get_mut(index) {
                    *s = PackageStatus::Done;
                }
                self.log.live_line.clear();
                self.log.dirty.set(true);
            }
            install::InstallProgress::Failed { index, error } => {
                if let Some(s) = self.statuses.get_mut(index) {
                    *s = PackageStatus::Failed(error);
                }
                self.log.live_line.clear();
                self.log.dirty.set(true);
            }
            install::InstallProgress::Completed => {
                self.done = true;
                self._handle = None;
                self.log.live_line.clear();
                self.log.dirty.set(true);
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
        self.log.live_line.clear();
        self.log.push(String::new());
        self.log.push(format!("--- {label} cancelled ---"));
    }

    pub(crate) fn elapsed_display(&self) -> String {
        let secs = self.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        if secs < 60 {
            format!("{secs}s")
        } else {
            format!("{}m {}s", secs / 60, secs % 60)
        }
    }

    pub(crate) fn status_counts(&self) -> (usize, usize, usize) {
        let (mut done, mut failed, mut cancelled) = (0, 0, 0);
        for s in &self.statuses {
            match s {
                PackageStatus::Done => done += 1,
                PackageStatus::Failed(_) => failed += 1,
                PackageStatus::Cancelled => cancelled += 1,
                _ => {}
            }
        }
        (done, failed, cancelled)
    }
}

/// Tracks state for the update-scan flow: scanning, results, and selection.
#[derive(Default)]
pub(crate) struct UpdateScanState {
    pub(crate) log: LogBuffer,
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
    pub(crate) catalog_source: CatalogSource,
    pub(crate) categories: Vec<String>,
    pub(crate) selected: HashSet<String>,
    pub(crate) search: String,
    pub(crate) search_lower: String,
    pub(crate) settings: settings::WingetSettings,
    pub(crate) settings_tab: settings::SettingsTab,
    // Install state
    pub(crate) install_queue: Vec<Package>,
    pub(crate) install: ProgressState,
    /// Full installed package data from winget list (for uninstall screen)
    pub(crate) installed_packages: Vec<upgrade::InstalledPackage>,
    /// Installed packages: winget_id (lowercase) -> version (for O(1) is_installed lookups)
    pub(crate) installed_map: HashMap<String, String>,
    pub(crate) installed_scan_done: bool,
    pub(crate) _installed_scan_handle: Option<task::Handle>,
    // Uninstall state
    pub(crate) uninstall_selected: HashSet<String>,
    pub(crate) uninstall_queue: Vec<upgrade::InstalledPackage>,
    pub(crate) uninstall: ProgressState,
    pub(crate) size_scan_done: bool,
    // Update scan + upgrade state
    pub(crate) update_scan: UpdateScanState,
    pub(crate) upgrade_queue: Vec<UpgradeablePackage>,
    pub(crate) upgrade: ProgressState,
    /// Transient status message for export/import feedback.
    pub(crate) selection_status: Option<String>,
    // Version check state
    pub(crate) latest_release: Option<version::LatestRelease>,
    pub(crate) version_check_in_progress: bool,
    // System info
    pub(crate) system_info: sysinfo::SystemInfo,
    // Spinner animation
    pub(crate) spinner_frame: usize,
    // Winget search state
    pub(crate) winget_search_query: String,
    pub(crate) winget_search_results: Vec<upgrade::SearchPackage>,
    pub(crate) winget_search_selected: HashSet<String>,
    pub(crate) winget_search_scanning: bool,
    pub(crate) winget_search_error: Option<String>,
    pub(crate) winget_search_queue: Vec<upgrade::SearchPackage>,
    pub(crate) winget_search_install: ProgressState,
    pub(crate) _winget_search_handle: Option<task::Handle>,
}

impl App {
    fn new(dry_run: bool) -> (Self, Task<Message>) {
        let (scan_task, scan_handle) = Task::run(
            upgrade::scan_installed(dry_run),
            Message::InstalledScanProgress,
        )
        .abortable();

        let catalog_task = Task::perform(
            catalog::fetch_remote_catalog(dry_run),
            Message::CatalogFetched,
        );

        let version_task = if dry_run {
            Task::none()
        } else {
            Task::perform(
                version::check_latest_release(false),
                Message::VersionCheckCompleted,
            )
        };

        let catalog = catalog::load_catalog();
        let categories = catalog::categories(&catalog);

        (
            Self {
                dry_run,
                selected_profile: None,
                screen: Screen::default(),
                catalog,
                catalog_source: CatalogSource::Embedded,
                categories,
                selected: HashSet::new(),
                search: String::new(),
                search_lower: String::new(),
                settings: settings::load_settings(),
                settings_tab: settings::SettingsTab::default(),
                install_queue: Vec::new(),
                install: ProgressState::default(),
                installed_packages: Vec::new(),
                installed_map: HashMap::new(),
                installed_scan_done: false,
                _installed_scan_handle: Some(scan_handle.abort_on_drop()),
                uninstall_selected: HashSet::new(),
                uninstall_queue: Vec::new(),
                uninstall: ProgressState::default(),
                size_scan_done: false,
                update_scan: UpdateScanState::default(),
                upgrade_queue: Vec::new(),
                upgrade: ProgressState::default(),
                selection_status: None,
                system_info: sysinfo::gather(),
                latest_release: None,
                version_check_in_progress: !dry_run,
                spinner_frame: 0,
                winget_search_query: String::new(),
                winget_search_results: Vec::new(),
                winget_search_selected: HashSet::new(),
                winget_search_scanning: false,
                winget_search_error: None,
                winget_search_queue: Vec::new(),
                winget_search_install: ProgressState::default(),
                _winget_search_handle: None,
            },
            Task::batch([scan_task, catalog_task, version_task]),
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
    UninstallSelect,
    UninstallReview,
    Uninstalling,
    WingetSearch,
    WingetSearchInstalling,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    CatalogFetched(Result<(Vec<Package>, CatalogSource), String>),
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
    CopyLog,
    ClearCopyStatus,
    OpenSettings,
    SetSettingsTab(settings::SettingsTab),
    SetInstallMode(settings::InstallMode),
    SetScope(settings::OptionalScope),
    SetArchitecture(settings::OptionalArchitecture),
    ToggleForce(bool),
    ToggleIncludeUnknown(bool),
    ToggleIgnoreSecurityHash(bool),
    ToggleDisableInteractivity(bool),
    SetInstallLocation(String),
    VersionCheckCompleted(Result<version::LatestRelease, String>),
    CheckForAppUpdate,
    OpenReleasePage,
    DismissUpdateBanner,
    GoToUninstall,
    ToggleUninstallPackage(String),
    GoToUninstallReview,
    StartUninstall,
    CancelUninstall,
    UninstallProgress(install::InstallProgress),
    FinishUninstallAndReset,
    SizeScanResult(Vec<(String, u64)>),
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
    KeyConfirm,
    KeyEscape,
    FocusSearch,
    SelectAll,
    SpinnerTick,
    KeyIgnored,
    #[allow(dead_code)]
    Noop(()),
}

/// Fire a message after a 4-second delay (used for clearing transient UI feedback).
fn delayed_clear(msg: Message) -> Task<Message> {
    Task::perform(
        async { tokio::time::sleep(std::time::Duration::from_secs(4)).await },
        move |_| msg,
    )
}

/// Toggle a set of IDs: if all are selected, deselect all; otherwise select all.
fn toggle_set(set: &mut HashSet<String>, ids: Vec<String>) {
    let all_selected = ids.iter().all(|id| set.contains(id));
    if all_selected {
        for id in &ids {
            set.remove(id);
        }
    } else {
        set.extend(ids);
    }
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ── Domain handlers ──────────────────────────────────────
            Message::CatalogFetched(r) => self.handle_catalog_fetched(r),
            Message::InstalledScanProgress(e) => self.handle_installed_scan_progress(e),
            Message::ProfileSelected(p) => self.handle_profile_selected(p),
            Message::GoBack => self.handle_go_back(),
            Message::StartInstall => self.handle_start_install(),
            Message::CancelInstall => self.handle_cancel_install(),
            Message::InstallProgress(e) => self.handle_install_progress(e),
            Message::FinishAndReset => self.handle_finish_and_reset(),
            Message::StartUpdateScan => self.handle_start_update_scan(),
            Message::CancelUpdateScan => self.handle_cancel_update_scan(),
            Message::UpdateScanProgress(e) => self.handle_update_scan_progress(e),
            Message::StartUpgrade => self.handle_start_upgrade(),
            Message::CancelUpgrade => self.handle_cancel_upgrade(),
            Message::UpgradeProgress(e) => self.handle_upgrade_progress(e),
            Message::FinishUpdateAndReset => self.handle_finish_update_and_reset(),
            Message::GoToUninstall => self.handle_go_to_uninstall(),
            Message::ToggleUninstallPackage(id) => self.handle_toggle_uninstall_package(id),
            Message::GoToUninstallReview => self.handle_go_to_uninstall_review(),
            Message::StartUninstall => self.handle_start_uninstall(),
            Message::CancelUninstall => self.handle_cancel_uninstall(),
            Message::UninstallProgress(event) => self.handle_uninstall_progress(event),
            Message::FinishUninstallAndReset => self.handle_finish_uninstall_and_reset(),
            Message::SizeScanResult(sizes) => self.handle_size_scan_result(sizes),
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
            Message::ToggleCategory(cat) => self.handle_toggle_category(cat),
            Message::SelectAll => self.handle_select_all(),
            Message::ExportSelection => self.handle_export_selection(),
            Message::ExportCompleted(r) => self.handle_export_completed(r),
            Message::ImportSelection => self.handle_import_selection(),
            Message::ImportCompleted(r) => self.handle_import_completed(r),
            Message::CopyLog => self.handle_copy_log(),
            Message::VersionCheckCompleted(r) => self.handle_version_check_completed(r),
            Message::CheckForAppUpdate => self.handle_check_for_app_update(),
            Message::OpenReleasePage => self.handle_open_release_page(),
            Message::DismissUpdateBanner => {
                self.latest_release = None;
                Task::none()
            }
            Message::KeyConfirm => self.handle_key_confirm(),
            Message::KeyEscape => self.handle_key_escape(),
            Message::FocusSearch => self.handle_focus_search(),
            // ── Inline one-liners ────────────────────────────────────
            Message::TogglePackage(id) => {
                if !self.selected.remove(&id) {
                    self.selected.insert(id);
                }
                Task::none()
            }
            Message::ToggleUpgradePackage(id) => {
                if !self.update_scan.selected.remove(&id) {
                    self.update_scan.selected.insert(id);
                }
                Task::none()
            }
            Message::SearchChanged(v) => {
                self.search_lower = v.to_lowercase();
                self.search = v;
                Task::none()
            }
            Message::GoToReview => {
                self.screen = Screen::Review;
                Task::none()
            }
            Message::OpenSettings => {
                self.settings_tab = settings::SettingsTab::default();
                self.screen = Screen::Settings;
                Task::none()
            }
            Message::SetSettingsTab(tab) => {
                self.settings_tab = tab;
                Task::none()
            }
            Message::ClearSelectionStatus => {
                self.selection_status = None;
                Task::none()
            }
            Message::ClearCopyStatus => {
                self.install.copy_status = false;
                self.upgrade.copy_status = false;
                self.uninstall.copy_status = false;
                Task::none()
            }
            Message::SetInstallMode(mode) => {
                self.settings.install_mode = mode;
                self.save_settings()
            }
            Message::SetScope(opt) => {
                self.settings.scope = opt.0;
                self.save_settings()
            }
            Message::SetArchitecture(opt) => {
                self.settings.architecture = opt.0;
                self.save_settings()
            }
            Message::ToggleForce(v) => {
                self.settings.force = v;
                self.save_settings()
            }
            Message::ToggleIncludeUnknown(v) => {
                self.settings.include_unknown = v;
                self.save_settings()
            }
            Message::ToggleIgnoreSecurityHash(v) => {
                self.settings.ignore_security_hash = v;
                self.save_settings()
            }
            Message::ToggleDisableInteractivity(v) => {
                self.settings.disable_interactivity = v;
                self.save_settings()
            }
            Message::SetInstallLocation(v) => {
                self.settings.install_location = v;
                Task::none()
            }
            Message::SpinnerTick => {
                self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
                Task::none()
            }
            Message::KeyIgnored | Message::Noop(()) => Task::none(),
        }
    }

    /// Serialize current settings and fire an async write (best-effort).
    fn save_settings(&self) -> Task<Message> {
        let Some(content) = settings::serialize_settings(&self.settings) else {
            return Task::none();
        };
        Task::perform(settings::save_settings(content), Message::Noop)
    }

    // ── Navigation & lifecycle ───────────────────────────────────

    fn handle_catalog_fetched(
        &mut self,
        result: Result<(Vec<Package>, CatalogSource), String>,
    ) -> Task<Message> {
        if let Ok((packages, source)) = result {
            self.categories = catalog::categories(&packages);
            self.catalog = packages;
            self.catalog_source = source;
            let valid_ids: HashSet<&str> = self.catalog.iter().map(|p| p.id.as_str()).collect();
            self.selected.retain(|id| valid_ids.contains(id.as_str()));
        }
        Task::none()
    }

    fn handle_installed_scan_progress(
        &mut self,
        event: upgrade::InstalledScanProgress,
    ) -> Task<Message> {
        match event {
            upgrade::InstalledScanProgress::Activity { .. } => {}
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
                return Task::perform(uninstall::scan_sizes(pkgs), Message::SizeScanResult);
            }
            upgrade::InstalledScanProgress::Failed { .. } => {
                self.installed_scan_done = true;
                self._installed_scan_handle = None;
            }
        }
        Task::none()
    }

    fn handle_profile_selected(&mut self, profile: Profile) -> Task<Message> {
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
        self.clear_search();
        self.screen = Screen::PackageSelect;
        Task::none()
    }

    fn handle_go_back(&mut self) -> Task<Message> {
        match self.screen {
            Screen::Review => {
                self.screen = Screen::PackageSelect;
            }
            Screen::UpdateScanning => {
                self.update_scan._handle = None;
                self.screen = Screen::ProfileSelect;
            }
            Screen::Settings => {
                self.screen = Screen::ProfileSelect;
                return self.save_settings();
            }
            Screen::UpdateSelect => {
                self.screen = Screen::ProfileSelect;
            }
            Screen::UninstallSelect => {
                self.screen = Screen::ProfileSelect;
            }
            Screen::WingetSearch => {
                self._winget_search_handle = None;
                self.screen = Screen::ProfileSelect;
            }
            Screen::UninstallReview => {
                self.screen = Screen::UninstallSelect;
            }
            _ => {
                self.clear_search();
                self.screen = Screen::ProfileSelect;
            }
        }
        Task::none()
    }

    fn handle_finish_and_reset(&mut self) -> Task<Message> {
        self.selected_profile = None;
        self.selected.clear();
        self.clear_search();
        self.install_queue.clear();
        self.install = ProgressState::default();
        self.screen = Screen::ProfileSelect;
        Task::none()
    }

    fn handle_finish_update_and_reset(&mut self) -> Task<Message> {
        self.update_scan = UpdateScanState::default();
        self.upgrade_queue.clear();
        self.upgrade = ProgressState::default();
        self.screen = Screen::ProfileSelect;
        Task::none()
    }

    // ── Install flow ─────────────────────────────────────────────

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

    fn handle_cancel_install(&mut self) -> Task<Message> {
        self.install.cancel("Installation");
        Task::none()
    }

    fn handle_install_progress(&mut self, event: install::InstallProgress) -> Task<Message> {
        let queue = &self.install_queue;
        self.install.handle_event(event, |i| {
            let name = queue.get(i).map(|p| p.name.as_str()).unwrap_or("...");
            format!("Installing {name}")
        });
        Task::none()
    }

    // ── Update scan flow ─────────────────────────────────────────

    fn handle_start_update_scan(&mut self) -> Task<Message> {
        self.update_scan = UpdateScanState::default();
        self.clear_search();
        self.screen = Screen::UpdateScanning;

        let dry = self.dry_run;
        let include_unknown = self.settings.include_unknown;
        let (task, handle) = Task::run(
            upgrade::scan_upgrades(dry, include_unknown),
            Message::UpdateScanProgress,
        )
        .abortable();

        self.update_scan._handle = Some(handle.abort_on_drop());
        task
    }

    fn handle_cancel_update_scan(&mut self) -> Task<Message> {
        self.update_scan._handle = None;
        self.screen = Screen::ProfileSelect;
        Task::none()
    }

    fn handle_update_scan_progress(&mut self, event: upgrade::ScanProgress) -> Task<Message> {
        match event {
            upgrade::ScanProgress::Activity { line } => {
                self.update_scan.log.set_live(line);
            }
            upgrade::ScanProgress::Log { line } => {
                self.update_scan.log.push(line);
            }
            upgrade::ScanProgress::Completed { packages } => {
                self.update_scan.done = true;
                self.update_scan.log.live_line.clear();
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
                self.update_scan.log.live_line.clear();
                self.update_scan._handle = None;
            }
        }
        Task::none()
    }

    // ── Upgrade flow ─────────────────────────────────────────────

    fn handle_start_upgrade(&mut self) -> Task<Message> {
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
        task
    }

    fn handle_cancel_upgrade(&mut self) -> Task<Message> {
        self.upgrade.cancel("Upgrade");
        Task::none()
    }

    fn handle_upgrade_progress(&mut self, event: install::InstallProgress) -> Task<Message> {
        let queue = &self.upgrade_queue;
        self.upgrade.handle_event(event, |i| {
            let name = queue.get(i).map(|p| p.name.as_str()).unwrap_or("...");
            format!("Upgrading {name}")
        });
        Task::none()
    }

    // ── Uninstall flow ────────────────────────────────────────────

    fn handle_go_to_uninstall(&mut self) -> Task<Message> {
        self.clear_search();
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

    fn handle_uninstall_progress(&mut self, event: install::InstallProgress) -> Task<Message> {
        let queue = &self.uninstall_queue;
        self.uninstall.handle_event(event, |i| {
            let name = queue.get(i).map(|p| p.name.as_str()).unwrap_or("...");
            format!("Removing {name}")
        });
        Task::none()
    }

    fn handle_finish_uninstall_and_reset(&mut self) -> Task<Message> {
        self.clear_search();
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

    // ── Selection ────────────────────────────────────────────────

    fn handle_toggle_category(&mut self, cat: String) -> Task<Message> {
        let cat_ids: Vec<String> = self
            .catalog
            .iter()
            .filter(|p| p.category == cat)
            .map(|p| p.id.clone())
            .collect();
        toggle_set(&mut self.selected, cat_ids);
        Task::none()
    }

    fn handle_select_all(&mut self) -> Task<Message> {
        let sl = self.search_lower.as_str();
        match self.screen {
            Screen::PackageSelect => {
                let visible_ids: Vec<String> = self
                    .catalog
                    .iter()
                    .filter(|p| {
                        sl.is_empty() || p.name_lower.contains(sl) || p.desc_lower.contains(sl)
                    })
                    .map(|p| p.id.clone())
                    .collect();
                toggle_set(&mut self.selected, visible_ids);
            }
            Screen::UpdateSelect => {
                let visible_ids: Vec<String> = self
                    .update_scan
                    .packages
                    .iter()
                    .filter(|p| {
                        sl.is_empty() || p.name_lower.contains(sl) || p.winget_id_lower.contains(sl)
                    })
                    .map(|p| p.winget_id.clone())
                    .collect();
                toggle_set(&mut self.update_scan.selected, visible_ids);
            }
            Screen::UninstallSelect => {
                let filtered: Vec<String> = self
                    .installed_packages
                    .iter()
                    .filter(|p| {
                        sl.is_empty() || p.name_lower.contains(sl) || p.winget_id_lower.contains(sl)
                    })
                    .map(|p| p.winget_id_lower.clone())
                    .collect();
                toggle_set(&mut self.uninstall_selected, filtered);
            }
            Screen::WingetSearch => {
                return self.handle_select_all_winget_search();
            }
            _ => {}
        }
        Task::none()
    }

    // ── Export / import ──────────────────────────────────────────

    fn handle_export_selection(&mut self) -> Task<Message> {
        let selected = self.selected.clone();
        Task::perform(
            catalog::export_selection(selected),
            Message::ExportCompleted,
        )
    }

    fn handle_export_completed(&mut self, result: Result<(), String>) -> Task<Message> {
        match result {
            Ok(()) => {
                self.selection_status = Some("Selection exported".into());
            }
            Err(msg) if msg.is_empty() => return Task::none(),
            Err(msg) => {
                self.selection_status = Some(format!("Export failed: {msg}"));
            }
        }
        delayed_clear(Message::ClearSelectionStatus)
    }

    fn handle_import_selection(&mut self) -> Task<Message> {
        let valid_ids: HashSet<String> = self.catalog.iter().map(|p| p.id.clone()).collect();
        Task::perform(
            catalog::import_selection(valid_ids),
            Message::ImportCompleted,
        )
    }

    fn handle_import_completed(
        &mut self,
        result: Result<HashSet<String>, String>,
    ) -> Task<Message> {
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
        delayed_clear(Message::ClearSelectionStatus)
    }

    fn handle_copy_log(&mut self) -> Task<Message> {
        let state = match self.screen {
            Screen::Updating => &mut self.upgrade,
            Screen::Uninstalling => &mut self.uninstall,
            Screen::WingetSearchInstalling => &mut self.winget_search_install,
            _ => &mut self.install,
        };
        let (done, failed, cancelled) = state.status_counts();
        let mut header = format!("{done} succeeded, {failed} failed");
        if cancelled > 0 {
            header.push_str(&format!(", {cancelled} cancelled"));
        }
        let text = format!("{header}\n\n{}", state.log.lines.join("\n"));
        state.copy_status = true;

        Task::batch([
            clipboard::write(text),
            delayed_clear(Message::ClearCopyStatus),
        ])
    }

    // ── Version check ────────────────────────────────────────────

    fn handle_version_check_completed(
        &mut self,
        result: Result<version::LatestRelease, String>,
    ) -> Task<Message> {
        self.version_check_in_progress = false;
        if let Ok(release) = result
            && version::is_newer(&release.version, env!("CARGO_PKG_VERSION"))
        {
            self.latest_release = Some(release);
        }
        Task::none()
    }

    fn handle_check_for_app_update(&mut self) -> Task<Message> {
        self.version_check_in_progress = true;
        Task::perform(
            version::check_latest_release(true),
            Message::VersionCheckCompleted,
        )
    }

    fn handle_open_release_page(&mut self) -> Task<Message> {
        if let Some(release) = &self.latest_release {
            let url = release.html_url.clone();
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let _ = std::process::Command::new("cmd")
                    .args(["/c", "start", &url])
                    .creation_flags(install::CREATE_NO_WINDOW)
                    .spawn();
            }
        }
        Task::none()
    }

    // ── Keyboard shortcuts ───────────────────────────────────────

    fn handle_key_confirm(&mut self) -> Task<Message> {
        match self.screen {
            Screen::PackageSelect if !self.selected.is_empty() => {
                self.screen = Screen::Review;
                Task::none()
            }
            Screen::Review => self.handle_start_install(),
            Screen::Installing if self.install.done => self.handle_finish_and_reset(),
            Screen::UpdateScanning if self.update_scan.done => self.handle_go_back(),
            Screen::UpdateSelect if !self.update_scan.selected.is_empty() => {
                self.handle_start_upgrade()
            }
            Screen::Updating if self.upgrade.done => self.handle_finish_update_and_reset(),
            Screen::UninstallSelect if !self.uninstall_selected.is_empty() => {
                self.handle_go_to_uninstall_review()
            }
            Screen::UninstallReview => self.handle_start_uninstall(),
            Screen::Uninstalling if self.uninstall.done => self.handle_finish_uninstall_and_reset(),
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
            _ => Task::none(),
        }
    }

    fn handle_key_escape(&mut self) -> Task<Message> {
        match self.screen {
            Screen::PackageSelect | Screen::Review | Screen::UpdateSelect | Screen::Settings => {
                self.handle_go_back()
            }
            Screen::UninstallSelect | Screen::UninstallReview => self.handle_go_back(),
            Screen::WingetSearch => self.handle_go_back(),
            Screen::WingetSearchInstalling if !self.winget_search_install.done => {
                self.handle_cancel_winget_search_install()
            }
            Screen::WingetSearchInstalling if self.winget_search_install.done => {
                self.handle_finish_winget_search_install()
            }
            Screen::Installing if !self.install.done => self.handle_cancel_install(),
            Screen::UpdateScanning if !self.update_scan.done => self.handle_cancel_update_scan(),
            Screen::Updating if !self.upgrade.done => self.handle_cancel_upgrade(),
            Screen::Uninstalling if !self.uninstall.done => self.handle_cancel_uninstall(),
            _ => Task::none(),
        }
    }

    fn handle_focus_search(&self) -> Task<Message> {
        match self.screen {
            Screen::PackageSelect | Screen::UpdateSelect | Screen::UninstallSelect | Screen::WingetSearch => {
                widget::operation::focus(widget::Id::new(SEARCH_INPUT_ID))
            }
            _ => Task::none(),
        }
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
            Screen::UninstallSelect => self.view_uninstall_select(),
            Screen::UninstallReview => self.view_uninstall_review(),
            Screen::Uninstalling => self.view_uninstalling(),
            Screen::WingetSearch => self.view_winget_search(),
            Screen::WingetSearchInstalling => self.view_winget_search_installing(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let keyboard_sub = keyboard::listen().map(|event| match event {
            keyboard::Event::KeyPressed { key, modifiers, .. } => match key.as_ref() {
                keyboard::Key::Named(keyboard::key::Named::Enter) if modifiers.is_empty() => {
                    Message::KeyConfirm
                }
                keyboard::Key::Named(keyboard::key::Named::Escape) if modifiers.is_empty() => {
                    Message::KeyEscape
                }
                keyboard::Key::Character("k") if modifiers.command() => Message::FocusSearch,
                keyboard::Key::Character("a") if modifiers.command() => Message::SelectAll,
                _ => Message::KeyIgnored,
            },
            _ => Message::KeyIgnored,
        });

        let spinner_active = !self.installed_scan_done
            || matches!(self.screen, Screen::Installing if !self.install.done)
            || matches!(self.screen, Screen::UpdateScanning if !self.update_scan.done)
            || matches!(self.screen, Screen::Updating if !self.upgrade.done)
            || matches!(self.screen, Screen::Uninstalling if !self.uninstall.done)
            || matches!(self.screen, Screen::WingetSearch if self.winget_search_scanning)
            || matches!(self.screen, Screen::WingetSearchInstalling if !self.winget_search_install.done);

        if spinner_active {
            Subscription::batch([
                keyboard_sub,
                time::every(Duration::from_millis(80)).map(|_| Message::SpinnerTick),
            ])
        } else {
            keyboard_sub
        }
    }

    fn theme(&self) -> Theme {
        theme::default()
    }
}
