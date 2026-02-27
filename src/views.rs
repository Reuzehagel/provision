use iced::widget::{
    button, checkbox, column, container, progress_bar, row, scrollable, text, text_input,
};
use iced::{Element, Length, Theme, padding};

use crate::catalog::{self, Package};
use crate::install::PackageStatus;
use crate::profile::Profile;
use crate::upgrade::UpgradeablePackage;
use lucide_icons::Icon;

use crate::styles::{
    LUCIDE_FONT, MUTED, MUTED_FG, STATUS_AMBER, STATUS_BLUE, STATUS_GREEN, STATUS_RED,
    TERMINAL_TEXT, cancel_button_style, card_style, continue_button_style, divider_style,
    ghost_button_style, ghost_icon_button_style, icon_box_style, installed_badge_style,
    package_checkbox_style, terminal_box_style, update_card_style, warning_badge_style,
};
use crate::{App, Message, ProgressState};

impl App {
    pub(crate) fn view_profile_select(&self) -> Element<'_, Message> {
        // Logo icon box
        let logo_icon = text(char::from(Icon::Package))
            .size(20)
            .font(LUCIDE_FONT)
            .color(STATUS_BLUE);
        let logo_box = container(logo_icon)
            .style(icon_box_style)
            .padding(12)
            .center_x(44)
            .center_y(44);

        let title = text("Provision").size(24);
        let subtitle = text("Select a profile to get started")
            .size(14)
            .color(MUTED_FG);

        let heading_cluster = column![logo_box, title, subtitle]
            .spacing(8)
            .align_x(iced::Alignment::Center);

        // Profile cards — top row: Personal + Work; bottom row: Manual (full width)
        let [a, b, c] = Profile::ALL.map(|p| profile_card(p, self.selected_profile));

        let top_row = row![a, b].spacing(10).width(Length::Fill);
        let bottom_row = row![c].spacing(10).width(Length::Fill);

        let grid = column![top_row, bottom_row].spacing(10).width(Length::Fill);

        // Divider
        let divider = container(iced::widget::Space::new().height(1))
            .style(divider_style)
            .width(Length::Fill)
            .height(1);

        // Update row — subtle card
        let update_icon = text(char::from(Icon::RefreshCw))
            .size(15)
            .font(LUCIDE_FONT)
            .color(MUTED);
        let update_text = text("Check for updates").size(14).color(MUTED_FG);
        let chevron = text(char::from(Icon::ChevronRight))
            .size(14)
            .font(LUCIDE_FONT)
            .color(MUTED);

