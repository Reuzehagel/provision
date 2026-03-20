use iced::widget::{
    button, checkbox, column, container, pick_list, progress_bar, row, scrollable, text,
    text_input, toggler,
};
use iced::{Element, Length, Theme, padding};

use crate::catalog::{self, CatalogSource, Package};
use crate::install::PackageStatus;
use crate::profile::Profile;
use crate::settings::{InstallMode, OptionalArchitecture, OptionalScope, SettingsTab};
use crate::upgrade::UpgradeablePackage;
use crate::{SEARCH_INPUT_ID, SPINNER_FRAMES};
use lucide_icons::Icon;

use crate::github::BootstrapStatus;
use crate::styles::{
    BORDER, CARD_BG, LUCIDE_FONT, MUTED, MUTED_FG, REPOS_PURPLE, STATUS_AMBER, STATUS_BLUE,
    STATUS_GREEN, STATUS_RED, TERMINAL_TEXT, TEXT, browser_badge_style, cancel_button_style,
    card_container_style, continue_button_style, danger_button_style, ghost_button_style,
    hero_card_style, hero_profile_button_style, installed_badge_style, package_checkbox_style,
    reboot_badge_style, restart_badge_style, scan_button_style, selected_row_style, tab_style,
    terminal_box_style, tinted_icon_bg, tool_tile_style, update_banner_style, warning_badge_style,
};
use crate::{App, LogBuffer, Message, ProgressState};

impl App {
    pub(crate) fn view_profile_select(&self) -> Element<'_, Message> {
        // ── Section 1: Hero Card ────────────────────────────────
        let si = &self.system_info;

        let logo = text(char::from(Icon::Package))
            .size(16)
            .font(LUCIDE_FONT)
            .color(STATUS_BLUE);
        let title = text("Provision").size(16);
        let brand = row![logo, title]
            .spacing(6)
            .align_y(iced::Alignment::Center);

        let sys_info_text = text(format!(
            "{} · {} · {:.0} GB",
            si.hostname, si.cpu_name, si.ram_gb
        ))
        .size(12)
        .color(MUTED);

        let hero_top = row![
            brand,
            iced::widget::Space::new().width(Length::Fill),
            sys_info_text,
        ]
        .align_y(iced::Alignment::Center);

        // Profile buttons
        let profile_buttons: Vec<Element<'_, Message>> = Profile::ALL
            .iter()
            .map(|&p| {
                let icon = text(p.icon()).size(12).font(LUCIDE_FONT);
                let label = text(p.title()).size(12);
                let content = row![icon, label]
                    .spacing(6)
                    .align_y(iced::Alignment::Center);

                button(container(content).center_x(Length::Fill).padding([8, 4]))
                    .on_press(Message::ProfileSelected(p))
                    .width(Length::Fill)
                    .style(hero_profile_button_style)
                    .into()
            })
            .collect();

        let profile_row = iced::widget::Row::with_children(profile_buttons)
            .spacing(8)
            .width(Length::Fill);

        let hero = container(
            column![hero_top, profile_row]
                .spacing(12)
                .width(Length::Fill),
        )
        .padding(16)
        .width(Length::Fill)
        .style(hero_card_style);

        // ── Section 2: Update Banner ────────────────────────────
        let update_count = self.update_scan.packages.len();
        let scan_done = self.update_scan.done;

