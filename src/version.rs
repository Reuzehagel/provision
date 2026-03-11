use serde::Deserialize;

use crate::catalog::{CACHE_MAX_AGE, FETCH_TIMEOUT, dirs_cache_dir};

#[derive(Debug, Clone)]
pub struct LatestRelease {
    pub version: String,
    pub html_url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
}

#[derive(serde::Serialize, Deserialize)]
struct CachedRelease {
    version: String,
    html_url: String,
    checked_at: u64,
}

const API_URL: &str = "https://api.github.com/repos/Reuzehagel/provision/releases/latest";
const CACHE_FILE: &str = "latest_version.json";

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub async fn check_latest_release(force: bool) -> Result<LatestRelease, String> {
    let cache_dir = dirs_cache_dir()?;
    let cache_path = cache_dir.join(CACHE_FILE);
    let now = now_secs();

    // Check cache unless forced
    if !force
        && let Ok(data) = tokio::fs::read_to_string(&cache_path).await
        && let Ok(cached) = serde_json::from_str::<CachedRelease>(&data)
        && now.saturating_sub(cached.checked_at) < CACHE_MAX_AGE.as_secs()
    {
        return Ok(LatestRelease {
            version: cached.version,
            html_url: cached.html_url,
        });
    }

    // Fetch from GitHub API
    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let resp = client
        .get(API_URL)
        .header("User-Agent", "provision")
        .send()
        .await
        .map_err(|e| format!("fetch: {e}"))?
        .error_for_status()
        .map_err(|e| format!("fetch: {e}"))?;

    let body = resp.text().await.map_err(|e| format!("read body: {e}"))?;
    let release: GithubRelease =
        serde_json::from_str(&body).map_err(|e| format!("parse json: {e}"))?;

    let version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();

    // Write cache (best-effort)
    let cached = CachedRelease {
        version: version.clone(),
        html_url: release.html_url.clone(),
        checked_at: now,
    };
    if let Ok(json) = serde_json::to_string(&cached) {
        let _ = tokio::fs::create_dir_all(&cache_dir).await;
        let _ = tokio::fs::write(&cache_path, json).await;
    }

    Ok(LatestRelease {
        version,
        html_url: release.html_url,
    })
}

/// Compare semver strings: returns true if `latest` is newer than `current`.
pub fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> { s.split('.').filter_map(|p| p.parse().ok()).collect() };
    let l = parse(latest);
    let c = parse(current);
    l > c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
    }
}
