//! Helpers of the UI shell that need no controller: link check, file names and
//! the blocking Discord lookups.

use decoy_core::discord_api::{self, AppInfo, Asset};
use serde::Serialize;

pub const MSG_BAD_URL: &str = "Only http and https links can be opened";
const ICON_SIZE: u32 = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppLookup {
    pub id: String,
    pub name: String,
    pub icon_url: Option<String>,
}

impl From<AppInfo> for AppLookup {
    fn from(a: AppInfo) -> Self {
        let icon_url = a.icon.as_deref().filter(|i| !i.is_empty()).map(|icon| {
            format!(
                "https://cdn.discordapp.com/app-icons/{}/{icon}.png?size={ICON_SIZE}",
                a.id
            )
        });
        AppLookup {
            id: a.id,
            name: a.name,
            icon_url,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssetInfo {
    pub id: String,
    pub name: String,
    pub url: String,
}

impl AssetInfo {
    fn new(app_id: &str, a: Asset) -> Self {
        AssetInfo {
            url: format!(
                "https://cdn.discordapp.com/app-assets/{app_id}/{}.png",
                a.id
            ),
            id: a.id,
            name: a.name,
        }
    }
}

/// Blocking: name and icon of an application.
pub fn lookup_app(app_id: &str) -> Result<AppLookup, String> {
    discord_api::fetch_app(app_id)
        .map(AppLookup::from)
        .map_err(|e| e.to_string())
}

/// Blocking: uploaded Rich Presence assets of an application.
pub fn list_assets(app_id: &str) -> Result<Vec<AssetInfo>, String> {
    let id = app_id.trim();
    let assets = discord_api::fetch_assets(id).map_err(|e| e.to_string())?;
    Ok(assets.into_iter().map(|a| AssetInfo::new(id, a)).collect())
}

/// Only plain web links may be handed to the OS.
pub fn check_external_url(url: &str) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"));
    match rest {
        Some(r) if !r.is_empty() && !url.chars().any(|c| c.is_control() || c.is_whitespace()) => {
            Ok(())
        }
        _ => Err(MSG_BAD_URL.into()),
    }
}

/// Default file name for an exported profile.
pub fn export_file_name(profile_name: &str) -> String {
    let cleaned: String = profile_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.');
    let base = if cleaned.is_empty() {
        "profile"
    } else {
        cleaned
    };
    format!("{base}.decoy.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_url_check() {
        assert!(check_external_url("https://example.com/a?b=c").is_ok());
        assert!(check_external_url("HTTP://example.com").is_ok());
        for bad in [
            "",
            "https://",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "ftp://example.com",
            "example.com",
            "https://exa mple.com",
            "https://example.com\n",
            "  https://example.com",
            "mailto:a@b.c",
        ] {
            assert_eq!(check_external_url(bad).unwrap_err(), MSG_BAD_URL, "{bad:?}");
        }
    }

    #[test]
    fn file_names() {
        assert_eq!(export_file_name("My Game"), "My Game.decoy.json");
        assert_eq!(export_file_name("a/b\\c:d"), "a_b_c_d.decoy.json");
        assert_eq!(export_file_name("  "), "profile.decoy.json");
        assert_eq!(export_file_name("..."), "profile.decoy.json");
        assert_eq!(export_file_name("Café"), "Café.decoy.json");
    }

    #[test]
    fn app_lookup_urls() {
        let a = AppLookup::from(AppInfo {
            id: "5".into(),
            name: "N".into(),
            icon: Some("h".into()),
        });
        assert_eq!(
            a.icon_url.as_deref(),
            Some("https://cdn.discordapp.com/app-icons/5/h.png?size=64")
        );
        let a = AppLookup::from(AppInfo {
            id: "5".into(),
            name: "N".into(),
            icon: Some(String::new()),
        });
        assert_eq!(a.icon_url, None);
        let s = AssetInfo::new(
            "5",
            Asset {
                id: "9".into(),
                name: "logo".into(),
                kind: 1,
            },
        );
        assert_eq!(s.url, "https://cdn.discordapp.com/app-assets/5/9.png");
        // Bad ids fail before any request.
        assert!(lookup_app("../x").is_err());
        assert!(list_assets("").is_err());
    }
}