        let count_display: Element<'_, Message> = if scan_done {
            if update_count > 0 {
                text(update_count.to_string())
                    .size(28)
                    .color(STATUS_GREEN)
                    .into()
            } else {
                text(char::from(Icon::CircleCheck))
                    .size(24)
                    .font(LUCIDE_FONT)
                    .color(STATUS_GREEN)
                    .into()
            }
        } else {
            text("\u{2014}").size(28).color(MUTED).into()
        };

        let update_label: Element<'_, Message> = if scan_done && update_count == 0 {
            text("All up to date").size(12).color(TEXT).into()
        } else if scan_done {
            text("Updates available").size(12).color(TEXT).into()
        } else {
            text("Scan to check for updates")
                .size(12)
                .color(MUTED_FG)
                .into()
        };

        let installed_count = self.installed_map.len();
        let catalog_count = self.catalog.len();
        let stats_text = if !self.installed_scan_done && installed_count == 0 {
            "Scanning installed...".to_string()
        } else {
            format!("{installed_count} installed · {catalog_count} in catalog")
        };
        let stats_line = text(stats_text).size(12).color(MUTED_FG);

        let update_info = column![update_label, stats_line].spacing(2);

        let scan_btn = button(text("Scan").size(12))
            .on_press(Message::StartUpdateScan)
            .style(scan_button_style)
            .padding([7, 16]);

        let update_banner = container(
            row![
                count_display,
                update_info,
                iced::widget::Space::new().width(Length::Fill),
                scan_btn,
            ]
            .spacing(16)
            .align_y(iced::Alignment::Center)
            .width(Length::Fill),
        )
        .padding(16)
        .width(Length::Fill)
        .style(card_container_style);

        // ── Section 3: Tool Tiles ───────────────────────────────
        let uninstall_msg = if self.installed_scan_done && !self.installed_packages.is_empty() {
            Some(Message::GoToUninstall)
        } else {
            None
        };

        let tile_uninstall = tool_tile(Icon::Trash2, "Uninstall", STATUS_RED, uninstall_msg);
        let tile_search = tool_tile(
            Icon::Search,
            "Search",
            STATUS_AMBER,
            Some(Message::GoToWingetSearch),
        );
        let tile_repos = tool_tile(
            Icon::Github,
            "Repos",
            REPOS_PURPLE,
            Some(Message::GoToGitHubLogin),
        );

        let tiles = row![tile_uninstall, tile_search, tile_repos]
            .spacing(8)
            .width(Length::Fill);

        // ── Footer ──────────────────────────────────────────────
        let catalog_color = if self.catalog_source == CatalogSource::Remote {
            STATUS_GREEN
        } else {
            MUTED
        };
        let catalog_label = match self.catalog_source.label_suffix() {
            Some(suffix) => format!("{catalog_count} packages ({suffix})"),
            None => format!("{catalog_count} packages"),
        };
        let catalog_status = status_indicator(Icon::Package, catalog_label, catalog_color);

        let version_label = text(format!("v{}", env!("CARGO_PKG_VERSION")))
            .size(12)
            .color(MUTED);

        let settings_icon = button(
            text(char::from(Icon::Settings))
                .size(13)
                .font(LUCIDE_FONT)
                .color(MUTED),
        )
        .on_press(Message::OpenSettings)
        .style(ghost_button_style)
        .padding([2, 4]);

        let footer = row![
            catalog_status,
            iced::widget::Space::new().width(Length::Fill),
            version_label,
            settings_icon,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        // ── Assemble ────────────────────────────────────────────
        let mut content = column![hero].spacing(10).max_width(500);

        // Update version banner (between hero and update banner)
        if let Some(release) = &self.latest_release {
            let banner_icon = text(char::from(Icon::CircleArrowUp))
                .size(15)
                .font(LUCIDE_FONT)
                .color(STATUS_AMBER);
            let banner_text = text(format!("v{} available", release.version))
                .size(14)
                .color(TEXT);
            let banner_link = text("View release →").size(13).color(STATUS_AMBER);
            let dismiss_icon = text(char::from(Icon::X))
                .size(14)
                .font(LUCIDE_FONT)
                .color(MUTED_FG);
            let dismiss_btn = button(dismiss_icon)
                .on_press(Message::DismissUpdateBanner)
                .style(ghost_button_style)
                .padding([4, 6]);

            let banner_content = row![
                banner_icon,
                banner_text,
                banner_link,
                iced::widget::Space::new().width(Length::Fill),
                dismiss_btn,
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .padding([12, 16])
            .width(Length::Fill);

            let banner = button(banner_content)
                .on_press(Message::OpenReleasePage)
                .width(Length::Fill)
                .style(update_banner_style);

            content = content.push(banner);
        }

        let content = content.push(update_banner).push(tiles).push(footer);

        container(content)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(32)
            .into()
    }

    pub(crate) fn view_package_select(&self) -> Element<'_, Message> {
        let profile = self.selected_profile.unwrap_or(Profile::Manual);

        let header = search_header(profile.title(), &self.search);

        let search_lower = self.search_lower.as_str();
        let categories = &self.categories;
        let mut pkg_list = column![].spacing(18).width(Length::Fill);

        for cat in categories {
            let cat_packages: Vec<&Package> = self
                .catalog
                .iter()
                .filter(|p| {
                    p.category == *cat
                        && (search_lower.is_empty()
                            || p.name_lower.contains(search_lower)
                            || p.desc_lower.contains(search_lower))
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
            let toggle_icon = if selected_count == total_count {
                Icon::SquareCheckBig
            } else if selected_count > 0 {
                Icon::SquareMinus
            } else {
                Icon::Square
            };

            let cat_label = button(
                row![
                    text(char::from(toggle_icon))
                        .size(12)
                        .font(LUCIDE_FONT)
                        .color(MUTED_FG),
                    text(cat_label_text).size(12).color(MUTED_FG),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::ToggleCategory(cat.clone()))
            .style(ghost_button_style)
            .padding([2, 6]);

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

        let scrollable_list = scrollable(pkg_list.padding(padding::right(20)))
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
        let header = back_header("Review");

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

        let categories = &self.categories;
        let mut pkg_list = column![]
            .spacing(14)
            .width(Length::Fill)
            .padding(padding::right(20));

        for cat in categories {
            let cat_pkgs: Vec<&&Package> = queue.iter().filter(|p| p.category == *cat).collect();
            if cat_pkgs.is_empty() {
                continue;
            }

            let cat_label = text(catalog::category_display_name(cat).to_uppercase())
                .size(12)
                .color(MUTED_FG);

            let mut cat_col = column![cat_label].spacing(4);

            for pkg in cat_pkgs {
                let method_widget: Element<'_, Message> = match pkg.setup_kind() {
                    catalog::SetupKind::BrowserDownload => row![
                        text(char::from(Icon::ExternalLink))
                            .size(12)
                            .font(LUCIDE_FONT)
                            .color(STATUS_BLUE),
                        text("opens browser").size(12).color(STATUS_BLUE),
                    ]
                    .spacing(4)
                    .align_y(iced::Alignment::Center)
                    .into(),
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

                let name_text = text(&pkg.name).size(14);

                let mut name_row = row![name_text].spacing(8).align_y(iced::Alignment::Center);
                if self.is_installed(pkg) {
                    let badge_label = text("Already installed").size(12).color(STATUS_AMBER);
                    let badge = container(badge_label)
                        .style(warning_badge_style)
                        .padding([2, 6]);
                    name_row = name_row.push(badge);
                }

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

                let pkg_row = row![
                    name_row,
                    iced::widget::Space::new().width(Length::Fill),
                    method_widget,
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .padding([4, 0]);

                cat_col = cat_col.push(pkg_row);

                if let Some(ref post) = pkg.post_install {
                    let post_text = text(format!("\u{21b3} post-install: {post}"))
                        .size(12)
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
        let install_label = if new_count == 0 {
            format!("Reinstall {} packages", reinstall_count)
        } else if reinstall_count == 0 {
            format!("Install {} packages", new_count)
        } else {
            format!("Install {} + reinstall {}", new_count, reinstall_count)
        };
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

        let mut content = column![header, subtitle, scrollable_list]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

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
                format!("{special_count} packages need a terminal restart \u{2014} installed last.")
            } else {
                format!(
                    "{special_count} packages need manual steps \u{2014} they'll be installed last."
                )
            };

            content = content.push(status_indicator(Icon::TriangleAlert, summary, STATUS_AMBER));
        }

        let content = content.push(footer);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }

    pub(crate) fn view_installing(&self) -> Element<'_, Message> {
        let checklist = if self.install.done {
            self.build_post_install_checklist()
        } else {
            None
        };

        view_progress_screen(
            &self.install,
            ProgressLabels {
                verb: "Installing",
                done_label: "Installation",
                dry_run_warning: "No packages will actually be installed",
                cancel_msg: Message::CancelInstall,
                done_msg: Message::FinishAndReset,
            },
            self.install_queue.iter().map(|p| p.name.as_str()),
            self.dry_run,
            self.spinner_frame,
            checklist,
        )
    }

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

        let mut checklist = column![].spacing(8).width(Length::Fill);

        checklist = checklist.push(text("Post-install steps").size(14).color(TEXT));

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

    pub(crate) fn view_post_install_steps(&self) -> Element<'_, Message> {
        let header = text("Next steps").size(20);
        let subtitle = text("Some packages need manual action to finish setup.")
            .size(13)
            .color(MUTED);

        let mut cards = column![].spacing(8).width(Length::Fill);

        for (pkg, status) in self.install_queue.iter().zip(self.install.statuses.iter()) {
            let step = match (&status, &pkg.post_install) {
                (PackageStatus::Done, Some(msg)) => msg.as_str(),
                _ => continue,
            };

            let icon = text(char::from(Icon::Info))
                .size(16)
                .font(LUCIDE_FONT)
                .color(STATUS_BLUE);

            let card_content = column![
                row![icon, text(&pkg.name).size(14)]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                text(step).size(13).color(MUTED),
            ]
            .spacing(4);

            let card = container(card_content)
                .style(card_container_style)
                .padding(12)
                .width(Length::Fill);

            cards = cards.push(card);
        }

        let done_btn = button(text("Done").size(14))
            .style(continue_button_style)
            .padding([8, 20])
            .on_press(Message::DismissPostInstallSteps);

        let footer =
            row![iced::widget::Space::new().width(Length::Fill), done_btn,].width(Length::Fill);

        let content = column![
            header,
            subtitle,
            scrollable(cards).height(Length::Fill),
            footer
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

    pub(crate) fn view_update_scanning(&self) -> Element<'_, Message> {
        let scan = &self.update_scan;

        if scan.done {
            // Scan finished with no packages (or error) — show result
            let icon: Element<'_, Message> = if scan.error.is_some() {
                text(char::from(Icon::CircleX))
                    .size(32)
                    .font(LUCIDE_FONT)
                    .color(STATUS_RED)
                    .into()
            } else {
                text(char::from(Icon::CircleCheck))
                    .size(32)
                    .font(LUCIDE_FONT)
                    .color(STATUS_GREEN)
                    .into()
            };

            let heading = if scan.error.is_some() {
                text("Scan failed").size(18)
            } else {
                text("All packages are up to date").size(18)
            };

            let subtitle: Element<'_, Message> = if let Some(ref err) = scan.error {
                text(err.clone()).size(13).color(STATUS_RED).into()
            } else {
                text("No outdated packages found.")
                    .size(13)
                    .color(MUTED)
                    .into()
            };

            let back_btn = button(text("Done").size(14))
                .style(continue_button_style)
                .padding([8, 20])
                .on_press(Message::GoBack);

            let center = column![icon, heading, subtitle, back_btn]
                .spacing(12)
                .align_x(iced::Alignment::Center);

            return container(center)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .padding(28)
                .into();
        }

        // Still scanning — show centered spinner
        let spinner = text(SPINNER_FRAMES[self.spinner_frame])
            .size(24)
            .color(STATUS_BLUE);

        let heading = if self.dry_run {
            text("[DRY RUN] Scanning for updates...").size(16)
        } else {
            text("Scanning for updates...").size(16)
        };

        let activity: Element<'_, Message> = if !scan.log.live_line.is_empty() {
            text(&scan.log.live_line).size(12).color(MUTED).into()
        } else {
            text("Checking installed packages via winget...")
                .size(12)
                .color(MUTED)
                .into()
        };

        let cancel_btn = button(text("Cancel").size(14))
            .style(cancel_button_style)
            .padding([8, 20])
            .on_press(Message::CancelUpdateScan);

        let center = column![spinner, heading, activity, cancel_btn]
            .spacing(12)
            .align_x(iced::Alignment::Center);

        container(center)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(28)
            .into()
    }

    pub(crate) fn view_update_select(&self) -> Element<'_, Message> {
        let scan = &self.update_scan;

        let header = search_header("Updates", &self.search);

        let search_lower = self.search_lower.as_str();

        let filtered_packages: Vec<&UpgradeablePackage> = scan
            .packages
            .iter()
            .filter(|p| {
                search_lower.is_empty()
                    || p.name_lower.contains(search_lower)
                    || p.winget_id_lower.contains(search_lower)
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

        let scrollable_list = scrollable(pkg_list.padding(padding::right(20)))
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
            ProgressLabels {
                verb: "Upgrading",
                done_label: "Upgrade",
                dry_run_warning: "No packages will actually be upgraded",
                cancel_msg: Message::CancelUpgrade,
                done_msg: Message::FinishUpdateAndReset,
            },
            self.upgrade_queue.iter().map(|p| p.name.as_str()),
            self.dry_run,
            self.spinner_frame,
            None,
        )
    }

    pub(crate) fn view_settings(&self) -> Element<'_, Message> {
        let header = back_header("Settings");

        let tab_bar = row![
            tab_button("Winget", SettingsTab::Winget, self.settings_tab),
            tab_button("Changelog", SettingsTab::Changelog, self.settings_tab),
        ]
        .spacing(4);

        let tab_content: Element<'_, Message> = match self.settings_tab {
            SettingsTab::Winget => self.view_settings_winget(),
            SettingsTab::Changelog => view_settings_changelog(),
        };

        let content = column![header, tab_bar, tab_content]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }

    fn view_settings_winget(&self) -> Element<'_, Message> {
        let s = &self.settings;

        let subtitle = text("Settings are saved automatically")
            .size(13)
            .color(MUTED);

        // ── Install Behavior section ─────────────────────────────
        let section_behavior = text("INSTALL BEHAVIOR").size(12).color(MUTED_FG);

        let mode_row = setting_row(
            "Install mode",
            "Silent runs without UI, Interactive shows the installer",
            pick_list(
                &InstallMode::ALL[..],
                Some(s.install_mode),
                Message::SetInstallMode,
            )
            .text_size(13)
            .width(160)
            .into(),
        );

        let scope_row = setting_row(
            "Scope",
            "Per-user or system-wide installation",
            pick_list(
                &OptionalScope::ALL[..],
                Some(OptionalScope(s.scope)),
                Message::SetScope,
            )
            .text_size(13)
            .width(160)
            .into(),
        );

        let arch_row = setting_row(
            "Architecture",
            "Force a specific processor architecture",
            pick_list(
                &OptionalArchitecture::ALL[..],
                Some(OptionalArchitecture(s.architecture)),
                Message::SetArchitecture,
            )
            .text_size(13)
            .width(160)
            .into(),
        );

        let location_row = setting_row(
            "Install location",
            "Custom directory path (leave empty for default)",
            text_input("", &s.install_location)
                .on_input(Message::SetInstallLocation)
                .padding(6)
                .size(13)
                .width(260)
                .into(),
        );

        // ── Advanced section ─────────────────────────────────────
        let section_advanced = text("ADVANCED").size(12).color(MUTED_FG);

        let force_row = toggle_row(
            "Force reinstall",
            "Reinstall even if already installed (--force)",
            s.force,
            Message::ToggleForce,
        );

        let interactivity_row = toggle_row(
            "Disable interactivity",
            "Prevent winget from prompting (--disable-interactivity)",
            s.disable_interactivity,
            Message::ToggleDisableInteractivity,
        );

        let unknown_row = toggle_row(
            "Include unknown versions",
            "Show packages with unknown versions in update scans",
            s.include_unknown,
            Message::ToggleIncludeUnknown,
        );

        let hash_row = toggle_row(
            "Skip hash verification",
            "Ignore security hash checks (--ignore-security-hash)",
            s.ignore_security_hash,
            Message::ToggleIgnoreSecurityHash,
        );

        // ── App Updates section ───────────────────────────────────
        let section_updates = text("APP UPDATES").size(12).color(MUTED_FG);

        let update_status_text: Element<'_, Message> = if self.version_check_in_progress {
            text("Checking...").size(13).color(MUTED).into()
        } else if let Some(release) = &self.latest_release {
            row![
                text(format!("New version available: v{}", release.version))
                    .size(13)
                    .color(STATUS_AMBER),
            ]
            .into()
        } else {
            text("You're up to date")
                .size(13)
                .color(STATUS_GREEN)
                .into()
        };

        let mut check_btn = button(text("Check now").size(13))
            .style(ghost_button_style)
            .padding([6, 12]);
        if !self.version_check_in_progress {
            check_btn = check_btn.on_press(Message::CheckForAppUpdate);
        }

        let mut update_row = row![
            update_status_text,
            iced::widget::Space::new().width(Length::Fill),
            check_btn
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill);

        if !self.version_check_in_progress && self.latest_release.is_some() {
            let view_btn = button(text("View release →").size(13).color(STATUS_AMBER))
                .on_press(Message::OpenReleasePage)
                .style(ghost_button_style)
                .padding([6, 12]);
            update_row = update_row.push(view_btn);
        }

        let settings_list = column![
            subtitle,
            section_behavior,
            mode_row,
            scope_row,
            arch_row,
            location_row,
            section_advanced,
            force_row,
            interactivity_row,
            unknown_row,
            hash_row,
            section_updates,
            update_row,
        ]
        .spacing(12)
        .padding(padding::right(20))
        .width(Length::Fill);

        scrollable(settings_list)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    pub(crate) fn view_uninstall_select(&self) -> Element<'_, Message> {
        let header = search_header("Uninstall packages", &self.search);

        let search_lower = self.search_lower.as_str();

        let filtered: Vec<&crate::upgrade::InstalledPackage> = self
            .installed_packages
            .iter()
            .filter(|p| {
                search_lower.is_empty()
                    || p.name_lower.contains(search_lower)
                    || p.winget_id_lower.contains(search_lower)
            })
            .collect();

        let total = self.installed_packages.len();
        let shown = filtered.len();
        let subtitle = if shown < total {
            text(format!("{shown} of {total} installed packages (filtered)"))
                .size(13)
                .color(MUTED)
        } else {
            text(format!("{total} installed packages"))
                .size(13)
                .color(MUTED)
        };

        // Column headers — left padding accounts for checkbox width + row padding
        let col_headers = row![
            iced::widget::Space::new().width(30),
            text("Name").size(12).color(MUTED).width(Length::Fill),
            text("Version").size(12).color(MUTED).width(130),
            text("Size").size(12).color(MUTED).width(80),
            text("Package ID").size(12).color(MUTED).width(160),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .padding(padding::left(8).right(28));

        // Package list
        let mut pkg_list = column![].spacing(2).width(Length::Fill);

        for pkg in &filtered {
            let is_checked = self.uninstall_selected.contains(&pkg.winget_id_lower);
            let id = pkg.winget_id_lower.clone();

            let cb = checkbox(is_checked)
                .on_toggle(move |_| Message::ToggleUninstallPackage(id.clone()))
                .size(14)
                .style(package_checkbox_style);

            let pkg_row = row![
                cb,
                text(&pkg.name).size(13).width(Length::Fill),
                text(&pkg.version).size(12).color(MUTED_FG).width(130),
                text(format_size(pkg.size_bytes))
                    .size(12)
                    .color(MUTED_FG)
                    .width(80),
                text(&pkg.winget_id)
                    .size(12)
                    .font(iced::Font::MONOSPACE)
                    .color(MUTED)
                    .width(160),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .padding([6, 8]);

            let row_el: Element<'_, Message> = if is_checked {
                container(pkg_row)
                    .style(selected_row_style)
                    .width(Length::Fill)
                    .into()
            } else {
                container(pkg_row).width(Length::Fill).into()
            };

            pkg_list = pkg_list.push(row_el);
        }

        let scrollable_list = scrollable(pkg_list.padding(padding::right(20)))
            .height(Length::Fill)
            .width(Length::Fill);

        // Footer
        let selected_count = self.uninstall_selected.len();
        let selected_size: u64 = self
            .installed_packages
            .iter()
            .filter(|p| self.uninstall_selected.contains(&p.winget_id_lower))
            .filter_map(|p| p.size_bytes)
            .sum();
        let footer_label = if selected_size > 0 {
            format!(
                "{selected_count} selected \u{00b7} ~{}",
                format_size(Some(selected_size))
            )
        } else {
            format!("{selected_count} selected")
        };
        let footer_text = text(footer_label).size(13).color(MUTED);

        let mut review_btn = button(text("Review uninstall").size(14))
            .style(danger_button_style)
            .padding([8, 20]);
        if selected_count > 0 {
            review_btn = review_btn.on_press(Message::GoToUninstallReview);
        }

        let footer = row![
            footer_text,
            iced::widget::Space::new().width(Length::Fill),
            review_btn,
        ]
        .align_y(iced::Alignment::Center);

        let content = column![header, subtitle, col_headers, scrollable_list, footer]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }

    pub(crate) fn view_uninstall_review(&self) -> Element<'_, Message> {
        let header = back_header("Confirm uninstall");

        // Gather selected packages
        let queue: Vec<&crate::upgrade::InstalledPackage> = self
            .installed_packages
            .iter()
            .filter(|p| self.uninstall_selected.contains(&p.winget_id_lower))
            .collect();
        let count = queue.len();

        // Warning banner
        let warn_icon = text(char::from(Icon::TriangleAlert))
            .size(16)
            .font(LUCIDE_FONT)
            .color(STATUS_AMBER);
        let warn_title = text("This action cannot be undone").size(14).color(TEXT);
        let warn_body = text(
            "Selected packages will be permanently removed from this system. \
             Application data may also be deleted.",
        )
        .size(12)
        .color(MUTED_FG);

        let warn_banner = container(
            row![
                warn_icon,
                column![warn_title, warn_body]
                    .spacing(4)
                    .width(Length::Fill),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .padding([14, 16]),
        )
        .style(|_: &_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0x45 as f32 / 255.0,
                0x1a as f32 / 255.0,
                0x03 as f32 / 255.0,
            ))),
            border: iced::Border {
                color: iced::Color::from_rgb(
                    0x92 as f32 / 255.0,
                    0x40 as f32 / 255.0,
                    0x0e as f32 / 255.0,
                ),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .width(Length::Fill);

        // Summary
        let total_size: u64 = queue.iter().filter_map(|p| p.size_bytes).sum();
        let summary_label = if total_size > 0 {
            format!(
                "{count} packages to uninstall \u{00b7} Estimated ~{} will be freed",
                format_size(Some(total_size))
            )
        } else {
            format!("{count} packages to uninstall")
        };
        let summary = text(summary_label).size(13).color(MUTED);

        // Package list
        let mut pkg_list = column![].spacing(6).width(Length::Fill);

        for pkg in &queue {
            let x_icon = text(char::from(Icon::X))
                .size(13)
                .font(LUCIDE_FONT)
                .color(STATUS_RED);
            let name_text = text(&pkg.name).size(14);
            let detail = text(format!(
                "{} \u{00b7} {} \u{00b7} {}",
                pkg.winget_id,
                pkg.version,
                format_size(pkg.size_bytes)
            ))
            .size(12)
            .color(MUTED);

            let pkg_row = row![
                x_icon,
                column![name_text, detail].spacing(2).width(Length::Fill),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .padding([6, 4]);

            pkg_list = pkg_list.push(pkg_row);
        }

        let scrollable_list = scrollable(pkg_list.padding(padding::right(20)))
            .height(Length::Fill)
            .width(Length::Fill);

        // Footer
        let edit_btn = button(text("Edit").size(14))
            .on_press(Message::GoBack)
            .style(ghost_button_style)
            .padding([8, 20]);
        let uninstall_btn = button(text(format!("Uninstall {count} packages")).size(14))
            .on_press(Message::StartUninstall)
            .style(danger_button_style)
            .padding([8, 20]);

        let footer = row![
            iced::widget::Space::new().width(Length::Fill),
            edit_btn,
            uninstall_btn,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let content = column![header, warn_banner, summary, scrollable_list, footer]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }

    pub(crate) fn view_uninstalling(&self) -> Element<'_, Message> {
        view_progress_screen(
            &self.uninstall,
            ProgressLabels {
                verb: "Uninstalling",
                done_label: "Uninstall",
                dry_run_warning: "No packages will actually be uninstalled",
                cancel_msg: Message::CancelUninstall,
                done_msg: Message::FinishUninstallAndReset,
            },
            self.uninstall_queue.iter().map(|p| p.name.as_str()),
            self.dry_run,
            self.spinner_frame,
            None,
        )
    }

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
                text(char::from(Icon::Search)).size(14).font(LUCIDE_FONT),
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
                text("Name").size(12).color(MUTED).width(Length::Fill),
                text("Version").size(12).color(MUTED).width(100),
                text("Package ID").size(12).color(MUTED).width(200),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .padding(padding::left(8).right(28));

            let mut pkg_list = column![].spacing(2).width(Length::Fill);

            for pkg in &self.winget_search_results {
                let is_installed = self.installed_map.contains_key(&pkg.winget_id_lower);

                if is_installed {
                    let badge = text("installed").size(12).color(STATUS_GREEN);

                    let pkg_row = row![
                        badge,
                        text(&pkg.name).size(13).color(MUTED).width(Length::Fill),
                        text(&pkg.version).size(12).color(MUTED).width(100),
                        text(&pkg.winget_id)
                            .size(12)
                            .font(iced::Font::MONOSPACE)
                            .color(MUTED)
                            .width(200),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .padding([6, 8]);

                    pkg_list = pkg_list.push(container(pkg_row).width(Length::Fill));
                } else {
                    let is_checked = self.winget_search_selected.contains(&pkg.winget_id);
                    let id = pkg.winget_id.clone();

                    let cb = checkbox(is_checked)
                        .on_toggle(move |_| Message::ToggleWingetSearchPackage(id.clone()))
                        .size(14)
                        .style(package_checkbox_style);

                    let pkg_row = row![
                        cb,
                        text(&pkg.name).size(13).width(Length::Fill),
                        text(&pkg.version).size(12).color(MUTED_FG).width(100),
                        text(&pkg.winget_id)
                            .size(12)
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
                                background: Some(iced::Background::Color(CARD_BG)),
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

        let mut install_btn = button(text(format!("Install selected ({selected_count})")).size(14))
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
            ProgressLabels {
                verb: "Installing",
                done_label: "Installation",
                dry_run_warning: "No packages will actually be installed",
                cancel_msg: Message::CancelWingetSearchInstall,
                done_msg: Message::FinishWingetSearchInstall,
            },
            self.winget_search_queue.iter().map(|p| p.name.as_str()),
            self.dry_run,
            self.spinner_frame,
            None,
        )
    }

    pub(crate) fn view_github_login(&self) -> Element<'_, Message> {
        let header = back_header("Clone repos");

        let content: Element<'_, Message> = if let Some(ref error) = self.github_auth_error {
            container(
                column![
                    text(char::from(Icon::CircleX))
                        .size(32)
                        .font(LUCIDE_FONT)
                        .color(STATUS_RED),
                    text("Authentication failed").size(16),
                    text(error).size(13).color(MUTED),
                    button(text("Try again").size(14))
                        .style(continue_button_style)
                        .padding([8, 20])
                        .on_press(Message::GoToGitHubLogin),
                ]
                .spacing(12)
                .align_x(iced::Alignment::Center)
                .width(Length::Fill),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else if let Some(ref code) = self.github_user_code {
            let copy_btn = button(
                text(char::from(Icon::Copy))
                    .size(14)
                    .font(LUCIDE_FONT)
                    .color(MUTED_FG),
            )
            .style(ghost_button_style)
            .padding([4, 6])
            .on_press(Message::CopyDeviceCode);

            let code_display = container(
                row![text(code).size(28).font(iced::Font::MONOSPACE), copy_btn,]
                    .spacing(12)
                    .align_y(iced::Alignment::Center),
            )
            .padding([16, 32])
            .style(|_: &_| container::Style {
                background: Some(iced::Background::Color(CARD_BG)),
                border: iced::Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: BORDER,
                },
                ..Default::default()
            });

            let open_btn = button(
                row![
                    text(char::from(Icon::ExternalLink))
                        .size(14)
                        .font(LUCIDE_FONT),
                    text("Open GitHub").size(14),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .style(continue_button_style)
            .padding([10, 24])
            .on_press(Message::OpenGitHubUrl);

            container(
                column![
                    text("Sign in to GitHub").size(18),
                    text("Enter this code at github.com/login/device")
                        .size(13)
                        .color(MUTED),
                    code_display,
                    open_btn,
                    spinner_indicator(
                        self.spinner_frame,
                        "Waiting for authorization...".into(),
                        MUTED,
                    ),
                ]
                .spacing(16)
                .align_x(iced::Alignment::Center)
                .width(Length::Fill),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            container(spinner_indicator(
                self.spinner_frame,
                "Connecting to GitHub...".into(),
                MUTED,
            ))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        };

        let layout = column![header, content]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }

    pub(crate) fn view_github_repos(&self) -> Element<'_, Message> {
        let header = back_header("Your repositories");

        let search_field = text_input("Filter repos...", &self.search)
            .id(iced::widget::Id::new(SEARCH_INPUT_ID))
            .on_input(Message::SearchChanged)
            .padding(8)
            .size(14)
            .width(Length::Fill);

        let results_content: Element<'_, Message> = if self.github_repos_loading {
            container(spinner_indicator(
                self.spinner_frame,
                "Loading repositories...".into(),
                MUTED,
            ))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else if self.github_repos.is_empty() {
            let msg = if let Some(ref e) = self.github_auth_error {
                e.as_str()
            } else {
                "No repositories found"
            };
            container(text(msg).size(14).color(MUTED))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else {
            let sl = self.search_lower.as_str();
            let queued_names: std::collections::HashSet<&str> = self
                .github_clone_queue
                .iter()
                .map(|item| item.repo.full_name.as_str())
                .collect();

            let mut repo_list = column![].spacing(2).width(Length::Fill);

            for repo in &self.github_repos {
                if !sl.is_empty() && !repo.name_lower.contains(sl) && !repo.desc_lower.contains(sl)
                {
                    continue;
                }

                let is_queued = queued_names.contains(repo.full_name.as_str());

                let visibility_badge = if repo.private {
                    text("private").size(12).color(STATUS_AMBER)
                } else {
                    text("public").size(12).color(MUTED)
                };

                let desc = text(repo.description.as_deref().unwrap_or(""))
                    .size(12)
                    .color(MUTED);

                let name_col = column![
                    row![text(&repo.name).size(14), visibility_badge,]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                    desc,
                ]
                .spacing(2)
                .width(Length::Fill);

                let action: Element<'_, Message> = if is_queued {
                    text(char::from(Icon::Check))
                        .size(14)
                        .font(LUCIDE_FONT)
                        .color(STATUS_GREEN)
                        .into()
                } else {
                    let full_name = repo.full_name.clone();
                    button(text("Select folder").size(12))
                        .style(ghost_button_style)
                        .padding([4, 10])
                        .on_press(Message::GitHubSelectFolder(full_name))
                        .into()
                };

                let repo_row = row![name_col, action]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .padding([8, 12]);

                let row_el: Element<'_, Message> = if is_queued {
                    container(repo_row)
                        .style(selected_row_style)
                        .width(Length::Fill)
                        .into()
                } else {
                    container(repo_row).width(Length::Fill).into()
                };

                repo_list = repo_list.push(row_el);
            }

            scrollable(repo_list.padding(padding::right(20)))
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        };

        // Clone queue section
        let queue_section: Element<'_, Message> = if self.github_clone_queue.is_empty() {
            iced::widget::Space::new().height(0).into()
        } else {
            let mut queue_col = column![text("Clone queue").size(13).color(MUTED),]
                .spacing(4)
                .width(Length::Fill);

            for item in &self.github_clone_queue {
                let full_name = item.repo.full_name.clone();
                let remove_btn = button(text(char::from(Icon::X)).size(12).font(LUCIDE_FONT))
                    .style(ghost_button_style)
                    .padding([2, 6])
                    .on_press(Message::GitHubRemoveFromQueue(full_name));

                let queue_row = row![
                    text(&item.repo.name).size(13),
                    text(char::from(Icon::ArrowRight))
                        .size(12)
                        .font(LUCIDE_FONT)
                        .color(MUTED),
                    text(item.destination.display().to_string())
                        .size(12)
                        .font(iced::Font::MONOSPACE)
                        .color(MUTED_FG),
                    iced::widget::Space::new().width(Length::Fill),
                    remove_btn,
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center);

                queue_col = queue_col.push(queue_row);
            }

            queue_col.into()
        };

        // Footer
        let queue_count = self.github_clone_queue.len();
        let mut clone_btn = button(text(format!("Clone all ({queue_count})")).size(14))
            .style(continue_button_style)
            .padding([8, 20]);
        if queue_count > 0 {
            clone_btn = clone_btn.on_press(Message::StartGitHubClone);
        }

        let footer = row![iced::widget::Space::new().width(Length::Fill), clone_btn,]
            .align_y(iced::Alignment::Center);

        let layout = column![header, search_field, results_content, queue_section, footer]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }

    pub(crate) fn view_github_cloning(&self) -> Element<'_, Message> {
        view_progress_screen(
            &self.github_clone,
            ProgressLabels {
                verb: "Cloning",
                done_label: "Clone",
                dry_run_warning: "No repos will actually be cloned",
                cancel_msg: Message::CancelGitHubClone,
                done_msg: Message::FinishGitHubClone,
            },
            self.github_clone_queue
                .iter()
                .map(|item| item.repo.name.as_str()),
            self.dry_run,
            self.spinner_frame,
            None,
        )
    }

    pub(crate) fn view_github_bootstrap(&self) -> Element<'_, Message> {
        let header = text("Setup scripts detected").size(18);
        let subtitle = text("These repos have bootstrap scripts that can set things up for you.")
            .size(13)
            .color(MUTED);

        let mut list = column![].spacing(8).width(Length::Fill);

        for (idx, item) in self.github_bootstrap_items.iter().enumerate() {
            let status_indicator: Element<'_, Message> = match &item.status {
                BootstrapStatus::Pending => iced::widget::Space::new().width(0).into(),
                BootstrapStatus::Running => {
                    spinner_indicator(self.spinner_frame, "Running...".into(), STATUS_BLUE)
                }
                BootstrapStatus::Done => text(char::from(Icon::Check))
                    .size(14)
                    .font(LUCIDE_FONT)
                    .color(STATUS_GREEN)
                    .into(),
                BootstrapStatus::Skipped => text("skipped").size(12).color(MUTED).into(),
                BootstrapStatus::Failed(e) => text(e).size(12).color(STATUS_RED).into(),
            };

            let actions: Element<'_, Message> = if item.status == BootstrapStatus::Pending {
                if item.scripts.len() == 1 {
                    let script = item.scripts[0].clone();
                    let run_btn = button(
                        row![
                            text(char::from(Icon::Play)).size(12).font(LUCIDE_FONT),
                            text(format!("Run {}", &item.scripts[0])).size(12),
                        ]
                        .spacing(6)
                        .align_y(iced::Alignment::Center),
                    )
                    .style(continue_button_style)
                    .padding([4, 12])
                    .on_press(Message::GitHubRunBootstrap(idx, script));

                    let skip_btn = button(text("Skip").size(12))
                        .style(ghost_button_style)
                        .padding([4, 10])
                        .on_press(Message::GitHubSkipBootstrap(idx));

                    row![run_btn, skip_btn].spacing(6).into()
                } else {
                    let mut btns = row![].spacing(4);
                    for script in &item.scripts {
                        let s = script.clone();
                        btns = btns.push(
                            button(text(script).size(12))
                                .style(ghost_button_style)
                                .padding([4, 8])
                                .on_press(Message::GitHubRunBootstrap(idx, s)),
                        );
                    }
                    btns = btns.push(
                        button(text("Skip").size(12))
                            .style(ghost_button_style)
                            .padding([4, 10])
                            .on_press(Message::GitHubSkipBootstrap(idx)),
                    );
                    btns.into()
                }
            } else {
                iced::widget::Space::new().width(0).into()
            };

            let item_row = row![
                column![
                    text(&item.repo_name).size(14),
                    text(item.repo_path.display().to_string())
                        .size(12)
                        .font(iced::Font::MONOSPACE)
                        .color(MUTED),
                ]
                .spacing(2)
                .width(Length::Fill),
                status_indicator,
                actions,
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .padding([10, 12]);

            list = list.push(
                container(item_row)
                    .style(selected_row_style)
                    .width(Length::Fill),
            );
        }

        let all_handled = self.github_bootstrap_items.iter().all(|item| {
            !matches!(
                item.status,
                BootstrapStatus::Pending | BootstrapStatus::Running
            )
        });

        let mut done_btn = button(text("Done").size(14))
            .style(continue_button_style)
            .padding([8, 20]);
        if all_handled {
            done_btn = done_btn.on_press(Message::FinishGitHubBootstrap);
        }

        let footer = row![iced::widget::Space::new().width(Length::Fill), done_btn,];

        let layout = column![header, subtitle, list, footer]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);

        container(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(28)
            .into()
    }
}

