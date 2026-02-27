use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::profile::Profile;

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Serialize, Deserialize)]
struct SelectionFile {
    selected: Vec<String>,
}

/// Show a Save dialog and write the selection to a TOML file.
pub async fn export_selection(selected: HashSet<String>) -> Result<(), String> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Export Selection")
        .add_filter("TOML files", &["toml"])
        .set_file_name("provision-selection.toml")
        .save_file()
        .await;

    let Some(handle) = handle else {
        return Err(String::new());
    };

    let mut ids: Vec<String> = selected.into_iter().collect();
    ids.sort();

    let file = SelectionFile { selected: ids };
    let content = toml::to_string_pretty(&file).map_err(|e| format!("Failed to serialize: {e}"))?;

    tokio::fs::write(handle.path(), content)
        .await
        .map_err(|e| format!("Failed to write file: {e}"))
}

/// Show an Open dialog, read the file, and return valid package IDs.
pub async fn import_selection(valid_ids: HashSet<String>) -> Result<HashSet<String>, String> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Import Selection")
        .add_filter("TOML files", &["toml"])
        .pick_file()
        .await;

    let Some(handle) = handle else {
        return Err(String::new());
    };

    let content = tokio::fs::read_to_string(handle.path())
        .await
        .map_err(|e| format!("Failed to read file: {e}"))?;

    let file: SelectionFile = toml::from_str(&content).map_err(|e| format!("Invalid TOML: {e}"))?;

    let imported: HashSet<String> = file
        .selected
        .into_iter()
        .filter(|id| valid_ids.contains(id))
        .collect();

    if imported.is_empty() {
        return Err("No recognized packages in file".to_string());
    }

    Ok(imported)
}
