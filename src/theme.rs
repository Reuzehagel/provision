use iced::theme::Palette;
use iced::{Color, Theme};

pub fn default() -> Theme {
    Theme::custom(
        "provision",
        Palette {
            background: Color::from_rgb(
                0x09 as f32 / 255.0,
                0x09 as f32 / 255.0,
                0x0b as f32 / 255.0,
            ), // zinc-950
            text: Color::from_rgb(
                0xfa as f32 / 255.0,
                0xfa as f32 / 255.0,
                0xfa as f32 / 255.0,
            ), // zinc-50
            primary: Color::from_rgb(
                0x3b as f32 / 255.0,
                0x82 as f32 / 255.0,
                0xf6 as f32 / 255.0,
            ), // blue-500
            success: Color::from_rgb(
                0x10 as f32 / 255.0,
                0xb9 as f32 / 255.0,
                0x81 as f32 / 255.0,
            ), // emerald-500
            danger: Color::from_rgb(
                0xef as f32 / 255.0,
                0x44 as f32 / 255.0,
                0x44 as f32 / 255.0,
            ), // red-500
            warning: Color::from_rgb(
                0xf5 as f32 / 255.0,
                0x9e as f32 / 255.0,
                0x0b as f32 / 255.0,
            ), // amber-500
        },
    )
}
