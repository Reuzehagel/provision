mod catalog;
mod install;
mod profile;
mod theme;

use std::collections::HashSet;

use iced::widget::{
    button, checkbox, column, container, progress_bar, row, scrollable, text, text_input,
};
use iced::{Border, Color, Element, Length, Shadow, Size, Task, Theme, padding, task};

use catalog::Package;
use install::PackageStatus;
use profile::Profile;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Provision")
        .theme(App::theme)
        .window_size(Size::new(900.0, 600.0))
        .font(iced_fonts::LUCIDE_FONT_BYTES)
        .run()
}

struct App {
    selected_profile: Option<Profile>,
    screen: Screen,
    catalog: Vec<Package>,
    selected: HashSet<String>,
    search: String,
    // Install state
    install_queue: Vec<Package>,
    install_statuses: Vec<PackageStatus>,
    install_current: usize,
    install_log: Vec<String>,
    install_live_line: String,
    install_done: bool,
    _install_handle: Option<task::Handle>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                selected_profile: None,
                screen: Screen::default(),
                catalog: catalog::load_catalog(),
                selected: HashSet::new(),
                search: String::new(),
                install_queue: Vec::new(),
                install_statuses: Vec::new(),
                install_current: 0,
                install_log: Vec::new(),
                install_live_line: String::new(),
                install_done: false,
                _install_handle: None,
            },
            Task::none(),
        )
    }
}

#[derive(Debug, Default)]
enum Screen {
    #[default]
    ProfileSelect,
    PackageSelect,
    Review,
    Installing,
}

#[derive(Debug, Clone)]
enum Message {
    ProfileSelected(Profile),
    GoBack,
    TogglePackage(String),
    SearchChanged(String),
    GoToReview,
    StartInstall,
    InstallProgress(install::InstallProgress),
    FinishAndReset,
}

const MUTED: Color = Color::from_rgb(0.55, 0.55, 0.58);
const TERMINAL_TEXT: Color = Color::from_rgb(0.7, 0.7, 0.7);
const STATUS_BLUE: Color = Color::from_rgb(0.3, 0.6, 1.0);
const STATUS_GREEN: Color = Color::from_rgb(0.3, 0.8, 0.4);
const STATUS_RED: Color = Color::from_rgb(0.9, 0.3, 0.3);

