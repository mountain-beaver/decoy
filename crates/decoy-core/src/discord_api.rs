//! Public, unauthenticated Discord lookups for an application id.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: String,
    /// 1 = large, 2 = small (Discord's asset type).
    #[serde(rename = "type", default)]
    pub kind: u8,
}

pub fn parse_app(json: &[u8]) -> Result<AppInfo> {
    Ok(serde_json::from_slice(json)?)
}

pub fn parse_assets(json: &[u8]) -> Result<Vec<Asset>> {
    Ok(serde_json::from_slice(json)?)
}

fn check_id(app_id: &str) -> Result<&str> {
    let id = app_id.trim();
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::Invalid("Application ID must be digits only".into()));
    }
    Ok(id)
}

/// Name and icon of an application (`/applications/{id}/rpc`).
pub fn fetch_app(app_id: &str) -> Result<AppInfo> {
    let id = check_id(app_id)?;
    let url = format!("https://discord.com/api/v10/applications/{id}/rpc");
    parse_app(&crate::http::get_bytes(
        &url,
        Duration::from_secs(15),
        1 << 20,
    )?)
}

/// Uploaded Rich Presence assets of an application.
pub fn fetch_assets(app_id: &str) -> Result<Vec<Asset>> {
    let id = check_id(app_id)?;
    let url = format!("https://discord.com/api/v10/oauth2/applications/{id}/assets");
    parse_assets(&crate::http::get_bytes(
        &url,
        Duration::from_secs(15),
        8 << 20,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_app() {
        let a = parse_app(
            br#"{"id":"123456789012345678","name":"My Game","icon":"abc","description":"x","hook":true}"#,
        )
        .unwrap();
        assert_eq!(
            a,
            AppInfo {
                id: "123456789012345678".into(),
                name: "My Game".into(),
                icon: Some("abc".into())
            }
        );
        let a = parse_app(br#"{"id":"1","name":"N","icon":null}"#).unwrap();
        assert_eq!(a.icon, None);
        assert!(parse_app(b"{}").is_err());
        assert!(parse_app(b"not json").is_err());
    }

    #[test]
    fn parses_assets() {
        let v = parse_assets(
            br#"[{"id":"9","type":1,"name":"logo"},{"id":"10","type":2,"name":"tiny"}]"#,
        )
        .unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].name, "logo");
        assert_eq!(v[0].kind, 1);
        assert_eq!(v[1].kind, 2);
        assert!(parse_assets(b"[]").unwrap().is_empty());
    }

    #[test]
    fn rejects_non_numeric_ids_before_any_request() {
        assert!(matches!(fetch_app("../x"), Err(Error::Invalid(_))));
        assert!(matches!(fetch_assets(""), Err(Error::Invalid(_))));
    }
}
