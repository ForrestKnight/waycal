use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::config::expand_tilde;
use crate::gws::AccountData;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cache {
    pub fetched_at: Option<DateTime<Local>>,
    /// Window [from, to) the cached events cover.
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub accounts: BTreeMap<String, AccountData>,
}

impl Cache {
    pub fn covers(&self, day: NaiveDate) -> bool {
        matches!((self.from, self.to), (Some(f), Some(t)) if f <= day && day < t)
    }
}

fn cache_path() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| expand_tilde("~/.cache"))
        .join("waycal/data.json")
}

pub fn load() -> Cache {
    std::fs::read_to_string(cache_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(cache: &Cache) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(text) = serde_json::to_string(cache) else { return };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}
