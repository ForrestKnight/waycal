use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    pub name: String,
    pub config_dir: String,
    pub credentials_file: String,
    #[serde(default = "default_account_color")]
    pub color: String,
}

fn default_account_color() -> String {
    "#8FBC8F".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_reminder_mins")]
    pub default_reminder_mins: i64,
    /// "HH:MM" local time for the daily task digest; None disables it.
    #[serde(default)]
    pub task_digest_time: Option<String>,
    #[serde(default = "default_hidden_event_types")]
    pub hide_event_types: Vec<String>,
    #[serde(default)]
    pub accounts: Vec<Account>,
}

fn default_poll_interval() -> u64 {
    300
}

fn default_reminder_mins() -> i64 {
    10
}

fn default_hidden_event_types() -> Vec<String> {
    vec!["workingLocation".into(), "birthday".into()]
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    PathBuf::from(path)
}

pub fn config_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| expand_tilde("~/.config"))
        .join("waycal/config.toml")
}

/// Loads the config file. Returns None when it doesn't exist (plain
/// calendar mode); parse errors are reported so a typo doesn't silently
/// disable the panels.
pub fn load() -> Option<Config> {
    let path = config_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "waycal: no config at {} — running plain calendar. \
                 Add [[accounts]] entries there to enable Google Calendar/Tasks.",
                path.display()
            );
            return None;
        }
        Err(e) => {
            eprintln!("waycal: cannot read {}: {}", path.display(), e);
            return None;
        }
    };
    match toml::from_str::<Config>(&text) {
        Ok(cfg) if cfg.accounts.is_empty() => {
            eprintln!("waycal: {} has no [[accounts]] — running plain calendar", path.display());
            None
        }
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!("waycal: invalid config {}: {}", path.display(), e);
            None
        }
    }
}
