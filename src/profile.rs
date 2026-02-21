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

    /// Lucide icon codepoint for this profile.
    pub fn icon(self) -> char {
        match self {
            Profile::Personal => '\u{e0f5}', // house
            Profile::Work => '\u{e062}',     // briefcase
            Profile::Homelab => '\u{e153}',  // server
            Profile::Manual => '\u{e154}',   // settings
        }
    }
}
