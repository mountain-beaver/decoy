//! Profiles and settings persisted in one JSON file.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::presence::Presence;
use crate::util::{new_id, now_secs};

const FILE: &str = "profiles.json";
const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub presence: Presence,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub active_profile: Option<String>,
    pub connect_on_launch: bool,
    pub start_in_tray: bool,
    pub autostart: bool,
    /// Off by default, so the app looks for a newer release unless told not to.
    pub skip_update_check: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Data {
    version: u32,
    settings: Settings,
    profiles: Vec<Profile>,
}

impl Default for Data {
    fn default() -> Self {
        Data {
            version: VERSION,
            settings: Settings::default(),
            profiles: Vec::new(),
        }
    }
}

/// Thread-safe handle to the on-disk profile store.
pub struct Store {
    dir: PathBuf,
    data: Mutex<Data>,
}

impl Store {
    /// Open the store in the platform config directory.
    pub fn open_default() -> Result<Store> {
        let dirs = directories::ProjectDirs::from("dev", "decoy", "Decoy")
            .ok_or_else(|| Error::Invalid("no home directory found".into()))?;
        Store::open(dirs.config_dir())
    }

    /// Open (or start) a store in `dir`. A corrupt file is renamed aside, not overwritten.
    pub fn open(dir: &Path) -> Result<Store> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(FILE);
        let data = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Data>(&bytes) {
                Ok(d) => d,
                Err(_) => {
                    let aside = dir.join(format!("{FILE}.corrupt-{}", now_secs()));
                    std::fs::rename(&path, aside)?;
                    Data::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Data::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Store {
            dir: dir.to_path_buf(),
            data: Mutex::new(data),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Data> {
        self.data.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Write via a temp file and rename so a crash never leaves a half-written file.
    fn save(&self, data: &Data) -> Result<()> {
        let json = serde_json::to_vec_pretty(data)?;
        let tmp = self.dir.join(format!("{FILE}.tmp"));
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, self.dir.join(FILE))?;
        Ok(())
    }

    /// Apply `f`, persist, and roll back in memory if saving fails.
    fn mutate<T>(&self, f: impl FnOnce(&mut Data) -> Result<T>) -> Result<T> {
        let mut guard = self.lock();
        let backup = guard.clone();
        let out = f(&mut guard)?;
        if let Err(e) = self.save(&guard) {
            *guard = backup;
            return Err(e);
        }
        Ok(out)
    }

    pub fn profiles(&self) -> Vec<Profile> {
        self.lock().profiles.clone()
    }

    pub fn get(&self, id: &str) -> Option<Profile> {
        self.lock().profiles.iter().find(|p| p.id == id).cloned()
    }

    /// Insert a profile, or replace the one with the same id. An empty id gets a fresh one.
    pub fn upsert(&self, mut profile: Profile) -> Result<Profile> {
        if profile.id.is_empty() {
            profile.id = new_id();
        }
        self.mutate(|d| {
            match d.profiles.iter_mut().find(|p| p.id == profile.id) {
                Some(slot) => *slot = profile.clone(),
                None => d.profiles.push(profile.clone()),
            }
            Ok(profile)
        })
    }

    /// Delete a profile; returns whether it existed. Clears `active_profile` if it pointed there.
    pub fn remove(&self, id: &str) -> Result<bool> {
        self.mutate(|d| {
            let before = d.profiles.len();
            d.profiles.retain(|p| p.id != id);
            if d.settings.active_profile.as_deref() == Some(id) {
                d.settings.active_profile = None;
            }
            Ok(d.profiles.len() != before)
        })
    }

    pub fn duplicate(&self, id: &str) -> Result<Profile> {
        self.mutate(|d| {
            let src = d
                .profiles
                .iter()
                .find(|p| p.id == id)
                .ok_or(Error::NotFound)?;
            let copy = Profile {
                id: new_id(),
                name: format!("{} copy", src.name),
                presence: src.presence.clone(),
            };
            d.profiles.push(copy.clone());
            Ok(copy)
        })
    }

    pub fn settings(&self) -> Settings {
        self.lock().settings.clone()
    }

    pub fn set_settings(&self, settings: Settings) -> Result<()> {
        self.mutate(|d| {
            d.settings = settings;
            Ok(())
        })
    }

    /// One profile as pretty JSON.
    pub fn export_profile(&self, id: &str) -> Result<String> {
        let p = self.get(id).ok_or(Error::NotFound)?;
        Ok(serde_json::to_string_pretty(&p)?)
    }

    /// Add a profile from exported JSON under a fresh id.
    pub fn import_profile(&self, json: &str) -> Result<Profile> {
        let mut p: Profile = serde_json::from_str(json)
            .map_err(|e| Error::Invalid(format!("not a Decoy profile: {e}")))?;
        if p.name.trim().is_empty() && p.presence.app_id.trim().is_empty() {
            return Err(Error::Invalid("not a Decoy profile".into()));
        }
        p.id = new_id();
        self.upsert(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str) -> Profile {
        Profile {
            id: String::new(),
            name: name.into(),
            presence: Presence {
                app_id: "123456789012345678".into(),
                details: Some("Hello".into()),
                ..Default::default()
            },
        }
    }

    #[test]
    fn empty_store_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open(dir.path()).unwrap();
        assert!(s.profiles().is_empty());
        assert_eq!(s.settings(), Settings::default());
        assert!(!dir.path().join(FILE).exists());
    }

    #[test]
    fn roundtrip_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open(dir.path()).unwrap();
        let a = s.upsert(profile("A")).unwrap();
        assert!(!a.id.is_empty());
        let mut a2 = a.clone();
        a2.name = "A2".into();
        s.upsert(a2).unwrap();
        s.set_settings(Settings {
            active_profile: Some(a.id.clone()),
            connect_on_launch: true,
            ..Default::default()
        })
        .unwrap();

        let s2 = Store::open(dir.path()).unwrap();
        assert_eq!(s2.profiles().len(), 1);
        assert_eq!(s2.get(&a.id).unwrap().name, "A2");
        assert!(s2.settings().connect_on_launch);
        assert_eq!(s2.settings().active_profile, Some(a.id.clone()));

        assert!(s2.remove(&a.id).unwrap());
        assert!(!s2.remove(&a.id).unwrap());
        assert_eq!(s2.settings().active_profile, None);
        assert!(Store::open(dir.path()).unwrap().profiles().is_empty());
    }

    #[test]
    fn duplicate_makes_independent_copy() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open(dir.path()).unwrap();
        let a = s.upsert(profile("Game")).unwrap();
        let b = s.duplicate(&a.id).unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(b.name, "Game copy");
        assert_eq!(a.presence, b.presence);
        assert_eq!(s.profiles().len(), 2);
        assert!(matches!(s.duplicate("nope"), Err(Error::NotFound)));
    }

    #[test]
    fn export_import() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open(dir.path()).unwrap();
        let a = s.upsert(profile("Game")).unwrap();
        let json = s.export_profile(&a.id).unwrap();
        assert!(json.contains('\n'));
        let b = s.import_profile(&json).unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(b.name, "Game");
        assert_eq!(b.presence, a.presence);
        assert_eq!(s.profiles().len(), 2);
        assert!(matches!(s.export_profile("nope"), Err(Error::NotFound)));
        assert!(matches!(s.import_profile("[1]"), Err(Error::Invalid(_))));
        assert!(matches!(s.import_profile("{}"), Err(Error::Invalid(_))));
        assert_eq!(s.profiles().len(), 2);
    }

