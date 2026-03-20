use iced::Theme;
use iced::theme::Palette;

use crate::styles;

pub fn default() -> Theme {
    Theme::custom(
        "provision",
        Palette {
            background: styles::BG,
            text: styles::TEXT,
            primary: styles::STATUS_BLUE,
            success: styles::STATUS_GREEN,
            danger: styles::STATUS_RED,
            warning: styles::STATUS_AMBER,
        },
    )
}