fn format_size(bytes: Option<u64>) -> String {
    match bytes {
        None => "\u{2014}".into(), // em dash
        Some(b) if b < 1024 => format!("{b} B"),
        Some(b) if b < 1024 * 1024 => format!("{:.0} KB", b as f64 / 1024.0),
        Some(b) if b < 1024 * 1024 * 1024 => format!("{:.0} MB", b as f64 / (1024.0 * 1024.0)),
        Some(b) => format!("{:.1} GB", b as f64 / (1024.0 * 1024.0 * 1024.0)),
    }
}

/// Back-button header: chevron-left icon button + heading text.
fn back_header(title: &str) -> iced::widget::Row<'_, Message> {
    let back_icon = text(char::from(Icon::ChevronLeft))
        .size(18)
        .font(LUCIDE_FONT);
    let back_btn = button(back_icon)
        .on_press(Message::GoBack)
        .style(ghost_button_style)
        .padding([6, 8]);

    row![back_btn, text(title).size(18)]
        .spacing(10)
        .align_y(iced::Alignment::Center)
}

/// Back-button header with a search field on the right.
fn search_header<'a>(title: &'a str, search: &'a str) -> iced::widget::Row<'a, Message> {
    let search_field = text_input("Search...", search)
        .id(iced::widget::Id::new(SEARCH_INPUT_ID))
        .on_input(Message::SearchChanged)
        .padding(8)
        .size(14)
        .width(200);

    back_header(title)
        .push(iced::widget::Space::new().width(Length::Fill))
        .push(search_field)
}