    #[test]
    fn save_is_atomic_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open(dir.path()).unwrap();
        s.upsert(profile("A")).unwrap();
        assert!(dir.path().join(FILE).exists());
        assert!(!dir.path().join("profiles.json.tmp").exists());
        let v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.path().join(FILE)).unwrap()).unwrap();
        assert_eq!(v["version"], 1);
        assert_eq!(v["profiles"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn corrupt_file_is_moved_aside_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE), b"{ not json").unwrap();
        let s = Store::open(dir.path()).unwrap();
        assert!(s.profiles().is_empty());
        let aside: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("profiles.json.corrupt-"))
            .collect();
        assert_eq!(aside.len(), 1);
        let kept = std::fs::read(dir.path().join(&aside[0])).unwrap();
        assert_eq!(kept, b"{ not json");
        assert!(!dir.path().join(FILE).exists());
    }

    #[test]
    fn old_files_without_new_fields_load() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(FILE),
            r#"{"version":1,"profiles":[{"id":"x","name":"Old","presence":{"app_id":"1"}}]}"#,
        )
        .unwrap();
        let s = Store::open(dir.path()).unwrap();
        assert_eq!(s.get("x").unwrap().name, "Old");
        assert_eq!(s.settings(), Settings::default());
    }

    #[test]
    fn ids_are_unique() {
        let ids: std::collections::HashSet<_> = (0..1000).map(|_| new_id()).collect();
        assert_eq!(ids.len(), 1000);
    }
}
