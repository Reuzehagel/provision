use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::profile::Profile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSource {
    Embedded,
    Cached,
    Remote,
}

impl CatalogSource {
    /// Human-readable suffix for the package count label.
    /// Returns `None` for `Cached` (no annotation needed).
    pub fn label_suffix(self) -> Option<&'static str> {
        match self {
            CatalogSource::Embedded => Some("built-in"),
            CatalogSource::Cached => None,
            CatalogSource::Remote => Some("updated"),
        }
    }
}

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
    /// Precomputed `name.to_lowercase()` for search filtering.
    #[serde(skip)]
    pub name_lower: String,
    /// Precomputed `description.to_lowercase()` for search filtering.
    #[serde(skip)]
    pub desc_lower: String,
    /// Precomputed `winget_id` lowercased, for installed-package lookups.
    #[serde(skip)]
    pub winget_id_lower: Option<String>,
}

impl Package {
    /// True if this package opens a browser URL instead of silently installing.
    pub fn is_browser_download(&self) -> bool {
        self.install_command
            .as_deref()
            .is_some_and(|c| c.starts_with("start http"))
    }
}

#[derive(Deserialize)]
struct CatalogFile {
    packages: Vec<Package>,
}

/// Populate precomputed lowercase fields after deserialization.
fn prepare_packages(packages: &mut [Package]) {
    for pkg in packages {
        pkg.name_lower = pkg.name.to_lowercase();
        pkg.desc_lower = pkg.description.to_lowercase();
        pkg.winget_id_lower = pkg.winget_id.as_ref().map(|id| id.to_lowercase());
    }
}

pub fn load_catalog() -> Vec<Package> {
    let raw = include_str!("../packages.toml");
    parse_catalog_toml(raw).expect("embedded packages.toml should be valid")
}

const REMOTE_URL: &str =
    "https://raw.githubusercontent.com/Reuzehagel/provision/main/packages.toml";
const CACHE_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);
const FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Try to load a fresh catalog from the local cache or GitHub.
///
/// Returns `Ok(packages)` on success, `Err(reason)` on any failure.
/// The caller should silently fall back to the embedded catalog on error.
pub async fn fetch_remote_catalog(dry_run: bool) -> Result<(Vec<Package>, CatalogSource), String> {
    if dry_run {
        return Err("skipped in dry-run mode".into());
    }

    let cache_dir = dirs_cache_dir()?;
    let cache_path = cache_dir.join("packages.toml");

    // If cache exists and is fresh, use it
    if let Ok(meta) = tokio::fs::metadata(&cache_path).await
        && let Ok(modified) = meta.modified()
        && modified.elapsed().unwrap_or(CACHE_MAX_AGE) < CACHE_MAX_AGE
    {
        let raw = tokio::fs::read_to_string(&cache_path)
            .await
            .map_err(|e| format!("cache read: {e}"))?;
        return parse_catalog_toml(&raw).map(|pkgs| (pkgs, CatalogSource::Cached));
    }

    // Fetch from remote
    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let resp = client
        .get(REMOTE_URL)
        .send()
        .await
        .map_err(|e| format!("fetch: {e}"))?
        .error_for_status()
        .map_err(|e| format!("fetch: {e}"))?;

    let raw = resp.text().await.map_err(|e| format!("read body: {e}"))?;

    let packages = parse_catalog_toml(&raw)?;

    // Write to cache (best-effort)
    let _ = tokio::fs::create_dir_all(&cache_dir).await;
    let _ = tokio::fs::write(&cache_path, &raw).await;

    Ok((packages, CatalogSource::Remote))
}

fn parse_catalog_toml(raw: &str) -> Result<Vec<Package>, String> {
    let file: CatalogFile = toml::from_str(raw).map_err(|e| format!("parse: {e}"))?;
    let mut packages = file.packages;
    prepare_packages(&mut packages);
    Ok(packages)
}

/// Resolve the app data directory: `%APPDATA%\provision`
pub(crate) fn dirs_cache_dir() -> Result<PathBuf, String> {
    let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA not set".to_string())?;
    Ok(PathBuf::from(appdata).join("provision"))
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
