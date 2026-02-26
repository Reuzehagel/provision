use iced::widget::{
    button, checkbox, column, container, progress_bar, row, scrollable, text, text_input,
};
use iced::{Element, Length, Theme, padding};

use crate::catalog::{self, Package};
use crate::install::PackageStatus;
use crate::profile::Profile;
use crate::styles::{
    ICON_CIRCLE, ICON_CIRCLE_CHECK, ICON_CIRCLE_X, ICON_LOADER, MUTED, STATUS_AMBER, STATUS_BLUE,
    STATUS_GREEN, STATUS_RED, TERMINAL_TEXT, back_button_style, cancel_button_style, card_style,
    continue_button_style, installed_badge_style, terminal_box_style,
};
use crate::{App, Message, ProgressState};

impl App {
    pub(crate) fn view_profile_select(&self) -> Element<'_, Message> {
        let title = text("Provision").size(40);
        let subtitle = text("Choose a profile to get started")
            .size(16)
            .color(MUTED);

        let [a, b, c, d] = Profile::ALL.map(|p| profile_card(p, self.selected_profile));

        let top_row = row![a, b].spacing(16);
        let bottom_row = row![c, d].spacing(16);

        let grid = column![top_row, bottom_row].spacing(16);

        // Update card — full width below the profile grid
        let update_icon = text('\u{e14b}') // refresh-cw
            .size(24)
            .font(iced_fonts::LUCIDE_FONT);
        let update_title = text("Update").size(18);
        let update_desc = text("Check for outdated packages and upgrade them")
            .size(14)
            .color(MUTED);

        let update_content = row![update_icon, column![update_title, update_desc].spacing(4)]
            .spacing(16)
            .align_y(iced::Alignment::Center)
            .padding(20)
            .width(Length::Fill);

        let update_card = button(update_content)
            .on_press(Message::StartUpdateScan)
            .width(Length::Fill)
            .style(move |theme: &Theme, status| card_style(theme, status, false));

