use iced::theme::Palette;
use iced::{Color, Theme};

use crate::styles;

pub fn default() -> Theme {
    Theme::custom(
        "provision",
        Palette {
            background: Color::from_rgb(
                0x09 as f32 / 255.0,
                0x09 as f32 / 255.0,
                0x0b as f32 / 255.0,
            ), // zinc-950
            text: styles::TEXT,
            primary: styles::STATUS_BLUE,
            success: styles::STATUS_GREEN,
            danger: styles::STATUS_RED,
            warning: styles::STATUS_AMBER,
        },
    )
}
