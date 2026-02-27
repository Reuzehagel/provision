#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Laptop,
    Desktop,
    Manual,
}

impl Profile {
    pub const ALL: [Profile; 3] = [Profile::Laptop, Profile::Desktop, Profile::Manual];

    pub fn title(self) -> &'static str {
        match self {
            Profile::Laptop => "Laptop",
            Profile::Desktop => "Desktop",
            Profile::Manual => "Manual",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Profile::Laptop => "Portable essentials",
            Profile::Desktop => "Full setup",
            Profile::Manual => "Start from scratch",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Profile::Laptop => "laptop",
            Profile::Desktop => "desktop",
            Profile::Manual => "manual",
        }
    }

    /// Lucide icon for this profile.
    pub fn icon(self) -> char {
        use lucide_icons::Icon;
        match self {
            Profile::Laptop => char::from(Icon::Laptop),
            Profile::Desktop => char::from(Icon::Monitor),
            Profile::Manual => char::from(Icon::Settings),
        }
    }
}
