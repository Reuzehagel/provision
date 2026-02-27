use std::collections::HashMap;
use std::fs;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CatalogFile {
    packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    id: String,
    name: String,
    description: String,
    category: String,
    winget_id: Option<String>,
    #[serde(default)]
    profiles: Vec<String>,
    post_install: Option<String>,
    install_command: Option<String>,
}

/// Fixed category order matching the app's display order.
const CATEGORY_ORDER: &[&str] = &[
    "browsers",
    "communication",
    "development",
    "documents",
    "games",
    "microsoft-tools",
    "multimedia",
    "utilities",
    "security-privacy",
    "design",
];

fn category_display_name(slug: &str) -> &str {
    match slug {
        "browsers" => "Browsers",
        "communication" => "Communication",
        "development" => "Development",
        "documents" => "Documents",
        "games" => "Games",
        "microsoft-tools" => "Microsoft Tools",
        "multimedia" => "Multimedia",
        "utilities" => "Utilities",
        "security-privacy" => "Security & Privacy",
        "design" => "Design",
        _ => slug,
    }
}

fn main() {
    let path = "packages.toml";
    let raw = fs::read_to_string(path).expect("failed to read packages.toml");
    let file: CatalogFile = toml::from_str(&raw).expect("failed to parse packages.toml");

    // Group packages by category.
    let mut by_category: HashMap<String, Vec<&Package>> = HashMap::new();
    for pkg in &file.packages {
        by_category
            .entry(pkg.category.clone())
            .or_default()
            .push(pkg);
    }

    // Sort each category alphabetically by name (case-insensitive).
    for packages in by_category.values_mut() {
        packages.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    // Warn about unknown categories.
    for cat in by_category.keys() {
        if !CATEGORY_ORDER.contains(&cat.as_str()) {
            eprintln!("warning: unknown category '{cat}' — appending at the end");
        }
    }

    // Build ordered list of categories: known order first, then any unknown.
    let mut ordered_categories: Vec<&str> = CATEGORY_ORDER
        .iter()
        .filter(|c| by_category.contains_key(**c))
        .copied()
        .collect();
    for cat in by_category.keys() {
        if !CATEGORY_ORDER.contains(&cat.as_str()) {
            ordered_categories.push(cat.as_str());
        }
    }

    // Generate the output.
    let mut out = String::from("# Provision — Package Catalog\n");

    for (i, &cat) in ordered_categories.iter().enumerate() {
        let display = category_display_name(cat);
        let bar = "─".repeat(57 - display.len().min(55));

        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format!("\n# ── {display} {bar}\n"));

        let packages = &by_category[cat];
        for pkg in packages {
            out.push_str("\n[[packages]]\n");
            out.push_str(&format!("id = {:?}\n", pkg.id));
            out.push_str(&format!("name = {:?}\n", pkg.name));
            out.push_str(&format!("description = {:?}\n", pkg.description));
            out.push_str(&format!("category = {:?}\n", pkg.category));
            if let Some(ref winget_id) = pkg.winget_id {
                out.push_str(&format!("winget_id = {:?}\n", winget_id));
            }
            out.push_str(&format!("profiles = {:?}\n", pkg.profiles));
            if let Some(ref post_install) = pkg.post_install {
                out.push_str(&format!("post_install = {:?}\n", post_install));
            }
            if let Some(ref install_command) = pkg.install_command {
                out.push_str(&format!("install_command = {:?}\n", install_command));
            }
        }
    }

    fs::write(path, &out).expect("failed to write packages.toml");

    let total: usize = by_category.values().map(|v| v.len()).sum();
    println!(
        "Sorted {total} packages across {} categories",
        ordered_categories.len()
    );
}