        let scan_status: Element<'_, Message> = if self.installed_scan_done {
            let count = self.installed.len();
            text(format!("{count} installed packages detected"))
                .size(13)
                .color(MUTED)
                .into()
        } else {
            row![
                text(ICON_LOADER)
                    .size(14)
                    .font(iced_fonts::LUCIDE_FONT)
                    .color(MUTED),
                text("Scanning installed packages...").size(13).color(MUTED),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center)
            .into()
        };

        let content = column![title, subtitle, scan_status, grid, update_card]
            .spacing(24)
            .align_x(iced::Alignment::Center);

        container(content)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(40)
            .into()
    }

    pub(crate) fn view_package_select(&self) -> Element<'_, Message> {
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

                let installed = self.is_installed(pkg);

                let cb = checkbox(is_checked)
                    .label(&pkg.name)
                    .on_toggle(move |_| Message::TogglePackage(id.clone()))
                    .size(18)
                    .text_size(15);

                let pkg_row: Element<'_, Message> = if installed {
                    let badge_label = text("Installed").size(11).color(STATUS_GREEN);
                    let badge = container(badge_label)
                        .style(installed_badge_style)
                        .padding([2, 8]);
                    row![cb, badge]
                        .spacing(10)
                        .align_y(iced::Alignment::Center)
                        .into()
                } else {
                    cb.into()
                };

                let desc_text = match self.installed_version(pkg) {
                    Some(ver) => format!("{} (v{ver})", pkg.description),
                    None => pkg.description.clone(),
                };

                let desc =
                    container(text(desc_text).size(13).color(MUTED)).padding(padding::left(30));

                cat_col = cat_col.push(pkg_row).push(desc);
            }

            pkg_list = pkg_list.push(cat_col);
        }

        let scrollable_list = scrollable(pkg_list)
            .height(Length::Fill)
            .width(Length::Fill);

        let count = self.selected.len();
        let installed_selected = self
            .catalog
            .iter()
            .filter(|p| self.selected.contains(&p.id) && self.is_installed(p))
            .count();
        let footer_label = if installed_selected > 0 {
            format!("{count} selected ({installed_selected} already installed)")
        } else {
            format!("{count} packages selected")
        };
        let footer_text = text(footer_label).size(14).color(MUTED);

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

    pub(crate) fn view_review(&self) -> Element<'_, Message> {
        let profile = self.selected_profile.unwrap_or(Profile::Manual);
        let header = screen_header(format!("{} \u{2014} Review", profile.title()));

        let queue: Vec<&Package> = self
            .catalog
            .iter()
            .filter(|p| self.selected.contains(&p.id))
            .collect();

        let reinstall_count = queue.iter().filter(|p| self.is_installed(p)).count();
        let subtitle_text = if reinstall_count > 0 {
            format!(
                "{} packages will be installed ({reinstall_count} already installed)",
                queue.len()
            )
        } else {
            format!("{} packages will be installed", queue.len())
        };
        let subtitle = text(subtitle_text).size(15).color(MUTED);

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

                let mut name_row = row![text(&pkg.name).size(15)].spacing(8);
                if self.is_installed(pkg) {
                    name_row =
                        name_row.push(text("(already installed)").size(13).color(STATUS_AMBER));
                }

                let pkg_row = row![
                    name_row,
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
        )
    }

    pub(crate) fn view_update_scanning(&self) -> Element<'_, Message> {
        let scan = &self.update_scan;

        let heading = if scan.done {
            if scan.error.is_some() {
                text("Scan Failed").size(28)
            } else {
                text("All packages are up to date").size(28)
            }
        } else if self.dry_run {
            text("[DRY RUN] Scanning for updates...").size(28)
        } else {
            text("Scanning for updates...").size(28)
        };

        let subtitle = if let Some(ref err) = scan.error {
            text(err.clone()).size(15).color(STATUS_RED)
        } else if scan.done {
            text("No outdated packages found.").size(15).color(MUTED)
        } else {
            text("Checking installed packages via winget...")
                .size(15)
                .color(MUTED)
        };

        let log_box = terminal_log_box(&scan.log, &scan.live_line)
            .height(Length::Fill)
            .width(Length::Fill);

        // Footer
        let mut back_btn = button(text("Back").size(15))
            .style(back_button_style)
            .padding([10, 28]);
        if scan.done {
            back_btn = back_btn.on_press(Message::GoBack);
        }

        let mut cancel_btn = button(text("Cancel").size(15))
            .style(cancel_button_style)
            .padding([10, 28]);
        if !scan.done {
            cancel_btn = cancel_btn.on_press(Message::CancelUpdateScan);
        }

        let footer = row![
            cancel_btn,
            iced::widget::Space::new().width(Length::Fill),
            back_btn
        ]
        .width(Length::Fill);

        let content = column![heading, subtitle, log_box, footer]
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(40)
            .into()
    }

    pub(crate) fn view_update_select(&self) -> Element<'_, Message> {
        let scan = &self.update_scan;
        let header = screen_header("Update \u{2014} Select Packages".into());

        let count = scan.selected.len();
        let total = scan.packages.len();
        let subtitle = text(format!("{total} outdated packages found"))
            .size(15)
            .color(MUTED);

        let mut pkg_list = column![].spacing(8).width(Length::Fill);

        for pkg in &scan.packages {
            let is_checked = scan.selected.contains(&pkg.winget_id);
            let id = pkg.winget_id.clone();

            let cb = checkbox(is_checked)
                .label(&pkg.name)
                .on_toggle(move |_| Message::ToggleUpgradePackage(id.clone()))
                .size(18)
                .text_size(15);

            let version_info = text(format!(
                "{} \u{2192} {}  ({})",
                pkg.current_version, pkg.available_version, pkg.winget_id
            ))
            .size(13)
            .color(MUTED);

            let desc = container(version_info).padding(padding::left(30));

            pkg_list = pkg_list.push(cb).push(desc);
        }

        let scrollable_list = scrollable(pkg_list)
            .height(Length::Fill)
            .width(Length::Fill);

        let footer_text = text(format!("{count} of {total} selected"))
            .size(14)
            .color(MUTED);

        let mut upgrade_btn = button(text("Upgrade").size(15))
            .style(continue_button_style)
            .padding([10, 28]);
        if count > 0 {
            upgrade_btn = upgrade_btn.on_press(Message::StartUpgrade);
        }

        let footer = row![
            footer_text,
            iced::widget::Space::new().width(Length::Fill),
            upgrade_btn
        ]
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

    pub(crate) fn view_updating(&self) -> Element<'_, Message> {
        view_progress_screen(
            &self.upgrade,
            &ProgressLabels {
                verb: "Upgrading",
                done_label: "Upgrade",
                dry_run_warning: "No packages will actually be upgraded",
            },
            self.upgrade_queue.iter().map(|p| p.name.as_str()),
            self.dry_run,
            Message::CancelUpgrade,
            Message::FinishUpdateAndReset,
        )
    }
}

