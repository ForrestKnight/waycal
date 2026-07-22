//! Headless notification daemon: polls both accounts, fires desktop
//! notifications for Google event reminders and a daily due-task digest.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

use chrono::{DateTime, Days, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::config::{self, Config};
use crate::model::parse_hhmm;
use crate::{cache, gws};

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    /// "event_id@reminder_ts" → reminder time, for dedup and pruning.
    #[serde(default)]
    notified: BTreeMap<String, DateTime<Local>>,
    #[serde(default)]
    digest_date: Option<NaiveDate>,
}

fn state_path() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| config::expand_tilde("~/.local/state"))
        .join("waycal/notified.json")
}

fn load_state() -> State {
    std::fs::read_to_string(state_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_state(state: &State) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string(state) {
        let _ = std::fs::write(path, text);
    }
}

fn notify(summary: &str, body: &str, open_url: Option<&str>) {
    let mut cmd = Command::new("notify-send");
    // -t 0: never expires, stays until the user clicks or dismisses it.
    cmd.args(["-a", "waycal", "-i", "x-office-calendar", "-t", "0"]);
    if let Some(url) = open_url {
        // With an action registered, notify-send blocks until the
        // notification is closed, so wait it out in a detached thread.
        cmd.args(["-A", "default=Open Meet", summary, body]);
        let url = url.to_string();
        std::thread::spawn(move || match cmd.output() {
            Ok(out) if out.stdout.trim_ascii() == b"default" => {
                let _ = Command::new("xdg-open").arg(url).status();
            }
            Ok(_) => {}
            Err(e) => eprintln!("waycal daemon: notify-send failed: {e}"),
        });
    } else if let Err(e) = cmd.args([summary, body]).status() {
        eprintln!("waycal daemon: notify-send failed: {e}");
    }
}

pub fn run() {
    let Some(cfg) = config::load() else {
        eprintln!("waycal daemon: needs accounts in the config file");
        std::process::exit(1);
    };
    eprintln!(
        "waycal daemon: watching {} account(s), polling every {}s",
        cfg.accounts.len(),
        cfg.poll_interval_secs
    );
    let mut state = load_state();
    loop {
        tick(&cfg, &mut state);
        std::thread::sleep(std::time::Duration::from_secs(cfg.poll_interval_secs.max(30)));
    }
}

fn tick(cfg: &Config, state: &mut State) {
    let now = Local::now();
    let today = now.date_naive();

    // Same window the popup uses, so both share a fresh cache.
    let from = today - Days::new(7);
    let to = today + Days::new(45);
    let mut errors = Vec::new();
    let mut cache = cache::Cache {
        fetched_at: Some(now),
        from: Some(from),
        to: Some(to),
        ..Default::default()
    };
    for account in &cfg.accounts {
        let data = gws::fetch_account(account, from, to, &cfg.hide_event_types, &mut errors);
        cache.accounts.insert(account.name.clone(), data);
    }
    for e in &errors {
        eprintln!("waycal daemon: {e}");
    }
    if errors.len() >= cfg.accounts.len() * 2 {
        // Everything failed (offline?) — don't overwrite a good cache.
        return;
    }
    cache::save(&cache);

    // Event reminders (timed events only; duplicates across accounts collapse by id).
    let mut seen = HashSet::new();
    for data in cache.accounts.values() {
        for ev in &data.events {
            let Some(start) = ev.start_time else { continue };
            if start <= now || !seen.insert(ev.id.clone()) {
                continue;
            }
            // Google reminder semantics: event overrides win; otherwise the
            // calendar's default popup reminders; otherwise our configured fallback.
            let minutes = match &ev.reminder_overrides {
                Some(overrides) => overrides.clone(),
                None => {
                    let defaults = data
                        .calendars
                        .iter()
                        .find(|c| c.id == ev.calendar_id)
                        .map(|c| c.default_reminder_mins.clone())
                        .unwrap_or_default();
                    if defaults.is_empty() { vec![cfg.default_reminder_mins] } else { defaults }
                }
            };
            for m in minutes {
                let remind_at = start - Duration::minutes(m);
                if remind_at > now {
                    continue;
                }
                let key = format!("{}@{}", ev.id, remind_at.timestamp());
                if state.notified.contains_key(&key) {
                    continue;
                }
                let mut body = format!("{} \u{00B7} {}", ev.account, ev.calendar_name);
                if let Some(loc) = &ev.location {
                    body.push_str(&format!("\n{loc}"));
                }
                if let Some(url) = &ev.meet_url {
                    body.push_str(&format!("\n{url}"));
                }
                notify(
                    &format!("{} {}", start.format("%H:%M"), ev.summary),
                    &body,
                    ev.meet_url.as_deref(),
                );
                state.notified.insert(key, remind_at);
            }
        }
    }

    // Morning digest of tasks due (or overdue) today.
    if let Some(digest_at) = cfg.task_digest_time.as_deref().and_then(parse_hhmm)
        && now.time() >= digest_at && state.digest_date != Some(today) {
            let mut due: Vec<_> = cache
                .accounts
                .values()
                .flat_map(|d| d.tasks.iter())
                .filter(|t| !t.completed && t.due.is_some_and(|d| d <= today))
                .collect();
            due.sort_by_key(|t| (t.due, t.title.clone()));
            if !due.is_empty() {
                let body: Vec<String> = due
                    .iter()
                    .map(|t| {
                        let overdue = t.due.is_some_and(|d| d < today);
                        format!("\u{2022} {}{} ({})", t.title, if overdue { " (overdue)" } else { "" }, t.account)
                    })
                    .collect();
                notify(&format!("{} task(s) due today", due.len()), &body.join("\n"), None);
            }
            state.digest_date = Some(today);
        }

    let cutoff = now - Duration::days(2);
    state.notified.retain(|_, ts| *ts > cutoff);
    save_state(state);
}
