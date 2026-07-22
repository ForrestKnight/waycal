use chrono::{DateTime, Local, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub account: String,
    pub id: String,
    pub summary: String,
    pub access_role: String,
    /// Minutes-before values of the calendar's default popup reminders.
    pub default_reminder_mins: Vec<i64>,
    pub primary: bool,
}

impl Calendar {
    pub fn writable(&self) -> bool {
        self.access_role == "owner" || self.access_role == "writer"
    }

    pub fn from_json(account: &str, v: &Value) -> Option<Self> {
        Some(Self {
            account: account.to_string(),
            id: v.get("id")?.as_str()?.to_string(),
            summary: v.get("summary").and_then(Value::as_str).unwrap_or("").to_string(),
            access_role: v.get("accessRole").and_then(Value::as_str).unwrap_or("reader").to_string(),
            default_reminder_mins: v
                .get("defaultReminders")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter(|r| r.get("method").and_then(Value::as_str) == Some("popup"))
                        .filter_map(|r| r.get("minutes").and_then(Value::as_i64))
                        .collect()
                })
                .unwrap_or_default(),
            primary: v.get("primary").and_then(Value::as_bool).unwrap_or(false),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendee {
    pub email: String,
    pub name: Option<String>,
    /// accepted | declined | tentative | needsAction
    pub status: String,
    pub organizer: bool,
    pub is_self: bool,
}

impl Attendee {
    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.email)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub account: String,
    pub calendar_id: String,
    pub calendar_name: String,
    pub id: String,
    pub summary: String,
    pub all_day: bool,
    /// First day the event is shown on (local).
    pub start_date: NaiveDate,
    /// Last day the event is shown on, inclusive (all-day ends are exclusive in the API).
    pub end_date: NaiveDate,
    pub start_time: Option<DateTime<Local>>,
    pub end_time: Option<DateTime<Local>>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub meet_url: Option<String>,
    pub html_link: Option<String>,
    /// Explicit popup reminder overrides; None means "use calendar defaults".
    pub reminder_overrides: Option<Vec<i64>>,
    /// Guests, organizer first (default keeps old caches deserializable).
    #[serde(default)]
    pub attendees: Vec<Attendee>,
}

fn parse_event_boundary(v: &Value) -> Option<(NaiveDate, Option<DateTime<Local>>)> {
    if let Some(dt) = v.get("dateTime").and_then(Value::as_str) {
        let parsed = DateTime::parse_from_rfc3339(dt).ok()?.with_timezone(&Local);
        return Some((parsed.date_naive(), Some(parsed)));
    }
    let d = v.get("date").and_then(Value::as_str)?;
    Some((NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()?, None))
}

impl Event {
    pub fn from_json(account: &str, calendar_id: &str, calendar_name: &str, v: &Value) -> Option<Self> {
        if v.get("status").and_then(Value::as_str) == Some("cancelled") {
            return None;
        }
        let (start_date, start_time) = parse_event_boundary(v.get("start")?)?;
        let (end_date, end_time) = parse_event_boundary(v.get("end")?)?;
        let all_day = start_time.is_none();
        // All-day ends are exclusive: a one-day event has end = start + 1 day.
        let end_date = if all_day { end_date.pred_opt().unwrap_or(end_date).max(start_date) } else { end_date };

        let meet_url = v
            .get("hangoutLink")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                v.get("conferenceData")?
                    .get("entryPoints")?
                    .as_array()?
                    .iter()
                    .find(|e| e.get("entryPointType").and_then(Value::as_str) == Some("video"))?
                    .get("uri")?
                    .as_str()
                    .map(str::to_string)
            });

        let reminder_overrides = match v.get("reminders") {
            Some(r) if r.get("useDefault").and_then(Value::as_bool) == Some(false) => Some(
                r.get("overrides")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter(|o| o.get("method").and_then(Value::as_str) == Some("popup"))
                            .filter_map(|o| o.get("minutes").and_then(Value::as_i64))
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            _ => None,
        };

        let mut attendees: Vec<Attendee> = v
            .get("attendees")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    // Meeting rooms and other resources aren't people.
                    .filter(|g| g.get("resource").and_then(Value::as_bool) != Some(true))
                    .filter_map(|g| {
                        Some(Attendee {
                            email: g.get("email")?.as_str()?.to_string(),
                            name: g.get("displayName").and_then(Value::as_str).map(str::to_string),
                            status: g
                                .get("responseStatus")
                                .and_then(Value::as_str)
                                .unwrap_or("needsAction")
                                .to_string(),
                            organizer: g.get("organizer").and_then(Value::as_bool).unwrap_or(false),
                            is_self: g.get("self").and_then(Value::as_bool).unwrap_or(false),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        attendees.sort_by_key(|a| (!a.organizer, !a.is_self));

        let non_empty = |key: &str| {
            v.get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };

        Some(Self {
            account: account.to_string(),
            calendar_id: calendar_id.to_string(),
            calendar_name: calendar_name.to_string(),
            id: v.get("id")?.as_str()?.to_string(),
            summary: v
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or("(no title)")
                .to_string(),
            all_day,
            start_date,
            end_date,
            start_time,
            end_time,
            location: non_empty("location"),
            description: non_empty("description"),
            meet_url,
            html_link: non_empty("htmlLink"),
            reminder_overrides,
            attendees,
        })
    }

    pub fn covers(&self, day: NaiveDate) -> bool {
        self.start_date <= day && day <= self.end_date
    }

    pub fn time_label(&self) -> String {
        match (self.start_time, self.end_time) {
            (Some(s), Some(e)) => format!("{}–{}", s.format("%H:%M"), e.format("%H:%M")),
            (Some(s), None) => s.format("%H:%M").to_string(),
            _ => "all day".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskList {
    pub account: String,
    pub id: String,
    pub title: String,
}

impl TaskList {
    pub fn from_json(account: &str, v: &Value) -> Option<Self> {
        Some(Self {
            account: account.to_string(),
            id: v.get("id")?.as_str()?.to_string(),
            title: v.get("title").and_then(Value::as_str).unwrap_or("").to_string(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub account: String,
    pub tasklist_id: String,
    pub tasklist_title: String,
    pub id: String,
    pub title: String,
    pub notes: Option<String>,
    /// Google Tasks due dates carry no meaningful time component.
    pub due: Option<NaiveDate>,
    pub completed: bool,
}

impl Task {
    pub fn from_json(account: &str, tasklist_id: &str, tasklist_title: &str, v: &Value) -> Option<Self> {
        let due = v
            .get("due")
            .and_then(Value::as_str)
            .and_then(|d| NaiveDate::parse_from_str(&d[..10.min(d.len())], "%Y-%m-%d").ok());
        Some(Self {
            account: account.to_string(),
            tasklist_id: tasklist_id.to_string(),
            tasklist_title: tasklist_title.to_string(),
            id: v.get("id")?.as_str()?.to_string(),
            title: v
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("(untitled)")
                .to_string(),
            notes: v
                .get("notes")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            due,
            completed: v.get("status").and_then(Value::as_str) == Some("completed"),
        })
    }
}

/// Parses "HH:MM" into a NaiveTime (used for task_digest_time).
pub fn parse_hhmm(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M").ok()
}
