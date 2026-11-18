//! Discord's public list of detectable games, with a local cache and search.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::util::now_secs;

const DETECTABLE_URL: &str = "https://discord.com/api/v10/applications/detectable";
const CACHE_FILE: &str = "games.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

impl Game {
    pub fn icon_url(&self, size: u32) -> Option<String> {
        let hash = self.icon_hash.as_deref().filter(|h| !h.is_empty())?;
        Some(format!(
            "https://cdn.discordapp.com/app-icons/{}/{hash}.png?size={size}",
            self.id
        ))
    }
}

/// On-disk form of the cache.
#[derive(Serialize, Deserialize)]
struct Cache {
    fetched_at: u64,
    games: Vec<Game>,
}

/// Lowercased search keys, computed once per entry.
#[derive(Debug, Clone)]
struct Keys {
    name: String,
    name_len: usize,
    aliases: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GameIndex {
    pub fetched_at: u64,
    games: Vec<Game>,
    keys: Vec<Keys>,
}

impl GameIndex {
    pub fn new(games: Vec<Game>, fetched_at: u64) -> GameIndex {
        let keys = games
            .iter()
            .map(|g| {
                let name = g.name.to_lowercase();
                Keys {
                    name_len: name.chars().count(),
                    name,
                    aliases: g.aliases.iter().map(|a| a.to_lowercase()).collect(),
                }
            })
            .collect();
        GameIndex {
            fetched_at,
            games,
            keys,
        }
    }

    /// Parse Discord's `detectable` array, keeping only id, name, icon and aliases.
    pub fn parse_detectable(json: &[u8]) -> Result<GameIndex> {
        let games: Vec<Game> = serde_json::from_slice(json)?;
        Ok(GameIndex::new(games, now_secs()))
    }

    /// Download the list (about 13 MB of JSON).
    pub fn fetch() -> Result<GameIndex> {
        let body = crate::http::get_bytes(DETECTABLE_URL, Duration::from_secs(30), 128 << 20)?;
        Self::parse_detectable(&body)
    }

    pub fn load(cache_dir: &Path) -> Result<GameIndex> {
        let bytes = std::fs::read(cache_dir.join(CACHE_FILE))?;
        let c: Cache = serde_json::from_slice(&bytes)?;
        Ok(GameIndex::new(c.games, c.fetched_at))
    }

    pub fn save(&self, cache_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(cache_dir)?;
        let data = serde_json::to_vec(&Cache {
            fetched_at: self.fetched_at,
            games: self.games.clone(),
        })?;
        let tmp = cache_dir.join(format!("{CACHE_FILE}.tmp"));
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, cache_dir.join(CACHE_FILE))?;
        Ok(())
    }

    pub fn is_stale(&self, max_age_secs: u64) -> bool {
        now_secs().saturating_sub(self.fetched_at) > max_age_secs
    }

    pub fn len(&self) -> usize {
        self.games.len()
    }

    pub fn is_empty(&self) -> bool {
        self.games.is_empty()
    }

    pub fn games(&self) -> &[Game] {
        &self.games
    }

    /// Case-insensitive search. Ranking: exact name, name prefix, word prefix,
    /// substring, alias; ties by shorter name, then alphabetically.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Game> {
        let q = query.trim().to_lowercase();
        if q.is_empty() || limit == 0 {
            return Vec::new();
        }
        let mut hits: Vec<(u8, usize, usize)> = Vec::new();
        for (i, k) in self.keys.iter().enumerate() {
            if let Some(tier) = tier(k, &q) {
                hits.push((tier, k.name_len, i));
            }
        }
        hits.sort_by(|a, b| {
            (a.0, a.1)
                .cmp(&(b.0, b.1))
                .then_with(|| self.keys[a.2].name.cmp(&self.keys[b.2].name))
                .then_with(|| self.games[a.2].name.cmp(&self.games[b.2].name))
        });
        hits.into_iter()
            .take(limit)
            .map(|(_, _, i)| &self.games[i])
            .collect()
    }
}

