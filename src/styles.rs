use iced::widget::{button, checkbox, container};
use iced::{Background, Border, Color, Font, Shadow, Theme};

/// The Lucide icon font (from the `lucide-icons` crate).
pub const LUCIDE_FONT: Font = Font::with_name("lucide");

// ── Zinc neutral palette ──────────────────────────────────────────
pub const TEXT: Color = Color::from_rgb(
    0xfa as f32 / 255.0,
    0xfa as f32 / 255.0,
    0xfa as f32 / 255.0,
); // zinc-50
pub const MUTED_FG: Color = Color::from_rgb(
    0xa1 as f32 / 255.0,
    0xa1 as f32 / 255.0,
    0xaa as f32 / 255.0,
); // zinc-400
pub const MUTED: Color = Color::from_rgb(
    0x71 as f32 / 255.0,
    0x71 as f32 / 255.0,
    0x7a as f32 / 255.0,
); // zinc-500
pub const TERMINAL_TEXT: Color = MUTED_FG; // zinc-400
pub const CARD_BG: Color = Color::from_rgb(
    0x18 as f32 / 255.0,
    0x18 as f32 / 255.0,
    0x1b as f32 / 255.0,
); // zinc-900
pub const CARD_HOVER: Color = Color::from_rgb(
    0x27 as f32 / 255.0,
    0x27 as f32 / 255.0,
    0x2a as f32 / 255.0,
); // zinc-800
pub const BORDER: Color = Color::from_rgb(
    0x27 as f32 / 255.0,
    0x27 as f32 / 255.0,
    0x2a as f32 / 255.0,
); // zinc-800
pub const BORDER_FOCUS: Color = Color::from_rgb(
    0x3f as f32 / 255.0,
    0x3f as f32 / 255.0,
    0x46 as f32 / 255.0,
); // zinc-700

// ── Accent colors ─────────────────────────────────────────────────
pub const STATUS_BLUE: Color = Color::from_rgb(
    0x3b as f32 / 255.0,
    0x82 as f32 / 255.0,
    0xf6 as f32 / 255.0,
); // blue-500
pub const STATUS_GREEN: Color = Color::from_rgb(
    0x10 as f32 / 255.0,
    0xb9 as f32 / 255.0,
    0x81 as f32 / 255.0,
); // emerald-500
pub const STATUS_RED: Color = Color::from_rgb(
    0xef as f32 / 255.0,
    0x44 as f32 / 255.0,
    0x44 as f32 / 255.0,
); // red-500
pub const STATUS_AMBER: Color = Color::from_rgb(
    0xf5 as f32 / 255.0,
    0x9e as f32 / 255.0,
    0x0b as f32 / 255.0,
); // amber-500

const PRIMARY_HOVER: Color = Color::from_rgb(
    0x25 as f32 / 255.0,
    0x63 as f32 / 255.0,
    0xeb as f32 / 255.0,
); // blue-600

// ── Button styles ─────────────────────────────────────────────────

