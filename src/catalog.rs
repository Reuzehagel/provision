use std::collections::HashSet;

use serde::Deserialize;

use crate::profile::Profile;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub winget_id: Option<String>,
    #[serde(default)]
    pub profiles: Vec<String>,
    pub post_install: Option<String>,
    pub install_command: Option<String>,
}

#[derive(Deserialize)]
struct CatalogFile {
    packages: Vec<Package>,
}

pub fn load_catalog() -> Vec<Package> {
    let raw = include_str!("../packages.toml");
    let file: CatalogFile = toml::from_str(raw).expect("packages.toml should be valid");
    file.packages
}

/// Return the set of package IDs that should be pre-selected for a profile.
pub fn default_selection(catalog: &[Package], profile: Profile) -> HashSet<String> {
    let slug = profile.slug();
    catalog
        .iter()
        .filter(|p| p.profiles.iter().any(|s| s == slug))
        .map(|p| p.id.clone())
        .collect()
}

/// Human-readable display name for a category slug.
pub fn category_display_name(slug: &str) -> &str {
    match slug {
        "browsers" => "Browsers",
        "communication" => "Communication",
        "development" => "Development",
        "documents" => "Documents",
        "games" => "Games",
        "multimedia" => "Multimedia",
        "microsoft-tools" => "Microsoft Tools",
        "utilities" => "Utilities",
        "security-privacy" => "Security & Privacy",
        "design" => "Design",
        _ => slug,
    }
}

/// Ordered unique categories, preserving declaration order from the catalog.
pub fn categories(catalog: &[Package]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for pkg in catalog {
        if seen.insert(pkg.category.as_str()) {
            result.push(pkg.category.clone());
        }
    }
    result
}
