mod profile;
mod theme;

use iced::widget::{button, column, container, row, text};
use iced::{Border, Color, Element, Length, Shadow, Size, Theme};

use profile::Profile;

fn main() -> iced::Result {
    iced::application("Provision", App::update, App::view)
        .theme(App::theme)
        .window_size(Size::new(900.0, 600.0))
        .run()
}

#[derive(Debug, Default)]
struct App {
    selected_profile: Option<Profile>,
    screen: Screen,
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
}

const MUTED: Color = Color::from_rgb(0.55, 0.55, 0.58);

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::ProfileSelected(profile) => {
                self.selected_profile = Some(profile);
                self.screen = Screen::PackageSelect;
            }
            Message::GoBack => {
                self.screen = Screen::ProfileSelect;
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

        let back = button(text("Back").size(14))
            .on_press(Message::GoBack)
            .style(|theme: &Theme, status| {
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
            })
            .padding([6, 16]);

        let heading = text(format!("{} — Package Selection", profile.title())).size(28);
        let placeholder = text("Package catalog coming soon...")
            .size(16)
            .color(MUTED);

        let content = column![back, heading, placeholder]
            .spacing(20)
            .align_x(iced::Alignment::Start);

        container(content)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
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
