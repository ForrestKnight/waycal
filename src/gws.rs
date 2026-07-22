use std::process::Command;

use chrono::{DateTime, Local, NaiveDate};
use serde_json::{Value, json};

use crate::config::{Account, expand_tilde};
use crate::model::{Calendar, Event, Task, TaskList};

pub type Result<T> = std::result::Result<T, String>;

/// Runs one gws invocation for the given account and parses the JSON reply.
pub fn run(account: &Account, args: &[&str], params: Option<&Value>, body: Option<&Value>) -> Result<Value> {
    // gws drops a stray `download.html` in its cwd when a response has no
    // body (e.g. deletes), so keep it out of wherever waycal was launched.
    let scratch = std::env::temp_dir().join("waycal-gws");
    let _ = std::fs::create_dir_all(&scratch);
    let mut cmd = Command::new("gws");
    cmd.env("GOOGLE_WORKSPACE_CLI_CONFIG_DIR", expand_tilde(&account.config_dir))
        .env("GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE", expand_tilde(&account.credentials_file))
        .current_dir(&scratch)
        .args(args);
    if let Some(p) = params {
        cmd.arg("--params").arg(p.to_string());
    }
    if let Some(b) = body {
        cmd.arg("--json").arg(b.to_string());
    }
    let out = cmd
        .output()
        .map_err(|e| format!("[{}] cannot run gws: {}", account.name, e))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "[{}] gws {} failed: {}",
            account.name,
            args.join(" "),
            err.lines().last().unwrap_or("unknown error").trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Ok(Value::Null); // delete returns no body
    }
    serde_json::from_str(trimmed).map_err(|e| format!("[{}] bad JSON from gws: {}", account.name, e))
}

fn items(v: &Value) -> Vec<Value> {
    v.get("items").and_then(Value::as_array).cloned().unwrap_or_default()
}

pub fn list_calendars(account: &Account) -> Result<Vec<Calendar>> {
    let v = run(account, &["calendar", "calendarList", "list"], Some(&json!({"maxResults": 250})), None)?;
    Ok(items(&v)
        .iter()
        // Match what the user shows in the Google Calendar UI.
        .filter(|c| c.get("selected").and_then(Value::as_bool).unwrap_or(false))
        .filter_map(|c| Calendar::from_json(&account.name, c))
        .collect())
}

pub fn list_events(
    account: &Account,
    cal: &Calendar,
    from: DateTime<Local>,
    to: DateTime<Local>,
    hide_types: &[String],
) -> Result<Vec<Event>> {
    let params = json!({
        "calendarId": cal.id,
        "singleEvents": true,
        "orderBy": "startTime",
        "timeMin": from.to_rfc3339(),
        "timeMax": to.to_rfc3339(),
        "maxResults": 250,
    });
    let v = run(account, &["calendar", "events", "list"], Some(&params), None)?;
    Ok(items(&v)
        .iter()
        .filter(|e| {
            let ty = e.get("eventType").and_then(Value::as_str).unwrap_or("default");
            !hide_types.iter().any(|h| h == ty)
        })
        .filter_map(|e| Event::from_json(&account.name, &cal.id, &cal.summary, e))
        .collect())
}

/// `notify` controls whether Google emails the guests about the change.
pub fn insert_event(account: &Account, calendar_id: &str, body: &Value, notify: bool) -> Result<Value> {
    let params = json!({
        "calendarId": calendar_id,
        "conferenceDataVersion": 1,
        "sendUpdates": if notify { "all" } else { "none" },
    });
    run(account, &["calendar", "events", "insert"], Some(&params), Some(body))
}

pub fn patch_event(account: &Account, calendar_id: &str, event_id: &str, body: &Value, notify: bool) -> Result<Value> {
    let params = json!({
        "calendarId": calendar_id,
        "eventId": event_id,
        "conferenceDataVersion": 1,
        "sendUpdates": if notify { "all" } else { "none" },
    });
    run(account, &["calendar", "events", "patch"], Some(&params), Some(body))
}