// Lucide icon codepoints
const ICON_CIRCLE: char = '\u{e098}';
const ICON_LOADER: char = '\u{e114}';
const ICON_CIRCLE_CHECK: char = '\u{e09a}';
const ICON_CIRCLE_X: char = '\u{e0a2}';

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ProfileSelected(profile) => {
                self.selected_profile = Some(profile);
                self.selected = catalog::default_selection(&self.catalog, profile);
                self.search.clear();
                self.screen = Screen::PackageSelect;
            }
            Message::GoBack => match self.screen {
                Screen::Review => {
                    self.screen = Screen::PackageSelect;
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

                let statuses = vec![PackageStatus::Pending; queue.len()];

                self.install_queue = queue.clone();
                self.install_statuses = statuses;
                self.install_current = 0;
                self.install_log.clear();
                self.install_live_line.clear();
                self.install_done = false;
                self.screen = Screen::Installing;

                let (task, handle) =
                    Task::run(install::install_all(queue), Message::InstallProgress).abortable();

                self._install_handle = Some(handle.abort_on_drop());

                return task;
            }
            Message::InstallProgress(event) => match event {
                install::InstallProgress::Started { index } => {
                    if let Some(s) = self.install_statuses.get_mut(index) {
                        *s = PackageStatus::Installing;
                    }
                    self.install_current = index;
                    self.install_live_line.clear();
                    // Add a separator line for the new package
                    let name = self
                        .install_queue
                        .get(index)
                        .map(|p| p.name.as_str())
                        .unwrap_or("...");
                    if index > 0 {
                        self.install_log.push(String::new());
                    }
                    self.install_log.push(format!("--- Installing {name} ---"));
                }
                install::InstallProgress::Log { line, .. } => {
                    self.install_log.push(line);
                    self.install_live_line.clear();
                    // Cap at 200 lines
                    if self.install_log.len() > 200 {
                        self.install_log.drain(..self.install_log.len() - 200);
                    }
                }
                install::InstallProgress::Activity { line, .. } => {
                    self.install_live_line = line;
                }
                install::InstallProgress::Succeeded { index } => {
                    if let Some(s) = self.install_statuses.get_mut(index) {
                        *s = PackageStatus::Done;
                    }
                    self.install_live_line.clear();
                }
                install::InstallProgress::Failed { index, error } => {
                    if let Some(s) = self.install_statuses.get_mut(index) {
                        *s = PackageStatus::Failed(error);
                    }
                    self.install_live_line.clear();
                }
                install::InstallProgress::Completed => {
                    self.install_done = true;
                    self._install_handle = None;
                    self.install_live_line.clear();
                }
            },
            Message::FinishAndReset => {
                self.selected_profile = None;
                self.selected.clear();
                self.search.clear();
                self.install_queue.clear();
                self.install_statuses.clear();
                self.install_log.clear();
                self.install_live_line.clear();
                self.install_done = false;
                self._install_handle = None;
                self.screen = Screen::ProfileSelect;
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
        }
    }

    fn theme(&self) -> Theme {
        theme::default()
    }

    fn view_profile_select(&self) -> Element<'_, Message> {
        let title = text("Provision").size(40);
        let subtitle = text("Choose a profile to get started")
            .size(16)
            .color(MUTED);

        let [a, b, c, d] = Profile::ALL.map(|p| profile_card(p, self.selected_profile));

        let top_row = row![a, b].spacing(16);
        let bottom_row = row![c, d].spacing(16);

        let grid = column![top_row, bottom_row].spacing(16);

        let content = column![title, subtitle, grid]
            .spacing(24)
            .align_x(iced::Alignment::Center);

        container(content)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(40)
            .into()
    }

    fn view_package_select(&self) -> Element<'_, Message> {
        let profile = self.selected_profile.unwrap_or(Profile::Manual);
        let header = screen_header(format!("{} \u{2014} Package Selection", profile.title()));

        let search_field = text_input("Search packages...", &self.search)
            .on_input(Message::SearchChanged)
            .padding(10)
            .size(16);

        let search_lower = self.search.to_lowercase();

        let categories = catalog::categories(&self.catalog);
        let mut pkg_list = column![].spacing(24).width(Length::Fill);

        for cat in &categories {
            let cat_packages: Vec<&Package> = self
                .catalog
                .iter()
                .filter(|p| {
                    p.category == *cat
                        && (search_lower.is_empty()
                            || p.name.to_lowercase().contains(&search_lower)
                            || p.description.to_lowercase().contains(&search_lower))
                })
                .collect();

            if cat_packages.is_empty() {
                continue;
            }

            let cat_label = text(catalog::category_display_name(cat).to_uppercase())
                .size(13)
                .color(MUTED);

            let mut cat_col = column![cat_label].spacing(8);

            for pkg in cat_packages {
                let is_checked = self.selected.contains(&pkg.id);
                let id = pkg.id.clone();

                let cb = checkbox(is_checked)
                    .label(&pkg.name)
                    .on_toggle(move |_| Message::TogglePackage(id.clone()))
                    .size(18)
                    .text_size(15);

                let desc = container(text(&pkg.description).size(13).color(MUTED))
                    .padding(padding::left(30));

                cat_col = cat_col.push(cb).push(desc);
            }

            pkg_list = pkg_list.push(cat_col);
        }

        let scrollable_list = scrollable(pkg_list)
            .height(Length::Fill)
            .width(Length::Fill);

        let count = self.selected.len();
        let footer_text = text(format!("{count} packages selected"))
            .size(14)
            .color(MUTED);

        let mut continue_btn = button(text("Continue").size(15))
            .style(continue_button_style)
            .padding([10, 28]);
        if count > 0 {
            continue_btn = continue_btn.on_press(Message::GoToReview);
        }

        let footer = row![
            footer_text,
            iced::widget::Space::new().width(Length::Fill),
            continue_btn
        ]
        .align_y(iced::Alignment::Center);

        let content = column![header, search_field, scrollable_list, footer]
            .spacing(16)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(40)
            .into()
    }

    fn view_review(&self) -> Element<'_, Message> {
        let profile = self.selected_profile.unwrap_or(Profile::Manual);
        let header = screen_header(format!("{} \u{2014} Review", profile.title()));

        let queue: Vec<&Package> = self
            .catalog
            .iter()
            .filter(|p| self.selected.contains(&p.id))
            .collect();

        let subtitle = text(format!("{} packages will be installed", queue.len()))
            .size(15)
            .color(MUTED);

        let categories = catalog::categories(&self.catalog);
        let mut pkg_list = column![].spacing(20).width(Length::Fill);

        for cat in &categories {
            let cat_pkgs: Vec<&&Package> = queue.iter().filter(|p| p.category == *cat).collect();
            if cat_pkgs.is_empty() {
                continue;
            }

            let cat_label = text(catalog::category_display_name(cat).to_uppercase())
                .size(13)
                .color(MUTED);

            let mut cat_col = column![cat_label].spacing(6);

            for pkg in cat_pkgs {
                let method = match (&pkg.install_command, &pkg.winget_id) {
                    (Some(cmd), _) => cmd.clone(),
                    (_, Some(wid)) => format!("winget: {wid}"),
                    _ => "unknown".into(),
                };

                let pkg_row = row![
                    text(&pkg.name).size(15),
                    iced::widget::Space::new().width(Length::Fill),
                    text(method).size(13).color(MUTED),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center);

                cat_col = cat_col.push(pkg_row);

                if let Some(ref post) = pkg.post_install {
                    let post_text = container(
                        text(format!("+ post-install: {post}"))
                            .size(12)
                            .color(MUTED),
                    )
                    .padding(padding::left(16));
                    cat_col = cat_col.push(post_text);
                }
            }

            pkg_list = pkg_list.push(cat_col);
        }

        let scrollable_list = scrollable(pkg_list)
            .height(Length::Fill)
            .width(Length::Fill);

        let install_btn = button(text("Install").size(15))
            .on_press(Message::StartInstall)
            .style(continue_button_style)
            .padding([10, 28]);

        let footer = row![iced::widget::Space::new().width(Length::Fill), install_btn]
            .align_y(iced::Alignment::Center);

        let content = column![header, subtitle, scrollable_list, footer]
            .spacing(16)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(40)
            .into()
    }

    fn view_installing(&self) -> Element<'_, Message> {
        let total = self.install_queue.len();
        let done_count = self
            .install_statuses
            .iter()
            .filter(|s| matches!(s, PackageStatus::Done))
            .count();
        let failed_count = self
            .install_statuses
            .iter()
            .filter(|s| matches!(s, PackageStatus::Failed(_)))
            .count();

        let heading = if self.install_done {
            text("Installation Complete").size(28)
        } else {
            text(format!(
                "Installing {} of {total}...",
                self.install_current + 1
            ))
            .size(28)
        };

        let subtitle = if self.install_done {
            text(format!("{done_count} succeeded, {failed_count} failed"))
                .size(15)
                .color(MUTED)
        } else {
            let name = self
                .install_queue
                .get(self.install_current)
                .map(|p| p.name.as_str())
                .unwrap_or("...");
            text(format!("Currently installing: {name}"))
                .size(15)
                .color(MUTED)
        };

        // Progress bar
        let completed = (done_count + failed_count) as f32;
        let progress = progress_bar(0.0..=total as f32, completed);

        // Package list with status icons
        let mut pkg_list = column![].spacing(4).width(Length::Fill);
        for (i, pkg) in self.install_queue.iter().enumerate() {
            let (icon_char, color, label) = match &self.install_statuses[i] {
                PackageStatus::Pending => (ICON_CIRCLE, MUTED, "Pending".into()),
                PackageStatus::Installing => (ICON_LOADER, STATUS_BLUE, "Installing...".into()),
                PackageStatus::Done => (ICON_CIRCLE_CHECK, STATUS_GREEN, "Done".into()),
                PackageStatus::Failed(e) => (ICON_CIRCLE_X, STATUS_RED, format!("Failed: {e}")),
            };

            let icon = text(icon_char)
                .size(16)
                .font(iced_fonts::LUCIDE_FONT)
                .color(color);

            let pkg_row = row![
                icon,
                text(&pkg.name).size(14),
                iced::widget::Space::new().width(Length::Fill),
                text(label).size(12).color(color),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);

            pkg_list = pkg_list.push(pkg_row);
        }

        let scrollable_pkgs = scrollable(pkg_list)
            .height(Length::FillPortion(3))
            .width(Length::Fill);

        // Terminal log box -- finalized lines + live activity line
        let mut terminal_text = self.install_log.join("\n");
        if !self.install_live_line.is_empty() {
            if !terminal_text.is_empty() {
                terminal_text.push('\n');
            }
            terminal_text.push_str(&self.install_live_line);
        }

        let terminal_content = column![
            text(terminal_text)
                .size(12)
                .font(iced::Font::MONOSPACE)
                .color(TERMINAL_TEXT)
        ]
        .width(Length::Fill)
        .padding(12);

        let log_box = container(
            scrollable(terminal_content)
                .anchor_bottom()
                .height(Length::Fill)
                .width(Length::Fill),
        )
        .style(terminal_box_style)
        .height(Length::FillPortion(2))
        .width(Length::Fill);

        // Footer — always rendered to avoid layout shift
        let mut done_btn = button(text("Done").size(15))
            .style(continue_button_style)
            .padding([10, 28]);
        if self.install_done {
            done_btn = done_btn.on_press(Message::FinishAndReset);
        }
        let footer =
            row![iced::widget::Space::new().width(Length::Fill), done_btn].width(Length::Fill);

        let content = column![
            heading,
            subtitle,
            progress,
            scrollable_pkgs,
            log_box,
            footer
        ]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(40)
            .into()
    }
}