fn tier(k: &Keys, q: &str) -> Option<u8> {
    if k.name == q {
        return Some(0);
    }
    if k.name.starts_with(q) {
        return Some(1);
    }
    let mut substring = false;
    for (idx, _) in k.name.match_indices(q) {
        substring = true;
        if k.name[..idx]
            .chars()
            .next_back()
            .is_some_and(|c| !c.is_alphanumeric())
        {
            return Some(2);
        }
    }
    if substring {
        return Some(3);
    }
    k.aliases.iter().any(|a| a.contains(q)).then_some(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: &str, name: &str, aliases: &[&str]) -> Game {
        Game {
            id: id.into(),
            name: name.into(),
            icon_hash: Some(format!("h{id}")),
            aliases: aliases.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn fixture() -> GameIndex {
        GameIndex::new(
            vec![
                game("1", "Minecraft Dungeons", &[]),
                game("2", "Mine", &[]),
                game("3", "Craft Mine Rush", &[]),
                game("4", "Terraria", &["mine and dig"]),
                game("5", "Minecraft", &["mc"]),
                game("6", "Determined", &[]),
                game("7", "Mine Craft", &[]),
                game("8", "Minefield", &[]),
            ],
            100,
        )
    }

    fn names(idx: &GameIndex, q: &str, limit: usize) -> Vec<String> {
        idx.search(q, limit)
            .iter()
            .map(|g| g.name.clone())
            .collect()
    }

    #[test]
    fn ranking_tiers_and_ties() {
        let idx = fixture();
        assert_eq!(
            names(&idx, "MINE", 20),
            vec![
                "Mine",               // exact
                "Minecraft",          // prefix, 9 chars, alphabetical first
                "Minefield",          // prefix, 9 chars
                "Mine Craft",         // prefix, 10 chars
                "Minecraft Dungeons", // prefix, longest
                "Craft Mine Rush",    // word prefix
                "Determined",         // substring
                "Terraria",           // alias only
            ]
        );
    }

    #[test]
    fn alias_and_limit_and_empty() {
        let idx = fixture();
        assert_eq!(names(&idx, "mc", 5), vec!["Minecraft"]);
        assert_eq!(names(&idx, "mine", 2).len(), 2);
        assert!(idx.search("", 5).is_empty());
        assert!(idx.search("   ", 5).is_empty());
        assert!(idx.search("zzz", 5).is_empty());
        assert!(idx.search("mine", 0).is_empty());
    }

    #[test]
    fn icon_url() {
        let g = game("5", "Minecraft", &[]);
        assert_eq!(
            g.icon_url(64).unwrap(),
            "https://cdn.discordapp.com/app-icons/5/h5.png?size=64"
        );
        let mut g = g;
        g.icon_hash = None;
        assert_eq!(g.icon_url(64), None);
    }

    #[test]
    fn parses_detectable_and_drops_extra_keys() {
        let json = br#"[
          {"id":"1","name":"A Game","icon_hash":"abc","aliases":["ag"],"executables":[{"name":"a.exe"}],"hook":true},
          {"id":"2","name":"B","icon_hash":null}
        ]"#;
        let idx = GameIndex::parse_detectable(json).unwrap();
        assert_eq!(idx.len(), 2);
        assert_eq!(idx.games()[0].aliases, vec!["ag"]);
        assert_eq!(idx.games()[1].icon_hash, None);
        assert!(!idx.is_stale(60));
        assert!(GameIndex::parse_detectable(b"{}").is_err());
    }

    #[test]
    fn cache_roundtrip_and_staleness() {
        let dir = tempfile::tempdir().unwrap();
        let idx = fixture();
        idx.save(&dir.path().join("nested")).unwrap();
        let back = GameIndex::load(&dir.path().join("nested")).unwrap();
        assert_eq!(back.len(), idx.len());
        assert_eq!(back.fetched_at, 100);
        assert!(back.is_stale(3600));
        assert_eq!(names(&back, "mc", 3), vec!["Minecraft"]);
        assert!(GameIndex::load(dir.path()).is_err());
    }
}