pub fn delete_event(account: &Account, calendar_id: &str, event_id: &str) -> Result<()> {
    let params = json!({"calendarId": calendar_id, "eventId": event_id});
    run(account, &["calendar", "events", "delete"], Some(&params), None).map(|_| ())
}

pub fn list_tasklists(account: &Account) -> Result<Vec<TaskList>> {
    let v = run(account, &["tasks", "tasklists", "list"], Some(&json!({"maxResults": 100})), None)?;
    Ok(items(&v).iter().filter_map(|t| TaskList::from_json(&account.name, t)).collect())
}

pub fn list_tasks(account: &Account, list: &TaskList) -> Result<Vec<Task>> {
    let params = json!({"tasklist": list.id, "maxResults": 100, "showCompleted": false});
    let v = run(account, &["tasks", "tasks", "list"], Some(&params), None)?;
    Ok(items(&v)
        .iter()
        .filter_map(|t| Task::from_json(&account.name, &list.id, &list.title, t))
        .collect())
}

pub fn insert_task(account: &Account, tasklist_id: &str, body: &Value) -> Result<Value> {
    run(account, &["tasks", "tasks", "insert"], Some(&json!({"tasklist": tasklist_id})), Some(body))
}

pub fn patch_task(account: &Account, tasklist_id: &str, task_id: &str, body: &Value) -> Result<Value> {
    let params = json!({"tasklist": tasklist_id, "task": task_id});
    run(account, &["tasks", "tasks", "patch"], Some(&params), Some(body))
}

/// Full-resource update. Unlike patch, omitted optional fields (due, notes)
/// are cleared — and gws' schema validation rejects explicit nulls, so this
/// is the only way to clear them.
pub fn update_task(account: &Account, tasklist_id: &str, task_id: &str, body: &Value) -> Result<Value> {
    let params = json!({"tasklist": tasklist_id, "task": task_id});
    run(account, &["tasks", "tasks", "update"], Some(&params), Some(body))
}

pub fn delete_task(account: &Account, tasklist_id: &str, task_id: &str) -> Result<()> {
    let params = json!({"tasklist": tasklist_id, "task": task_id});
    run(account, &["tasks", "tasks", "delete"], Some(&params), None).map(|_| ())
}

pub fn complete_task(account: &Account, tasklist_id: &str, task_id: &str) -> Result<Value> {
    patch_task(account, tasklist_id, task_id, &json!({"status": "completed"}))
}

/// Everything waycal shows for one account.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AccountData {
    pub calendars: Vec<Calendar>,
    pub events: Vec<Event>,
    pub tasklists: Vec<TaskList>,
    pub tasks: Vec<Task>,
}

/// Fetches calendars+events (over [from, to)) and tasklists+tasks for one account.
/// Partial failures degrade to what could be fetched; errors are collected.
pub fn fetch_account(
    account: &Account,
    from: NaiveDate,
    to: NaiveDate,
    hide_types: &[String],
    errors: &mut Vec<String>,
) -> AccountData {
    let mut data = AccountData::default();

    let day_start = |d: NaiveDate| {
        d.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).earliest().unwrap()
    };
    match list_calendars(account) {
        Ok(cals) => {
            for cal in &cals {
                match list_events(account, cal, day_start(from), day_start(to), hide_types) {
                    Ok(evs) => data.events.extend(evs),
                    Err(e) => errors.push(e),
                }
            }
            data.calendars = cals;
        }
        Err(e) => errors.push(e),
    }

    match list_tasklists(account) {
        Ok(lists) => {
            for list in &lists {
                match list_tasks(account, list) {
                    Ok(ts) => data.tasks.extend(ts),
                    Err(e) => errors.push(e),
                }
            }
            data.tasklists = lists;
        }
        Err(e) => errors.push(e),
    }

    data.events.sort_by_key(|e| (e.start_date, e.start_time));
    data
}
