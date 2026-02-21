#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Personal,
    Work,
    Homelab,
    Manual,
}

impl Profile {
    pub const ALL: [Profile; 4] = [
        Profile::Personal,
        Profile::Work,
        Profile::Homelab,
        Profile::Manual,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Profile::Personal => "Personal",
            Profile::Work => "Work",
            Profile::Homelab => "Homelab",
            Profile::Manual => "Manual",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Profile::Personal => "Browsers, media, gaming, and everyday tools",
            Profile::Work => "Dev tools, communication, and productivity apps",
            Profile::Homelab => "Server utilities, containers, and networking tools",
            Profile::Manual => "Start from scratch \u{2014} pick exactly what you want",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Profile::Personal => "personal",
            Profile::Work => "work",
            Profile::Homelab => "homelab",
            Profile::Manual => "manual",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Profile::Personal => "\u{1F3E0}",
            Profile::Work => "\u{1F4BC}",
            Profile::Homelab => "\u{1F5A5}\u{FE0F}",
            Profile::Manual => "\u{2699}\u{FE0F}",
        }
    }
}