/// A setting row: label + description on the left, widget on the right.
fn setting_row<'a>(
    label: &'a str,
    description: &'a str,
    widget: Element<'a, Message>,
) -> Element<'a, Message> {
    row![
        column![
            text(label).size(14),
            text(description).size(12).color(MUTED),
        ]
        .spacing(2)
        .width(Length::Fill),
        widget,
    ]
    .spacing(16)
    .align_y(iced::Alignment::Center)
    .into()
}

/// A toggle row: label + description on the left, toggler on the right.
fn toggle_row<'a>(
    label: &'a str,
    description: &'a str,
    is_on: bool,
    on_toggle: fn(bool) -> Message,
) -> Element<'a, Message> {
    row![
        column![
            text(label).size(14),
            text(description).size(12).color(MUTED),
        ]
        .spacing(2)
        .width(Length::Fill),
        toggler(is_on).on_toggle(on_toggle).size(20),
    ]
    .spacing(16)
    .align_y(iced::Alignment::Center)
    .into()
}

/// Terminal log box: monospace text in a dark container, auto-scrolled to bottom.
/// Uses the `LogBuffer::joined()` cache to avoid re-joining lines every frame.
fn terminal_log_box(log: &LogBuffer) -> iced::widget::Container<'_, Message> {
    let terminal_text = log.joined().to_string();

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