        let update_content = row![
            update_icon,
            update_text,
            iced::widget::Space::new().width(Length::Fill),
            chevron,
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .padding([14, 16])
        .width(Length::Fill);

        let update_card = button(update_content)
            .on_press(Message::StartUpdateScan)
            .width(Length::Fill)
            .style(update_card_style);

        // Scan status
        let scan_status: Element<'_, Message> = if self.installed_scan_done {
            let count = self.installed.len();
            row![
                text(char::from(Icon::Check))
                    .size(12)
                    .font(LUCIDE_FONT)
                    .color(MUTED),
                text(format!("{count} packages detected"))
                    .size(12)
                    .color(MUTED),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .into()
        } else {
            row![
                text(char::from(Icon::Loader))
                    .size(12)
                    .font(LUCIDE_FONT)
                    .color(MUTED),
                text("Scanning installed packages...").size(12).color(MUTED),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .into()
        };

        let content = column![heading_cluster, grid, divider, update_card, scan_status]
            .spacing(14)
            .align_x(iced::Alignment::Center)
            .max_width(500);

        container(content)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(32)
            .into()
    }

    pub(crate) fn view_package_select(&self) -> Element<'_, Message> {
        let profile = self.selected_profile.unwrap_or(Profile::Manual);

        // Header: icon-only back + heading + spacer + search
        let back_icon = text(char::from(Icon::ChevronLeft))
            .size(18)
            .font(LUCIDE_FONT);
        let back_btn = button(back_icon)
            .on_press(Message::GoBack)
            .style(ghost_icon_button_style)
            .padding([6, 8]);

        let heading = text(profile.title()).size(18);

        let search_field = text_input("Search...", &self.search)
            .on_input(Message::SearchChanged)
            .padding(8)
            .size(14)
            .width(200);

        let header = row![
            back_btn,
            heading,
            iced::widget::Space::new().width(Length::Fill),
            search_field,
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let search_lower = self.search.to_lowercase();

        let categories = catalog::categories(&self.catalog);
        let mut pkg_list = column![].spacing(18).width(Length::Fill);

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

            // Count selected in this category
            let selected_count = cat_packages
                .iter()
                .filter(|p| self.selected.contains(&p.id))
                .count();
            let total_count = cat_packages.len();

            let cat_label_text = format!(
                "{} \u{2014} {}/{}",
                catalog::category_display_name(cat).to_uppercase(),
                selected_count,
                total_count,
            );
            let cat_label = text(cat_label_text).size(11).color(MUTED_FG);

            // Split packages into left/right columns
            let half = cat_packages.len().div_ceil(2);
            let left_pkgs = &cat_packages[..half];
            let right_pkgs = &cat_packages[half..];

            let mut left_col = column![].spacing(2);
            let mut right_col = column![].spacing(2);

            for pkg in left_pkgs {
                left_col = left_col.push(package_row(pkg, self));
            }
            for pkg in right_pkgs {
                right_col = right_col.push(package_row(pkg, self));
            }

            let two_col = row![
                left_col.width(Length::FillPortion(1)),
                right_col.width(Length::FillPortion(1)),
            ]
            .spacing(32);

            let cat_col = column![cat_label, two_col].spacing(6);

            pkg_list = pkg_list.push(cat_col);
        }

        let scrollable_list = scrollable(pkg_list)
            .height(Length::Fill)
            .width(Length::Fill);

        // Footer
        let count = self.selected.len();
        let installed_selected = self
            .catalog
            .iter()
            .filter(|p| self.selected.contains(&p.id) && self.is_installed(p))
            .count();
        let footer_label = if installed_selected > 0 {
            format!("{count} selected \u{00b7} {installed_selected} installed")
        } else {
            format!("{count} selected")
        };
        let footer_text = text(footer_label).size(13).color(MUTED);

        let import_btn = button(
            row![
                text(char::from(Icon::Upload)).size(14).font(LUCIDE_FONT),
                text("Import").size(13),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::ImportSelection)
        .style(ghost_button_style)
        .padding([6, 12]);

        let mut export_btn = button(
            row![
                text(char::from(Icon::Download)).size(14).font(LUCIDE_FONT),
                text("Export").size(13),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .style(ghost_button_style)
        .padding([6, 12]);
        if count > 0 {
            export_btn = export_btn.on_press(Message::ExportSelection);
        }

        let status_text: Element<'_, Message> = if let Some(ref msg) = self.selection_status {
            let color = if msg.contains("failed") {
                STATUS_RED
            } else {
                STATUS_GREEN
            };
            text(msg).size(12).color(color).into()
        } else {
            iced::widget::Space::new().into()
        };

        let mut continue_btn = button(text("Continue").size(14))
            .style(continue_button_style)
            .padding([8, 20]);
        if count > 0 {
            continue_btn = continue_btn.on_press(Message::GoToReview);
        }

        let footer = row![
            footer_text,
            import_btn,
            export_btn,
            status_text,
            iced::widget::Space::new().width(Length::Fill),
            continue_btn,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let content = column![header, scrollable_list, footer]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }

    pub(crate) fn view_review(&self) -> Element<'_, Message> {
        // Header: icon-only back + "Review"
        let back_icon = text(char::from(Icon::ChevronLeft))
            .size(18)
            .font(LUCIDE_FONT);
        let back_btn = button(back_icon)
            .on_press(Message::GoBack)
            .style(ghost_icon_button_style)
            .padding([6, 8]);

        let heading = text("Review").size(18);

        let header = row![back_btn, heading]
            .spacing(10)
            .align_y(iced::Alignment::Center);

        let queue: Vec<&Package> = self
            .catalog
            .iter()
            .filter(|p| self.selected.contains(&p.id))
            .collect();

        let reinstall_count = queue.iter().filter(|p| self.is_installed(p)).count();
        let subtitle_text = if reinstall_count > 0 {
            format!(
                "{} packages \u{00b7} {} already installed",
                queue.len(),
                reinstall_count,
            )
        } else {
            format!("{} packages", queue.len())
        };
        let subtitle = text(subtitle_text).size(13).color(MUTED);

        let categories = catalog::categories(&self.catalog);
        let mut pkg_list = column![].spacing(14).width(Length::Fill);

        for cat in &categories {
            let cat_pkgs: Vec<&&Package> = queue.iter().filter(|p| p.category == *cat).collect();
            if cat_pkgs.is_empty() {
                continue;
            }

            let cat_label = text(catalog::category_display_name(cat).to_uppercase())
                .size(11)
                .color(MUTED_FG);

            let mut cat_col = column![cat_label].spacing(4);

            for pkg in cat_pkgs {
                let method = match (&pkg.install_command, &pkg.winget_id) {
                    (Some(cmd), _) => cmd.clone(),
                    (_, Some(wid)) => wid.clone(),
                    _ => "unknown".into(),
                };

                let name_text = text(&pkg.name).size(14);

                let mut name_row = row![name_text].spacing(8).align_y(iced::Alignment::Center);
                if self.is_installed(pkg) {
                    let badge_label = text("Already installed").size(10).color(STATUS_AMBER);
                    let badge = container(badge_label)
                        .style(warning_badge_style)
                        .padding([2, 6]);
                    name_row = name_row.push(badge);
                }

                let pkg_row = row![
                    name_row,
                    iced::widget::Space::new().width(Length::Fill),
                    text(method)
                        .size(11)
                        .font(iced::Font::MONOSPACE)
                        .color(MUTED),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .padding([4, 0]);

                cat_col = cat_col.push(pkg_row);

                if let Some(ref post) = pkg.post_install {
                    let post_text = text(format!("\u{21b3} post-install: {post}"))
                        .size(11)
                        .color(MUTED);
                    cat_col = cat_col.push(container(post_text).padding([2, 0]));
                }
            }

            pkg_list = pkg_list.push(cat_col);
        }

        let scrollable_list = scrollable(pkg_list)
            .height(Length::Fill)
            .width(Length::Fill);

        // Footer: Edit ghost button + Install N primary button
        let new_count = queue.len() - reinstall_count;
        let install_label = format!("Install {} packages", new_count.max(queue.len().min(1)));
        let edit_btn = button(text("Edit").size(14))
            .on_press(Message::GoBack)
            .style(ghost_button_style)
            .padding([8, 20]);
        let install_btn = button(text(install_label).size(14))
            .on_press(Message::StartInstall)
            .style(continue_button_style)
            .padding([8, 20]);

        let footer = row![
            iced::widget::Space::new().width(Length::Fill),
            edit_btn,
            install_btn,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let content = column![header, subtitle, scrollable_list, footer]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
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
                text("Scan Failed").size(20)
            } else {
                text("All packages are up to date").size(20)
            }
        } else if self.dry_run {
            text("[DRY RUN] Scanning for updates...").size(20)
        } else {
            text("Scanning for updates...").size(20)
        };

        let subtitle = if let Some(ref err) = scan.error {
            text(err.clone()).size(14).color(STATUS_RED)
        } else if scan.done {
            text("No outdated packages found.").size(14).color(MUTED)
        } else {
            text("Checking installed packages via winget...")
                .size(14)
                .color(MUTED)
        };

        let log_box = terminal_log_box(&scan.log, &scan.live_line)
            .height(Length::Fill)
            .width(Length::Fill);

        // Footer
        let mut cancel_btn = button(text("Cancel").size(14))
            .style(cancel_button_style)
            .padding([8, 20]);
        if !scan.done {
            cancel_btn = cancel_btn.on_press(Message::CancelUpdateScan);
        }

        let mut back_btn = button(text("Done").size(14))
            .style(continue_button_style)
            .padding([8, 20]);
        if scan.done {
            back_btn = back_btn.on_press(Message::GoBack);
        }

        let footer = row![
            cancel_btn,
            iced::widget::Space::new().width(Length::Fill),
            back_btn,
        ]
        .width(Length::Fill);

        let content = column![heading, subtitle, log_box, footer]
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }

    pub(crate) fn view_update_select(&self) -> Element<'_, Message> {
        let scan = &self.update_scan;

        // Header: icon-only back + heading
        let back_icon = text(char::from(Icon::ChevronLeft))
            .size(18)
            .font(LUCIDE_FONT);
        let back_btn = button(back_icon)
            .on_press(Message::GoBack)
            .style(ghost_icon_button_style)
            .padding([6, 8]);

        let heading = text("Updates").size(18);

        let search_field = text_input("Search...", &self.search)
            .on_input(Message::SearchChanged)
            .padding(8)
            .size(14)
            .width(200);

        let header = row![
            back_btn,
            heading,
            iced::widget::Space::new().width(Length::Fill),
            search_field,
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let search_lower = self.search.to_lowercase();

        let filtered_packages: Vec<&UpgradeablePackage> = scan
            .packages
            .iter()
            .filter(|p| {
                search_lower.is_empty()
                    || p.name.to_lowercase().contains(&search_lower)
                    || p.winget_id.to_lowercase().contains(&search_lower)
            })
            .collect();

        let count = scan.selected.len();
        let total = scan.packages.len();
        let shown = filtered_packages.len();
        let subtitle = if shown < total {
            text(format!("{shown} of {total} outdated packages (filtered)"))
                .size(13)
                .color(MUTED)
        } else {
            text(format!("{total} outdated packages found"))
                .size(13)
                .color(MUTED)
        };

        let mut pkg_list = column![].spacing(6).width(Length::Fill);

        for pkg in &filtered_packages {
            let is_checked = scan.selected.contains(&pkg.winget_id);
            let id = pkg.winget_id.clone();

            let cb = checkbox(is_checked)
                .label(&pkg.name)
                .on_toggle(move |_| Message::ToggleUpgradePackage(id.clone()))
                .size(16)
                .text_size(14)
                .style(package_checkbox_style);

            let version_info = text(format!(
                "{} \u{2192} {}  ({})",
                pkg.current_version, pkg.available_version, pkg.winget_id
            ))
            .size(12)
            .color(MUTED);

            let desc = container(version_info).padding(padding::left(26));

            pkg_list = pkg_list.push(cb).push(desc);
        }

        let scrollable_list = scrollable(pkg_list)
            .height(Length::Fill)
            .width(Length::Fill);

        let footer_text = text(format!("{count} of {total} selected"))
            .size(13)
            .color(MUTED);

        let mut upgrade_btn = button(text("Upgrade").size(14))
            .style(continue_button_style)
            .padding([8, 20]);
        if count > 0 {
            upgrade_btn = upgrade_btn.on_press(Message::StartUpgrade);
        }

        let footer = row![
            footer_text,
            iced::widget::Space::new().width(Length::Fill),
            upgrade_btn,
        ]
        .align_y(iced::Alignment::Center);

        let content = column![header, subtitle, scrollable_list, footer]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
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

    // Heading row: "Installing" + "3 of 12" muted
    let heading_row = if state.done {
        let label = match (dry_run, cancelled_count > 0) {
            (true, true) => "Dry Run Cancelled".to_string(),
            (true, false) => "Dry Run Complete".to_string(),
            (false, true) => format!("{} Cancelled", labels.done_label),
            (false, false) => format!("{} Complete", labels.done_label),
        };
        row![text(label).size(20)]
            .spacing(8)
            .align_y(iced::Alignment::Center)
    } else {
        let verb_text = if dry_run {
            format!("[DRY RUN] {}", labels.verb)
        } else {
            labels.verb.to_string()
        };
        let count_text = format!("{} of {total}", state.current + 1);
        row![
            text(verb_text).size(20),
            text(count_text).size(14).color(MUTED),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
    };

    let subtitle = if state.done {
        let mut parts = vec![
            format!("{done_count} succeeded"),
            format!("{failed_count} failed"),
        ];
        if cancelled_count > 0 {
            parts.push(format!("{cancelled_count} cancelled"));
        }
        text(parts.join(", ")).size(13).color(MUTED)
    } else if dry_run {
        text(labels.dry_run_warning).size(13).color(STATUS_AMBER)
    } else {
        let name = names.get(state.current).unwrap_or(&"...");
        text(*name).size(13).color(MUTED)
    };

    let completed = (done_count + failed_count + cancelled_count) as f32;
    let progress = progress_bar(0.0..=total as f32, completed);

    let active_label = format!("{}...", labels.verb);
    let mut pkg_list = column![].spacing(2).width(Length::Fill);
    for (i, name) in names.iter().enumerate() {
        let (icon_char, color, label) = match &state.statuses[i] {
            PackageStatus::Pending => (char::from(Icon::Circle), MUTED, "Pending".into()),
            PackageStatus::Installing => {
                (char::from(Icon::Loader), STATUS_BLUE, active_label.clone())
            }
            PackageStatus::Done => (char::from(Icon::CircleCheck), STATUS_GREEN, "Done".into()),
            PackageStatus::Failed(e) => (
                char::from(Icon::CircleX),
                STATUS_RED,
                format!("Failed: {e}"),
            ),
            PackageStatus::Cancelled => {
                (char::from(Icon::CircleX), STATUS_AMBER, "Cancelled".into())
            }
        };

        let icon = text(icon_char).size(14).font(LUCIDE_FONT).color(color);

        let pkg_row = row![
            icon,
            text(*name).size(14),
            iced::widget::Space::new().width(Length::Fill),
            text(label).size(12).color(color),
        ]
        .spacing(8)
        .padding([4, 0])
        .align_y(iced::Alignment::Center);

        pkg_list = pkg_list.push(pkg_row);
    }

    let scrollable_pkgs = scrollable(pkg_list)
        .height(Length::FillPortion(3))
        .width(Length::Fill);

    let log_box = terminal_log_box(&state.log, &state.live_line)
        .height(Length::FillPortion(2))
        .width(Length::Fill);

    let mut cancel_btn = button(text("Cancel").size(14))
        .style(cancel_button_style)
        .padding([8, 20]);
    if !state.done {
        cancel_btn = cancel_btn.on_press(cancel_msg);
    }

    let mut done_btn = button(text("Done").size(14))
        .style(continue_button_style)
        .padding([8, 20]);
    if state.done {
        done_btn = done_btn.on_press(done_msg);
    }

    let footer = row![
        cancel_btn,
        iced::widget::Space::new().width(Length::Fill),
        done_btn,
    ]
    .width(Length::Fill);

    let content = column![
        heading_row,
        subtitle,
        progress,
        scrollable_pkgs,
        log_box,
        footer,
    ]
    .spacing(12)
    .width(Length::Fill)
    .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(28)
        .into()
}

/// Single package row for the 2-column grid in package select.
fn package_row<'a>(pkg: &'a Package, app: &'a App) -> Element<'a, Message> {
    let is_checked = app.selected.contains(&pkg.id);
    let id = pkg.id.clone();
    let installed = app.is_installed(pkg);

    let cb = checkbox(is_checked)
        .label(&pkg.name)
        .on_toggle(move |_| Message::TogglePackage(id.clone()))
        .size(16)
        .text_size(14)
        .style(package_checkbox_style);

    let pkg_row_content: Element<'_, Message> = if installed {
        let badge_label = text("Installed").size(10).color(STATUS_GREEN);
        let badge = container(badge_label)
            .style(installed_badge_style)
            .padding([1, 6]);
        row![cb, badge]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
    } else {
        cb.into()
    };

    container(pkg_row_content).padding([4, 0]).into()
}

fn profile_card(profile: Profile, selected: Option<Profile>) -> Element<'static, Message> {
    let is_selected = selected == Some(profile);

    // Icon in a small bordered box
    let icon = text(profile.icon())
        .size(16)
        .font(LUCIDE_FONT)
        .color(MUTED_FG);
    let icon_box = container(icon)
        .style(icon_box_style)
        .padding(9)
        .center_x(36)
        .center_y(36);

    // Text column
    let title = text(profile.title()).size(14);
    let desc = text(profile.description()).size(12).color(MUTED_FG);
    let text_col = column![title, desc].spacing(2);

    // Horizontal layout: icon box + text
    let card_content = row![icon_box, text_col]
        .spacing(14)
        .align_y(iced::Alignment::Start)
        .padding(16)
        .width(Length::Fill);

    button(card_content)
        .on_press(Message::ProfileSelected(profile))
        .width(Length::Fill)
        .style(move |theme: &Theme, status| card_style(theme, status, is_selected))
        .into()
}
