use iced::widget::{button, container};
use iced::{Border, Color, Shadow, Theme};

// Named color constants
pub const MUTED: Color = Color::from_rgb(0.55, 0.55, 0.58);
pub const TERMINAL_TEXT: Color = Color::from_rgb(0.7, 0.7, 0.7);
pub const STATUS_BLUE: Color = Color::from_rgb(0.3, 0.6, 1.0);
pub const STATUS_GREEN: Color = Color::from_rgb(0.3, 0.8, 0.4);
pub const STATUS_RED: Color = Color::from_rgb(0.9, 0.3, 0.3);
pub const STATUS_AMBER: Color = Color::from_rgb(1.0, 0.75, 0.2);

// Lucide icon codepoints
pub const ICON_CIRCLE: char = '\u{e098}';
pub const ICON_LOADER: char = '\u{e114}';
pub const ICON_CIRCLE_CHECK: char = '\u{e09a}';
pub const ICON_CIRCLE_X: char = '\u{e0a2}';

/// Shared helper for colored button styles — `continue_button_style` and
/// `cancel_button_style` differ only in their base/hover colors.
fn colored_button_style(base: Color, hover: Color, status: button::Status) -> button::Style {
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

pub fn back_button_style(theme: &Theme, status: button::Status) -> button::Style {
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

pub fn continue_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = Color::from_rgb(0.24, 0.50, 0.96);
    let hover = Color::from_rgb(0.30, 0.56, 1.0);
    colored_button_style(base, hover, status)
}

pub fn cancel_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = Color::from_rgb(0.7, 0.2, 0.2);
    let hover = Color::from_rgb(0.85, 0.25, 0.25);
    colored_button_style(base, hover, status)
}

pub fn terminal_box_style(_theme: &Theme) -> container::Style {
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

pub fn installed_badge_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.25, 0.15))),
        border: Border {
            color: Color::from_rgb(0.25, 0.45, 0.25),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

pub fn card_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
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