/// Config that varies between progress screens (install, upgrade, uninstall, clone).
struct ProgressLabels {
    /// Present participle, e.g. "Installing" or "Upgrading"
    verb: &'static str,
    /// Noun for the done heading, e.g. "Installation" or "Upgrade"
    done_label: &'static str,
    /// Dry-run subtitle, e.g. "No packages will actually be installed"
    dry_run_warning: &'static str,
    cancel_msg: Message,
    done_msg: Message,
}

/// Shared layout for progress screens (install, upgrade, uninstall, clone).
fn view_progress_screen<'a>(
    state: &'a ProgressState,
    labels: ProgressLabels,
    names: impl Iterator<Item = &'a str>,
    dry_run: bool,
    spinner_frame: usize,
    extra_content: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let ProgressLabels {
        verb,
        done_label,
        dry_run_warning,
        cancel_msg,
        done_msg,
    } = labels;
    let names: Vec<&str> = names.collect();
    let total = names.len();
    let (done_count, failed_count, cancelled_count) = state.status_counts();

    // Heading row: "Installing" + "3 of 12" muted
    let heading_row = if state.done {
        let label = match (dry_run, cancelled_count > 0) {
            (true, true) => "Dry Run Cancelled".to_string(),
            (true, false) => "Dry Run Complete".to_string(),
            (false, true) => format!("{} Cancelled", done_label),
            (false, false) => format!("{} Complete", done_label),
        };
        row![text(label).size(20)]
            .spacing(8)
            .align_y(iced::Alignment::Center)
    } else {
        let verb_text = if dry_run {
            format!("[DRY RUN] {}", verb)
        } else {
            verb.to_string()
        };
        let count_text = format!(
            "{} of {total} \u{00b7} {}",
            state.current + 1,
            state.elapsed_display()
        );
        row![
            text(verb_text).size(20),
            text(count_text).size(14).color(MUTED),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
    };

    let subtitle: Element<'_, Message> = if state.done {
        let mut counts = row![].spacing(6).align_y(iced::Alignment::Center);

        counts = counts
            .push(
                text(char::from(Icon::CircleCheck))
                    .size(13)
                    .font(LUCIDE_FONT)
                    .color(STATUS_GREEN),
            )
            .push(
                text(format!("{done_count} succeeded"))
                    .size(13)
                    .color(STATUS_GREEN),
            );

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
    } else if dry_run {
        text(dry_run_warning).size(13).color(STATUS_AMBER).into()
    } else {
        let name = names.get(state.current).unwrap_or(&"...");
        text(*name).size(13).color(MUTED).into()
    };

    let completed = (done_count + failed_count + cancelled_count) as f32;
    let progress = progress_bar(0.0..=total as f32, completed);

    let active_label = format!("{}...", verb);
    let mut pkg_list = column![].spacing(2).width(Length::Fill);
    for (i, name) in names.iter().enumerate() {
        let (icon, color, label): (Element<'_, Message>, _, _) = match &state.statuses[i] {
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
                text(SPINNER_FRAMES[spinner_frame])
                    .size(14)
                    .color(STATUS_BLUE)
                    .into(),
                STATUS_BLUE,
                active_label.clone(),
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

    let mut cancel_btn = button(text("Cancel").size(14))
        .style(cancel_button_style)
        .padding([8, 20]);
    if !state.done {
        cancel_btn = cancel_btn.on_press(cancel_msg);
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
        done_btn = done_btn.on_press(done_msg);
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

    if let Some(extra) = extra_content {
        content = content.push(extra);
    }

    content = content.push(footer);

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

    let mut pkg_row = row![cb].spacing(8).align_y(iced::Alignment::Center);
    if installed {
        let badge_label = text("Installed").size(12).color(STATUS_GREEN);
        let badge = container(badge_label)
            .style(installed_badge_style)
            .padding([1, 6]);
        pkg_row = pkg_row.push(badge);
    }
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

    container(pkg_row).padding([4, 0]).into()
}

/// Tool tile: icon circle + label, used on the dashboard home screen.
fn tool_tile<'a>(
    icon: Icon,
    label: &'a str,
    color: iced::Color,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let bg_color = tinted_icon_bg(color);
    let icon_bg = container(
        text(char::from(icon))
            .size(14)
            .font(LUCIDE_FONT)
            .color(color),
    )
    .width(32)
    .height(32)
    .center_x(32)
    .center_y(32)
    .style(move |_theme: &Theme| container::Style {
        background: Some(iced::Background::Color(bg_color)),
        border: iced::Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    let content = column![icon_bg, text(label).size(12)]
        .spacing(8)
        .align_x(iced::Alignment::Center);

    let mut btn = button(container(content).padding([18, 10]).center_x(Length::Fill))
        .width(Length::Fill)
        .style(tool_tile_style);

    if let Some(msg) = on_press {
        btn = btn.on_press(msg);
    }

    btn.into()
}

/// Small icon + label row used for status indicators (e.g. catalog source, scan progress).
fn status_indicator(icon: Icon, label: String, color: iced::Color) -> Element<'static, Message> {
    row![
        text(char::from(icon))
            .size(12)
            .font(LUCIDE_FONT)
            .color(color),
        text(label).size(12).color(color),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .into()
}

/// Animated spinner + label row for loading indicators.
fn spinner_indicator(frame: usize, label: String, color: iced::Color) -> Element<'static, Message> {
    row![
        text(SPINNER_FRAMES[frame]).size(12).color(color),
        text(label).size(12).color(color),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .into()
}

/// A tab button for the settings screen.
fn tab_button<'a>(
    label: &'a str,
    tab: SettingsTab,
    active_tab: SettingsTab,
) -> Element<'a, Message> {
    let active = tab == active_tab;
    button(text(label).size(13))
        .on_press(Message::SetSettingsTab(tab))
        .style(move |theme: &Theme, status| tab_style(theme, status, active))
        .padding([6, 16])
        .into()
}

const CHANGELOG_RAW: &str = include_str!("../CHANGELOG.md");

enum ChangelogLine {
    Version(String),
    Section(String),
    Item(String),
}

fn parse_changelog(raw: &str) -> Vec<ChangelogLine> {
    raw.lines()
        .filter_map(|line| {
            line.strip_prefix("## ")
                .map(|r| ChangelogLine::Version(r.to_string()))
                .or_else(|| {
                    line.strip_prefix("### ")
                        .map(|r| ChangelogLine::Section(r.to_string()))
                })
                .or_else(|| {
                    line.strip_prefix("- ")
                        .map(|r| ChangelogLine::Item(r.to_string()))
                })
        })
        .collect()
}

fn view_settings_changelog<'a>() -> Element<'a, Message> {
    use std::sync::LazyLock;
    static CHANGELOG: LazyLock<Vec<ChangelogLine>> =
        LazyLock::new(|| parse_changelog(CHANGELOG_RAW));
    let lines = &*CHANGELOG;
    let mut col = column![].spacing(4).width(Length::Fill);

    for line in lines.iter() {
        match line {
            ChangelogLine::Version(v) => {
                col = col.push(container(text(v).size(16).color(TEXT)).padding(padding::top(8)));
            }
            ChangelogLine::Section(s) => {
                col = col.push(
                    container(text(s.to_uppercase()).size(12).color(MUTED_FG))
                        .padding(padding::top(8)),
                );
            }
            ChangelogLine::Item(item) => {
                col = col.push(
                    container(text(format!("\u{2022}  {item}")).size(13).color(MUTED_FG))
                        .padding(padding::left(8)),
                );
            }
        }
    }

    scrollable(col)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
