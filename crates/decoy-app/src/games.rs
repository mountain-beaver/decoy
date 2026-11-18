//! The Discord game list: cache, background refresh, search.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use decoy_core::GameIndex;
use serde::{Deserialize, Serialize};

/// The game list is refreshed when the cache is older than this.
const MAX_AGE_SECS: u64 = 7 * 24 * 3600;
const ICON_SIZE: u32 = 64;
const MAX_SEARCH_LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GamesInfo {
    pub count: usize,
    pub fetched_at: Option<u64>,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GameHit {
    pub id: String,
    pub name: String,
    pub icon_url: Option<String>,
}

pub type Fetcher = Box<dyn Fn() -> decoy_core::Result<GameIndex> + Send + Sync>;

#[derive(Default)]
struct Status {
    loading: bool,
    error: Option<String>,
}

pub struct Games {
    index: RwLock<Option<GameIndex>>,
    status: Mutex<Status>,
    cache_dir: PathBuf,
    fetcher: Fetcher,
    /// Called whenever `info()` would return something new.
    on_change: Arc<dyn Fn() + Send + Sync>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl Games {
    /// A game list that downloads from Discord.
    pub fn new(cache_dir: &Path, on_change: Arc<dyn Fn() + Send + Sync>) -> Arc<Games> {
        Self::with_fetcher(cache_dir, on_change, Box::new(GameIndex::fetch))
    }

    pub fn with_fetcher(
        cache_dir: &Path,
        on_change: Arc<dyn Fn() + Send + Sync>,
        fetcher: Fetcher,
    ) -> Arc<Games> {
        Arc::new(Games {
            index: RwLock::new(None),
            status: Mutex::new(Status::default()),
            cache_dir: cache_dir.to_path_buf(),
            fetcher,
            on_change,
        })
    }

    /// Load the cache, and refresh in the background when it is missing or old.
    pub fn start(self: &Arc<Self>) {
        let cached = GameIndex::load(&self.cache_dir).ok();
        let stale = cached.as_ref().is_none_or(|i| i.is_stale(MAX_AGE_SECS));
        self.set_index(cached);
        if stale {
            self.refresh();
        }
    }

    fn set_index(&self, index: Option<GameIndex>) {
        *self.index.write().unwrap_or_else(|p| p.into_inner()) = index;
    }

    /// Replace the list (tests, or after a download).
    pub fn set(&self, index: GameIndex) {
        self.set_index(Some(index));
    }

    pub fn info(&self) -> GamesInfo {
        let index = self.index.read().unwrap_or_else(|p| p.into_inner());
        let st = lock(&self.status);
        GamesInfo {
            count: index.as_ref().map_or(0, |g| g.len()),
            fetched_at: index.as_ref().map(|g| g.fetched_at),
            loading: st.loading,
            error: st.error.clone(),
        }
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<GameHit> {
        let index = self.index.read().unwrap_or_else(|p| p.into_inner());
        let Some(index) = index.as_ref() else {
            return Vec::new();
        };
        index
            .search(query, limit.min(MAX_SEARCH_LIMIT))
            .into_iter()
            .map(|g| GameHit {
                id: g.id.clone(),
                name: g.name.clone(),
                icon_url: g.icon_url(ICON_SIZE),
            })
            .collect()
    }

    /// Fetch the game list on a background thread; no-op if one is running.
    pub fn refresh(self: &Arc<Self>) {
        {
            let mut st = lock(&self.status);
            if st.loading {
                return;
            }
            st.loading = true;
            st.error = None;
        }
        (self.on_change)();
        let this = self.clone();
        let spawned = std::thread::Builder::new()
            .name("decoy-games".into())
            .spawn(move || {
                let error = match (this.fetcher)() {
                    Ok(index) => {
                        if let Err(e) = index.save(&this.cache_dir) {
                            eprintln!("decoy: could not cache the game list: {e}");
                        }
                        this.set(index);
                        None
                    }
                    Err(e) => Some(e.to_string()),
                };
                {
                    let mut st = lock(&this.status);
                    st.loading = false;
                    st.error = error;
                }
                (this.on_change)();
            });
        if let Err(e) = spawned {
            let mut st = lock(&self.status);
            st.loading = false;
            st.error = Some(format!("Could not start the download: {e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use decoy_core::Game;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn wait_until(mut f: impl FnMut() -> bool) {
        let end = Instant::now() + Duration::from_secs(5);
        while !f() {
            assert!(Instant::now() < end, "timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn game(id: &str, name: &str, icon: Option<&str>) -> Game {
        Game {
            id: id.into(),
            name: name.into(),
            icon_hash: icon.map(Into::into),
            aliases: vec![],
        }
    }

    fn counter() -> (Arc<AtomicUsize>, Arc<dyn Fn() + Send + Sync>) {
        let n = Arc::new(AtomicUsize::new(0));
        let n2 = n.clone();
        (
            n,
            Arc::new(move || {
                n2.fetch_add(1, Ordering::SeqCst);
            }),
        )
    }

    fn failing() -> Fetcher {
        Box::new(|| Err(decoy_core::Error::Network("offline".into())))
    }

    #[test]
    fn search_uses_injected_index() {
        let dir = tempfile::tempdir().unwrap();
        let (_, cb) = counter();
        let g = Games::with_fetcher(dir.path(), cb, failing());
        assert!(g.search("mine", 5).is_empty());
        g.set(GameIndex::new(
            vec![game("1", "Minecraft", Some("abc")), game("2", "Mine", None)],
            42,
        ));
        let hits = g.search("mine", 10);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].name, "Mine");
        assert_eq!(hits[0].icon_url, None);
        assert_eq!(
            hits[1].icon_url.as_deref(),
            Some("https://cdn.discordapp.com/app-icons/1/abc.png?size=64")
        );
        assert_eq!(g.search("mine", 1).len(), 1);
        let info = g.info();
        assert_eq!((info.count, info.fetched_at), (2, Some(42)));
    }

    #[test]
    fn refresh_success_saves_cache_and_notifies() {
        let dir = tempfile::tempdir().unwrap();
        let (n, cb) = counter();
        let g = Games::with_fetcher(
            dir.path(),
            cb,
            Box::new(|| Ok(GameIndex::new(vec![game("1", "G", None)], now_secs()))),
        );
        g.start();
        wait_until(|| {
            let i = g.info();
            !i.loading && i.count == 1
        });
        assert_eq!(g.info().error, None);
        assert!(dir.path().join("games.json").exists());
        // Loading started, loading finished.
        wait_until(|| n.load(Ordering::SeqCst) >= 2);

        // A fresh cache is used without fetching again.
        let (tx, rx) = mpsc::channel();
        let (_, cb) = counter();
        let g2 = Games::with_fetcher(
            dir.path(),
            cb,
            Box::new(move || {
                tx.send(()).unwrap();
                Err(decoy_core::Error::Network("should not be called".into()))
            }),
        );
        g2.start();
        assert_eq!(g2.info().count, 1);
        assert!(rx.recv_timeout(Duration::from_millis(200)).is_err());
    }

    #[test]
    fn refresh_failure_lands_in_error() {
        let dir = tempfile::tempdir().unwrap();
        let (_, cb) = counter();
        let g = Games::with_fetcher(dir.path(), cb, failing());
        g.start();
        wait_until(|| {
            let i = g.info();
            !i.loading && i.error.is_some()
        });
        let i = g.info();
        assert_eq!(i.count, 0);
        assert_eq!(i.error.as_deref(), Some("Network error: offline"));
        g.refresh();
        wait_until(|| {
            let i = g.info();
            !i.loading && i.error.is_some()
        });
    }

    #[test]
    fn stale_cache_is_used_and_refreshed() {
        let dir = tempfile::tempdir().unwrap();
        GameIndex::new(vec![], 1).save(dir.path()).unwrap();
        let (tx, rx) = mpsc::channel();
        let (_, cb) = counter();
        let g = Games::with_fetcher(
            dir.path(),
            cb,
            Box::new(move || {
                tx.send(()).unwrap();
                Err(decoy_core::Error::Network("x".into()))
            }),
        );
        g.start();
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
        assert_eq!(g.info().fetched_at, Some(1));
    }
}