pub fn continue_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text_color) = match status {
        button::Status::Hovered => (PRIMARY_HOVER, Color::WHITE),
        button::Status::Pressed => (STATUS_BLUE, Color::WHITE),
        button::Status::Disabled => (CARD_BG, MUTED),
        button::Status::Active => (STATUS_BLUE, Color::WHITE),
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color,
        border: Border {
            color: if matches!(status, button::Status::Disabled) {
                BORDER
            } else {
                Color::TRANSPARENT
            },
            width: if matches!(status, button::Status::Disabled) {
                1.0
            } else {
                0.0
            },
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn cancel_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text_color, border_color) = match status {
        button::Status::Hovered => (
            Color::from_rgba(
                0xef as f32 / 255.0,
                0x44 as f32 / 255.0,
                0x44 as f32 / 255.0,
                0.1,
            ),
            STATUS_RED,
            STATUS_RED,
        ),
        button::Status::Pressed => (
            Color::from_rgba(
                0xef as f32 / 255.0,
                0x44 as f32 / 255.0,
                0x44 as f32 / 255.0,
                0.15,
            ),
            STATUS_RED,
            STATUS_RED,
        ),
        button::Status::Disabled => (Color::TRANSPARENT, MUTED, BORDER),
        button::Status::Active => (Color::TRANSPARENT, STATUS_RED, BORDER),
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn ghost_icon_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text_color) = match status {
        button::Status::Hovered => (CARD_BG, TEXT),
        button::Status::Pressed => (CARD_HOVER, TEXT),
        _ => (Color::TRANSPARENT, MUTED_FG),
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn ghost_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text_color) = match status {
        button::Status::Hovered => (CARD_BG, TEXT),
        button::Status::Pressed => (CARD_HOVER, TEXT),
        _ => (Color::TRANSPARENT, MUTED_FG),
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

// ── Card styles ───────────────────────────────────────────────────

pub fn card_style(_theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let base_bg = if selected { CARD_HOVER } else { CARD_BG };

    let border_color = if selected { BORDER_FOCUS } else { BORDER };

    let background = match status {
        button::Status::Hovered => CARD_HOVER,
        _ => base_bg,
    };

    let hover_border = match status {
        button::Status::Hovered => BORDER_FOCUS,
        _ => border_color,
    };

    button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: TEXT,
        border: Border {
            color: hover_border,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn update_card_style(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => CARD_BG,
        _ => Color::TRANSPARENT,
    };

    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: MUTED_FG,
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

// ── Container styles ──────────────────────────────────────────────

pub fn terminal_box_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(
            0x0a as f32 / 255.0,
            0x0a as f32 / 255.0,
            0x0a as f32 / 255.0,
        ))),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub fn installed_badge_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(
            0x06 as f32 / 255.0,
            0x5f as f32 / 255.0,
            0x46 as f32 / 255.0,
        ))),
        border: Border {
            color: Color::from_rgba(
                0x10 as f32 / 255.0,
                0xb9 as f32 / 255.0,
                0x81 as f32 / 255.0,
                0.3,
            ),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

pub fn warning_badge_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0xf5 as f32 / 255.0,
            0x9e as f32 / 255.0,
            0x0b as f32 / 255.0,
            0.15,
        ))),
        border: Border {
            color: Color::from_rgba(
                0xf5 as f32 / 255.0,
                0x9e as f32 / 255.0,
                0x0b as f32 / 255.0,
                0.3,
            ),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

pub fn icon_box_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(CARD_BG)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub fn divider_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(BORDER)),
        ..Default::default()
    }
}

// ── Checkbox styles ───────────────────────────────────────────────

pub fn package_checkbox_style(_theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    match status {
        checkbox::Status::Active { is_checked } | checkbox::Status::Hovered { is_checked } => {
            let hovered = matches!(status, checkbox::Status::Hovered { .. });
            if is_checked {
                checkbox::Style {
                    background: Background::Color(if hovered {
                        PRIMARY_HOVER
                    } else {
                        STATUS_BLUE
                    }),
                    icon_color: Color::WHITE,
                    border: Border {
                        radius: 3.0.into(),
                        width: 1.0,
                        color: if hovered { PRIMARY_HOVER } else { STATUS_BLUE },
                    },
                    text_color: Some(TEXT),
                }
            } else {
                checkbox::Style {
                    background: Background::Color(Color::TRANSPARENT),
                    icon_color: Color::WHITE,
                    border: Border {
                        radius: 3.0.into(),
                        width: 1.0,
                        color: if hovered { MUTED_FG } else { BORDER_FOCUS },
                    },
                    text_color: Some(MUTED_FG),
                }
            }
        }
        checkbox::Status::Disabled { is_checked } => {
            if is_checked {
                checkbox::Style {
                    background: Background::Color(BORDER_FOCUS),
                    icon_color: MUTED,
                    border: Border {
                        radius: 3.0.into(),
                        width: 1.0,
                        color: BORDER_FOCUS,
                    },
                    text_color: Some(MUTED),
                }
            } else {
                checkbox::Style {
                    background: Background::Color(Color::TRANSPARENT),
                    icon_color: MUTED,
                    border: Border {
                        radius: 3.0.into(),
                        width: 1.0,
                        color: BORDER,
                    },
                    text_color: Some(MUTED),
                }
            }
        }
    }
}
