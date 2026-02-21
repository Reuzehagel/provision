mod catalog;
mod profile;
mod theme;

use std::collections::HashSet;

use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{Border, Color, Element, Length, Shadow, Size, Theme, padding};

use catalog::Package;
use profile::Profile;

fn main() -> iced::Result {
    iced::application("Provision", App::update, App::view)
        .theme(App::theme)
        .window_size(Size::new(900.0, 600.0))
        .run()
}

struct App {
    selected_profile: Option<Profile>,
    screen: Screen,
    catalog: Vec<Package>,
    selected: HashSet<String>,
    search: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            selected_profile: None,
            screen: Screen::default(),
            catalog: catalog::load_catalog(),
            selected: HashSet::new(),
            search: String::new(),
        }
    }
}

#[derive(Debug, Default)]
enum Screen {
    #[default]
    ProfileSelect,
    PackageSelect,
}

#[derive(Debug, Clone)]
enum Message {
    ProfileSelected(Profile),
    GoBack,
    TogglePackage(String),
    SearchChanged(String),
}

const MUTED: Color = Color::from_rgb(0.55, 0.55, 0.58);

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::ProfileSelected(profile) => {
                self.selected_profile = Some(profile);
                self.selected = catalog::default_selection(&self.catalog, profile);
                self.search.clear();
                self.screen = Screen::PackageSelect;
            }
            Message::GoBack => {
                self.search.clear();
                self.screen = Screen::ProfileSelect;
            }
            Message::TogglePackage(id) => {
                if !self.selected.remove(&id) {
                    self.selected.insert(id);
                }
            }
            Message::SearchChanged(value) => {
                self.search = value;
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match self.screen {
            Screen::ProfileSelect => self.view_profile_select(),
            Screen::PackageSelect => self.view_package_select(),
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

        let back = button(text("< Back").size(14))
            .on_press(Message::GoBack)
            .style(back_button_style)
            .padding([6, 16]);

        let heading = text(format!("{} — Package Selection", profile.title())).size(28);

        let header = row![back, heading]
            .spacing(16)
            .align_y(iced::Alignment::Center);

        let search_field = text_input("Search packages...", &self.search)
            .on_input(Message::SearchChanged)
            .padding(10)
            .size(16);

        let search_lower = self.search.to_lowercase();

        let categories = catalog::categories(&self.catalog);
        let mut pkg_list = column![].spacing(24);

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

                let cb = checkbox(&pkg.name, is_checked)
                    .on_toggle(move |_| Message::TogglePackage(id.clone()))
                    .size(18)
                    .text_size(15);

                let desc = container(text(&pkg.description).size(13).color(MUTED))
                    .padding(padding::left(30));

                cat_col = cat_col.push(cb).push(desc);
            }

            pkg_list = pkg_list.push(cat_col);
        }

        let scrollable_list = scrollable(pkg_list).height(Length::Fill);

        let count = self.selected.len();
        let footer = text(format!("{count} packages selected"))
            .size(14)
            .color(MUTED);

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
}

fn profile_card(profile: Profile, selected: Option<Profile>) -> Element<'static, Message> {
    let is_selected = selected == Some(profile);

    let icon = text(profile.icon()).size(32);
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
    }
}