/// Back button + heading row, reused across PackageSelect, Review, and similar screens.
fn screen_header(label: String) -> Element<'static, Message> {
    let back = button(text("< Back").size(14))
        .on_press(Message::GoBack)
        .style(back_button_style)
        .padding([6, 16]);

    let heading = text(label).size(28);

    row![back, heading]
        .spacing(16)
        .align_y(iced::Alignment::Center)
        .into()
}

fn profile_card(profile: Profile, selected: Option<Profile>) -> Element<'static, Message> {
    let is_selected = selected == Some(profile);

    let icon = text(profile.icon()).size(32).font(iced_fonts::LUCIDE_FONT);
    let title = text(profile.title()).size(20);
    let desc = text(profile.description()).size(14);

    let card_content = column![icon, title, desc]
        .spacing(8)
        .padding(24)
        .width(Length::Fill);

    button(card_content)
        .on_press(Message::ProfileSelected(profile))
        .width(340)
        .style(move |theme: &Theme, status| card_style(theme, status, is_selected))
        .into()
}

fn back_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let bg = match status {
        button::Status::Hovered => palette.background.strong.color,
        _ => palette.background.weak.color,
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: palette.background.base.text,
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

fn continue_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = Color::from_rgb(0.24, 0.50, 0.96);
    let hover = Color::from_rgb(0.30, 0.56, 1.0);
    let disabled = Color::from_rgb(0.2, 0.2, 0.24);

    let (bg, text_color) = match status {
        button::Status::Hovered => (hover, Color::WHITE),
        button::Status::Pressed => (base, Color::WHITE),
        button::Status::Disabled => (disabled, MUTED),
        button::Status::Active => (base, Color::WHITE),
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

fn terminal_box_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.10))),
        border: Border {
            color: Color::from_rgb(0.2, 0.2, 0.22),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn card_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let palette = theme.extended_palette();

    let base_bg = if selected {
        palette.primary.weak.color
    } else {
        Color::from_rgb(0.16, 0.16, 0.18)
    };

    let border_color = if selected {
        palette.primary.base.color
    } else {
        Color::from_rgb(0.28, 0.28, 0.30)
    };

    let background = match status {
        button::Status::Hovered => {
            if selected {
                palette.primary.base.color
            } else {
                Color::from_rgb(0.22, 0.22, 0.24)
            }
        }
        _ => base_bg,
    };

    button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: Color::WHITE,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}