/// Terminal log box: monospace text in a dark container, auto-scrolled to bottom.
fn terminal_log_box<'a>(log: &[String], live_line: &str) -> iced::widget::Container<'a, Message> {
    let mut terminal_text = log.join("\n");
    if !live_line.is_empty() {
        if !terminal_text.is_empty() {
            terminal_text.push('\n');
        }
        terminal_text.push_str(live_line);
    }

    let terminal_content = column![
        text(terminal_text)
            .size(12)
            .font(iced::Font::MONOSPACE)
            .color(TERMINAL_TEXT)
    ]
    .width(Length::Fill)
    .padding(12);

    container(
        scrollable(terminal_content)
            .anchor_bottom()
            .height(Length::Fill)
            .width(Length::Fill),
    )
    .style(terminal_box_style)
}

/// Labels that vary between the install and upgrade progress screens.
struct ProgressLabels {
    /// Present participle, e.g. "Installing" or "Upgrading"
    verb: &'static str,
    /// Noun for the done heading, e.g. "Installation" or "Upgrade"
    done_label: &'static str,
    /// Dry-run subtitle, e.g. "No packages will actually be installed"
    dry_run_warning: &'static str,
}

/// Shared layout for both the Installing and Updating screens.
fn view_progress_screen<'a>(
    state: &ProgressState,
    labels: &ProgressLabels,
    names: impl Iterator<Item = &'a str>,
    dry_run: bool,
    cancel_msg: Message,
    done_msg: Message,
) -> Element<'a, Message> {
    let names: Vec<&str> = names.collect();
    let total = names.len();
    let (done_count, failed_count, cancelled_count) = state.status_counts();

    let heading = if state.done {
        let label = match (dry_run, cancelled_count > 0) {
            (true, true) => "Dry Run Cancelled".to_string(),
            (true, false) => "Dry Run Complete".to_string(),
            (false, true) => format!("{} Cancelled", labels.done_label),
            (false, false) => format!("{} Complete", labels.done_label),
        };
        text(label).size(28)
    } else if dry_run {
        text(format!(
            "[DRY RUN] {} {} of {total}...",
            labels.verb,
            state.current + 1
        ))
        .size(28)
    } else {
        text(format!(
            "{} {} of {total}...",
            labels.verb,
            state.current + 1
        ))
        .size(28)
    };

    let subtitle = if state.done {
        let mut parts = vec![
            format!("{done_count} succeeded"),
            format!("{failed_count} failed"),
        ];
        if cancelled_count > 0 {
            parts.push(format!("{cancelled_count} cancelled"));
        }
        text(parts.join(", ")).size(15).color(MUTED)
    } else if dry_run {
        text(labels.dry_run_warning).size(15).color(STATUS_AMBER)
    } else {
        let name = names.get(state.current).unwrap_or(&"...");
        text(format!("Currently {}: {name}", labels.verb.to_lowercase()))
            .size(15)
            .color(MUTED)
    };

    let completed = (done_count + failed_count + cancelled_count) as f32;
    let progress = progress_bar(0.0..=total as f32, completed);

    let active_label = format!("{}...", labels.verb);
    let mut pkg_list = column![].spacing(4).width(Length::Fill);
    for (i, name) in names.iter().enumerate() {
        let (icon_char, color, label) = match &state.statuses[i] {
            PackageStatus::Pending => (ICON_CIRCLE, MUTED, "Pending".into()),
            PackageStatus::Installing => (ICON_LOADER, STATUS_BLUE, active_label.clone()),
            PackageStatus::Done => (ICON_CIRCLE_CHECK, STATUS_GREEN, "Done".into()),
            PackageStatus::Failed(e) => (ICON_CIRCLE_X, STATUS_RED, format!("Failed: {e}")),
            PackageStatus::Cancelled => (ICON_CIRCLE_X, STATUS_AMBER, "Cancelled".into()),
        };

        let icon = text(icon_char)
            .size(16)
            .font(iced_fonts::LUCIDE_FONT)
            .color(color);

        let pkg_row = row![
            icon,
            text(*name).size(14),
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

    let log_box = terminal_log_box(&state.log, &state.live_line)
        .height(Length::FillPortion(2))
        .width(Length::Fill);

    let mut cancel_btn = button(text("Cancel").size(15))
        .style(cancel_button_style)
        .padding([10, 28]);
    if !state.done {
        cancel_btn = cancel_btn.on_press(cancel_msg);
    }

    let mut done_btn = button(text("Done").size(15))
        .style(continue_button_style)
        .padding([10, 28]);
    if state.done {
        done_btn = done_btn.on_press(done_msg);
    }

    let footer = row![
        cancel_btn,
        iced::widget::Space::new().width(Length::Fill),
        done_btn
    ]
    .width(Length::Fill);

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
