//! User-facing presence model and its conversion to a Discord activity.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    #[default]
    Playing,
    Listening,
    Watching,
    Competing,
}

impl ActivityKind {
    fn code(self) -> u8 {
        match self {
            Self::Playing => 0,
            Self::Listening => 2,
            Self::Watching => 3,
            Self::Competing => 5,
        }
    }
}

/// Which text the member list shows next to the user name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusDisplay {
    #[default]
    Name,
    State,
    Details,
}

impl StatusDisplay {
    fn code(self) -> u8 {
        match self {
            Self::Name => 0,
            Self::State => 1,
            Self::Details => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Party {
    pub current: u32,
    pub max: u32,
}

impl Default for Party {
    fn default() -> Self {
        Self { current: 1, max: 4 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TimestampMode {
    #[default]
    None,
    SinceConnect,
    SinceUpdate,
    LocalTime,
    Custom {
        #[serde(default)]
        start_ms: Option<i64>,
        #[serde(default)]
        end_ms: Option<i64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Image {
    /// Asset key uploaded to the application, or an http(s) URL.
    pub source: String,
    pub text: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Button {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Presence {
    pub app_id: String,
    pub kind: ActivityKind,
    pub name: Option<String>,
    pub details: Option<String>,
    pub details_url: Option<String>,
    pub state: Option<String>,
    pub state_url: Option<String>,
    pub status_display: StatusDisplay,
    pub party: Option<Party>,
    pub timestamps: TimestampMode,
    pub large_image: Option<Image>,
    pub small_image: Option<Image>,
    pub buttons: Vec<Button>,
}

/// Field a validation issue points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    AppId,
    Name,
    Details,
    DetailsUrl,
    State,
    StateUrl,
    Party,
    Timestamps,
    LargeImage,
    LargeImageText,
    LargeImageUrl,
    SmallImage,
    SmallImageText,
    SmallImageUrl,
    Buttons,
    ButtonLabel(usize),
    ButtonUrl(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub field: Field,
    pub message: String,
}

/// Clock values needed to turn timestamp modes into absolute times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeContext {
    pub now_ms: i64,
    pub connected_at_ms: i64,
    pub utc_offset_secs: i32,
}

/// The moment (Unix ms) the local day containing `now_ms` began.
pub fn local_day_start_ms(now_ms: i64, utc_offset_secs: i32) -> i64 {
    let offset_ms = i64::from(utc_offset_secs) * 1000;
    let local = now_ms + offset_ms;
    local - local.rem_euclid(DAY_MS) - offset_ms
}

/// Length the way Discord measures it. Its client checks the limits in
/// JavaScript, where a string's length is its UTF-16 size, so an emoji counts
/// as two.
fn len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Trimmed text, or `None` when empty or whitespace-only.
fn clean(s: &Option<String>) -> Option<&str> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

fn is_url(s: &str) -> bool {
    s.starts_with("https://") || s.starts_with("http://")
}

const DAY_MS: i64 = 86_400_000;
const PARTY_ID: &str = "decoy-party";

impl Presence {
    /// Check every rule Discord enforces; an empty list means the presence is sendable.
    pub fn validate(&self) -> Vec<Issue> {
        let mut out = Vec::new();
        let mut add = |field: Field, message: &str| {
            out.push(Issue {
                field,
                message: message.to_string(),
            })
        };

        let id = self.app_id.trim();
        if !(17..=20).contains(&id.len()) || !id.bytes().all(|b| b.is_ascii_digit()) {
            add(Field::AppId, "Application ID must be 17 to 20 digits");
        }

        for (field, label, value) in [
            (Field::Name, "Name", &self.name),
            (Field::Details, "Details", &self.details),
            (Field::State, "State", &self.state),
        ] {
            if let Some(v) = clean(value) {
                if !(2..=128).contains(&len(v)) {
                    add(field, &format!("{label} must be 2 to 128 characters"));
                }
            }
        }
        for (field, label, value) in [
            (Field::DetailsUrl, "Details link", &self.details_url),
            (Field::StateUrl, "State link", &self.state_url),
        ] {
            if let Some(v) = clean(value) {
                check_url(&mut add, field, label, v);
            }
        }

        if let Some(p) = &self.party {
            if p.current < 1 {
                add(Field::Party, "Party size must be at least 1");
            } else if p.max < p.current {
                add(
                    Field::Party,
                    "Party maximum must not be smaller than the current size",
                );
            }
        }

        if let TimestampMode::Custom {
            start_ms: Some(s),
            end_ms: Some(e),
        } = &self.timestamps
        {
            if e <= s {
                add(Field::Timestamps, "End time must be after the start time");
            }
        }

        for (img, source, text, url, name) in [
            (
                &self.large_image,
                Field::LargeImage,
                Field::LargeImageText,
                Field::LargeImageUrl,
                "Large image",
            ),
            (
                &self.small_image,
                Field::SmallImage,
                Field::SmallImageText,
                Field::SmallImageUrl,
                "Small image",
            ),
        ] {
            let Some(img) = img else { continue };
            let src = img.source.trim();
            if src.is_empty() {
                continue;
            }
            if len(src) > 256 {
                add(
                    source,
                    &format!("{name} source must be 256 characters or fewer"),
                );
            } else if src.contains("://") && !is_url(src) {
                add(
                    source,
                    &format!("{name} URL must start with https:// or http://"),
                );
            }
            if let Some(t) = clean(&img.text) {
                if !(2..=128).contains(&len(t)) {
                    add(text, &format!("{name} text must be 2 to 128 characters"));
                }
            }
            if let Some(u) = clean(&img.url) {
                check_url(&mut add, url, &format!("{name} link"), u);
            }
        }

        if self.buttons.len() > 2 {
            add(Field::Buttons, "Use at most 2 buttons");
        }
        for (i, b) in self.buttons.iter().enumerate() {
            let label = b.label.trim();
            if label.is_empty() || len(label) > 32 {
                add(
                    Field::ButtonLabel(i),
                    "Button label must be 1 to 32 characters",
                );
            }
            let url = b.url.trim();
            if url.is_empty() {
                add(Field::ButtonUrl(i), "Button link is required");
            } else {
                check_url(&mut add, Field::ButtonUrl(i), "Button link", url);
            }
        }
        out
    }

    /// Build the activity object for `SET_ACTIVITY`. Absent values are omitted.
    pub fn to_activity(&self, ctx: &TimeContext) -> Value {
        let mut a = Map::new();
        a.insert("type".into(), self.kind.code().into());
        let mut put = |key: &str, v: &Option<String>| {
            if let Some(s) = clean(v) {
                a.insert(key.into(), s.into());
            }
        };
        put("name", &self.name);
        put("details", &self.details);
        put("details_url", &self.details_url);
        put("state", &self.state);
        put("state_url", &self.state_url);
        if self.status_display != StatusDisplay::Name {
            a.insert(
                "status_display_type".into(),
                self.status_display.code().into(),
            );
        }

        let (start, end) = match &self.timestamps {
            TimestampMode::None => (None, None),
            TimestampMode::SinceConnect => (Some(ctx.connected_at_ms), None),
            TimestampMode::SinceUpdate => (Some(ctx.now_ms), None),
            TimestampMode::LocalTime => {
                let offset_ms = i64::from(ctx.utc_offset_secs) * 1000;
                let local = ctx.now_ms + offset_ms;
                (Some(local - local.rem_euclid(DAY_MS) - offset_ms), None)
            }
            TimestampMode::Custom { start_ms, end_ms } => (*start_ms, *end_ms),
        };
        let mut ts = Map::new();
        if let Some(s) = start {
            ts.insert("start".into(), s.into());
        }
        if let Some(e) = end {
            ts.insert("end".into(), e.into());
        }
        if !ts.is_empty() {
            a.insert("timestamps".into(), Value::Object(ts));
        }

        let mut assets = Map::new();
        for (prefix, img) in [("large", &self.large_image), ("small", &self.small_image)] {
            let Some(img) = img else { continue };
            let src = img.source.trim();
            if src.is_empty() {
                continue;
            }
            assets.insert(format!("{prefix}_image"), src.into());
            if let Some(t) = clean(&img.text) {
                assets.insert(format!("{prefix}_text"), t.into());
            }
            if let Some(u) = clean(&img.url) {
                assets.insert(format!("{prefix}_url"), u.into());
            }
        }
        if !assets.is_empty() {
            a.insert("assets".into(), Value::Object(assets));
        }

        if let Some(p) = &self.party {
            a.insert(
                "party".into(),
                json!({ "id": PARTY_ID, "size": [p.current, p.max] }),
            );
        }

        let buttons: Vec<Value> = self
            .buttons
            .iter()
            .filter(|b| !b.label.trim().is_empty() && !b.url.trim().is_empty())
            .map(|b| json!({ "label": b.label.trim(), "url": b.url.trim() }))
            .collect();
        if !buttons.is_empty() {
            a.insert("buttons".into(), buttons.into());
        }
        a.insert("instance".into(), false.into());
        Value::Object(a)
    }
}

fn check_url(add: &mut impl FnMut(Field, &str), field: Field, label: &str, url: &str) {
    if !is_url(url) {
        add(
            field,
            &format!("{label} must start with https:// or http://"),
        );
    } else if len(url) > 512 {
        add(field, &format!("{label} must be 512 characters or fewer"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "123456789012345678";

    fn base() -> Presence {
        Presence {
            app_id: ID.into(),
            ..Default::default()
        }
    }

    fn ctx() -> TimeContext {
        TimeContext {
            now_ms: 1_700_000_000_000, // 2023-11-14 22:13:20 UTC
            connected_at_ms: 1_699_999_000_000,
            utc_offset_secs: 3600,
        }
    }

    fn fields(p: &Presence) -> Vec<Field> {
        p.validate().into_iter().map(|i| i.field).collect()
    }

    #[test]
    fn minimal_presence_is_valid() {
        assert!(base().validate().is_empty());
    }

    #[test]
    fn app_id_rules() {
        for bad in [
            "",
            "1234",
            "1234567890123456",
            "123456789012345678901",
            "12345678901234567x",
        ] {
            let p = Presence {
                app_id: bad.into(),
                ..Default::default()
            };
            assert_eq!(fields(&p), vec![Field::AppId], "{bad}");
        }
        for ok in ["12345678901234567", "12345678901234567890"] {
            let p = Presence {
                app_id: ok.into(),
                ..Default::default()
            };
            assert!(p.validate().is_empty(), "{ok}");
        }
    }

    #[test]
    fn length_is_counted_the_way_discord_counts() {
        let mut p = base();
        // One emoji is two UTF-16 units: long enough alone, and 65 of them are too many.
        p.details = Some("🎮".into());
        p.state = Some("🎮".repeat(64));
        assert!(p.validate().is_empty());
        p.state = Some("🎮".repeat(65));
        assert_eq!(fields(&p), vec![Field::State]);
    }

    #[test]
    fn text_length_rules() {
        let mut p = base();
        p.details = Some("x".into());
        p.state = Some("y".repeat(129));
        p.name = Some("é".repeat(128)); // counted in UTF-16 units, not bytes
        let issues = p.validate();
        assert_eq!(
            issues.iter().map(|i| i.field).collect::<Vec<_>>(),
            vec![Field::Details, Field::State]
        );
        assert_eq!(issues[0].message, "Details must be 2 to 128 characters");
        // whitespace-only counts as absent
        p.details = Some("   ".into());
        p.state = None;
        assert!(p.validate().is_empty());
    }

    #[test]
    fn url_rules() {
        let mut p = base();
        p.details_url = Some("ftp://x".into());
        p.state_url = Some(format!("https://{}", "a".repeat(510)));
        assert_eq!(fields(&p), vec![Field::DetailsUrl, Field::StateUrl]);
        p.details_url = Some("http://ok".into());
        p.state_url = Some("https://ok".into());
        assert!(p.validate().is_empty());
    }

    #[test]
    fn image_rules() {
        let mut p = base();
        p.large_image = Some(Image {
            source: "a".repeat(257),
            text: Some("x".into()),
            url: Some("nope".into()),
        });
        p.small_image = Some(Image {
            source: "ftp://host/x.png".into(),
            ..Default::default()
        });
        assert_eq!(
            fields(&p),
            vec![
                Field::LargeImage,
                Field::LargeImageText,
                Field::LargeImageUrl,
                Field::SmallImage
            ]
        );
        p.large_image = Some(Image {
            source: "https://cdn.example/x.png".into(),
            text: Some("Hello".into()),
            url: None,
        });
        p.small_image = Some(Image {
            source: "asset_key".into(),
            ..Default::default()
        });
        assert!(p.validate().is_empty());
    }

    #[test]
    fn button_rules() {
        let mut p = base();
        p.buttons = vec![
            Button {
                label: "".into(),
                url: "https://a".into(),
            },
            Button {
                label: "l".repeat(33),
                url: "a.com".into(),
            },
            Button {
                label: "ok".into(),
                url: "https://a".into(),
            },
        ];
        assert_eq!(
            fields(&p),
            vec![
                Field::Buttons,
                Field::ButtonLabel(0),
                Field::ButtonLabel(1),
                Field::ButtonUrl(1)
            ]
        );
    }

    #[test]
    fn party_rules() {
        let mut p = base();
        p.party = Some(Party { current: 0, max: 4 });
        assert_eq!(fields(&p), vec![Field::Party]);
        p.party = Some(Party { current: 5, max: 4 });
        assert_eq!(fields(&p), vec![Field::Party]);
        p.party = Some(Party { current: 4, max: 4 });
        assert!(p.validate().is_empty());
    }

    #[test]
    fn custom_timestamp_rules() {
        let mut p = base();
        p.timestamps = TimestampMode::Custom {
            start_ms: Some(10),
            end_ms: Some(10),
        };
        assert_eq!(fields(&p), vec![Field::Timestamps]);
        p.timestamps = TimestampMode::Custom {
            start_ms: Some(10),
            end_ms: Some(11),
        };
        assert!(p.validate().is_empty());
        p.timestamps = TimestampMode::Custom {
            start_ms: None,
            end_ms: Some(5),
        };
        assert!(p.validate().is_empty());
    }

    #[test]
    fn minimal_activity_omits_everything_absent() {
        let a = base().to_activity(&ctx());
        assert_eq!(a, json!({ "type": 0, "instance": false }));
    }

    #[test]
    fn full_activity() {
        let p = Presence {
            app_id: ID.into(),
            kind: ActivityKind::Listening,
            name: Some(" Decoy ".into()),
            details: Some("Details".into()),
            details_url: Some("https://d".into()),
            state: Some("State".into()),
            state_url: Some("https://s".into()),
            status_display: StatusDisplay::Details,
            party: Some(Party { current: 2, max: 5 }),
            timestamps: TimestampMode::SinceUpdate,
            large_image: Some(Image {
                source: "big".into(),
                text: Some("Big".into()),
                url: Some("https://b".into()),
            }),
            small_image: Some(Image {
                source: "small".into(),
                text: Some("  ".into()),
                url: None,
            }),
            buttons: vec![Button {
                label: "Go".into(),
                url: "https://go".into(),
            }],
        };
        assert_eq!(
            p.to_activity(&ctx()),
            json!({
                "type": 2,
                "name": "Decoy",
                "details": "Details",
                "details_url": "https://d",
                "state": "State",
                "state_url": "https://s",
                "status_display_type": 2,
                "timestamps": { "start": 1_700_000_000_000i64 },
                "assets": {
                    "large_image": "big", "large_text": "Big", "large_url": "https://b",
                    "small_image": "small"
                },
                "party": { "id": "decoy-party", "size": [2, 5] },
                "buttons": [{ "label": "Go", "url": "https://go" }],
                "instance": false
            })
        );
    }

    #[test]
    fn timestamp_modes() {
        let ts = |m: TimestampMode| {
            let mut p = base();
            p.timestamps = m;
            p.to_activity(&ctx())["timestamps"].clone()
        };
        assert_eq!(ts(TimestampMode::None), Value::Null);
        assert_eq!(
            ts(TimestampMode::SinceConnect),
            json!({ "start": 1_699_999_000_000i64 })
        );
        assert_eq!(
            ts(TimestampMode::SinceUpdate),
            json!({ "start": 1_700_000_000_000i64 })
        );
        // now = 22:13:20 UTC, offset +1h -> local 23:13:20 on 2023-11-14;
        // local midnight = 2023-11-14 00:00 +01:00 = 2023-11-13 23:00 UTC.
        assert_eq!(
            ts(TimestampMode::LocalTime),
            json!({ "start": 1_699_916_400_000i64 })
        );
        assert_eq!(
            ts(TimestampMode::Custom {
                start_ms: Some(1),
                end_ms: Some(2)
            }),
            json!({ "start": 1, "end": 2 })
        );
        assert_eq!(
            ts(TimestampMode::Custom {
                start_ms: None,
                end_ms: Some(2)
            }),
            json!({ "end": 2 })
        );
        assert_eq!(
            ts(TimestampMode::Custom {
                start_ms: None,
                end_ms: None
            }),
            Value::Null
        );
    }

    #[test]
    fn local_time_with_negative_offset_crosses_midnight() {
        let mut p = base();
        p.timestamps = TimestampMode::LocalTime;
        // 02:00 UTC with offset -5h is 21:00 the previous local day.
        let c = TimeContext {
            now_ms: 1_699_927_200_000, // 02:00:00 UTC on 11-14
            connected_at_ms: 0,
            utc_offset_secs: -5 * 3600,
        };
        let start = p.to_activity(&c)["timestamps"]["start"].as_i64().unwrap();
        // local midnight of 11-13 is 05:00 UTC on 11-13
        assert_eq!(start, 1_699_851_600_000);
    }

    #[test]
    fn old_profiles_still_load() {
        let p: Presence = serde_json::from_str(r#"{"app_id":"123456789012345678"}"#).unwrap();
        assert_eq!(p.kind, ActivityKind::Playing);
        assert_eq!(p.timestamps, TimestampMode::None);
        let p: Presence = serde_json::from_str(
            r#"{"app_id":"1","timestamps":{"mode":"custom","start_ms":5},"future_field":1}"#,
        )
        .unwrap();
        assert_eq!(
            p.timestamps,
            TimestampMode::Custom {
                start_ms: Some(5),
                end_ms: None
            }
        );
    }
}
