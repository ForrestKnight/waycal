//! Create/edit forms for events and tasks.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{Days, Local, NaiveDate, NaiveTime, TimeZone};
use gtk4::prelude::*;
use serde_json::{Map, Value, json};

use super::{App, AppExt, Pane};
use crate::gws;
use crate::model::{Calendar, Event, Task, TaskList};

fn field_row(label: &str, widget: &impl IsA<gtk4::Widget>) -> gtk4::Box {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let lbl = gtk4::Label::new(Some(label));
    lbl.add_css_class("waycal-dim");
    lbl.set_width_chars(9);
    lbl.set_xalign(0.0);
    row.append(&lbl);
    row.append(widget);
    row
}

fn entry(text: &str, placeholder: &str, chars: i32) -> gtk4::Entry {
    let e = gtk4::Entry::new();
    e.set_text(text);
    e.set_placeholder_text(Some(placeholder));
    if chars > 0 {
        e.set_width_chars(chars);
        e.set_max_width_chars(chars);
    }
    e.set_hexpand(chars == 0);
    e
}

fn text_area(initial: &str) -> (gtk4::ScrolledWindow, gtk4::TextView) {
    let view = gtk4::TextView::new();
    view.set_wrap_mode(gtk4::WrapMode::WordChar);
    view.buffer().set_text(initial);
    let sw = gtk4::ScrolledWindow::new();
    sw.set_child(Some(&view));
    sw.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    sw.set_min_content_height(56);
    sw.set_max_content_height(90);
    sw.set_hexpand(true);
    (sw, view)
}

fn buffer_text(view: &gtk4::TextView) -> String {
    let b = view.buffer();
    b.text(&b.start_iter(), &b.end_iter(), false).trim().to_string()
}

fn header(pane: &gtk4::Box, text: &str) {
    let lbl = gtk4::Label::new(Some(text));
    lbl.add_css_class("waycal-section");
    lbl.set_xalign(0.0);
    pane.append(&lbl);
}

fn buttons_row(app: &Rc<App>, save: gtk4::Button) -> gtk4::Box {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    row.append(&save);
    let cancel = gtk4::Button::with_label("cancel");
    {
        let app = app.clone();
        cancel.connect_clicked(move |_| app.set_pane(Pane::Main));
    }
    row.append(&cancel);
    row
}

pub fn event(app: &Rc<App>, existing: Option<Event>) -> gtk4::Box {
    let pane = gtk4::Box::new(gtk4::Orientation::Vertical, 5);
    header(&pane, if existing.is_some() { "Edit event" } else { "New event" });

    // Account + calendar pickers (creation only; moving events is out of scope).
    let account_names: Vec<String> = app.cfg.accounts.iter().map(|a| a.name.clone()).collect();
    let writable_cals = {
        let app = app.clone();
        move |account: &str| -> Vec<Calendar> {
            app.cache
                .borrow()
                .accounts
                .get(account)
                .map(|d| d.calendars.iter().filter(|c| c.writable()).cloned().collect())
                .unwrap_or_default()
        }
    };

    let mut account_dd = None;
    let mut cal_dd = None;
    let cals: Rc<RefCell<Vec<Calendar>>> = Rc::new(RefCell::new(Vec::new()));

    match &existing {
        Some(ev) => {
            let source = gtk4::Label::new(Some(&format!("{} \u{00B7} {}", ev.account, ev.calendar_name)));
            source.add_css_class(&app.acct_class(&ev.account));
            source.add_css_class("waycal-dim");
            source.set_xalign(0.0);
            pane.append(&source);
        }
        None => {
            let names: Vec<&str> = account_names.iter().map(String::as_str).collect();
            let add = gtk4::DropDown::from_strings(&names);
            let cd = gtk4::DropDown::from_strings(&[]);
            cd.set_hexpand(true);

            let refill = {
                let cals = cals.clone();
                let cd = cd.clone();
                let writable_cals = writable_cals.clone();
                move |account: &str| {
                    let list = writable_cals(account);
                    let names: Vec<String> = list
                        .iter()
                        .map(|c| if c.primary { format!("{} (primary)", c.summary) } else { c.summary.clone() })
                        .collect();
                    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                    cd.set_model(Some(&gtk4::StringList::new(&refs)));
                    let primary = list.iter().position(|c| c.primary).unwrap_or(0);
                    cd.set_selected(primary as u32);
                    *cals.borrow_mut() = list;
                }
            };
            if let Some(first) = account_names.first() {
                refill(first);
            }
            // Cache may still be empty on first-ever launch; fill once sync lands.
            if cals.borrow().is_empty() {
                let refill = refill.clone();
                let add = add.clone();
                let account_names = account_names.clone();
                let cals = cals.clone();
                *app.on_sync.borrow_mut() = Some(Box::new(move || {
                    if cals.borrow().is_empty() {
                        if let Some(name) = account_names.get(add.selected() as usize) {
                            refill(name);
                        }
                    }
                }));
            }
            {
                let account_names = account_names.clone();
                add.connect_selected_notify(move |dd| {
                    if let Some(name) = account_names.get(dd.selected() as usize) {
                        refill(name);
                    }
                });
            }
            pane.append(&field_row("account", &add));
            pane.append(&field_row("calendar", &cd));
            account_dd = Some(add);
            cal_dd = Some(cd);
        }
    }

    let title = entry(existing.as_ref().map(|e| e.summary.as_str()).unwrap_or(""), "title", 0);
    pane.append(&field_row("title", &title));

    let all_day = gtk4::CheckButton::with_label("all day");
    all_day.set_active(existing.as_ref().map(|e| e.all_day).unwrap_or(false));
    pane.append(&field_row("", &all_day));

    let sel = app.selected.get();
    let (sd, ed) = existing
        .as_ref()
        .map(|e| (e.start_date, e.end_date))
        .unwrap_or((sel, sel));
    let (st, et) = existing
        .as_ref()
        .and_then(|e| e.start_time.zip(e.end_time))
        .map(|(s, e)| (s.format("%H:%M").to_string(), e.format("%H:%M").to_string()))
        .unwrap_or(("09:00".into(), "10:00".into()));

    let start_date = entry(&sd.format("%Y-%m-%d").to_string(), "YYYY-MM-DD", 10);
    let start_time = entry(&st, "HH:MM", 5);
    let start_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    start_row.append(&start_date);
    start_row.append(&start_time);
    pane.append(&field_row("start", &start_row));

    let end_date = entry(&ed.format("%Y-%m-%d").to_string(), "YYYY-MM-DD", 10);
    let end_time = entry(&et, "HH:MM", 5);
    let end_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    end_row.append(&end_date);
    end_row.append(&end_time);
    pane.append(&field_row("end", &end_row));

    let sync_times = {
        let start_time = start_time.clone();
        let end_time = end_time.clone();
        move |all: bool| {
            start_time.set_sensitive(!all);
            end_time.set_sensitive(!all);
        }
    };
    sync_times(all_day.is_active());
    {
        let sync_times = sync_times.clone();
        all_day.connect_toggled(move |c| sync_times(c.is_active()));
    }

    let location = entry(
        existing.as_ref().and_then(|e| e.location.as_deref()).unwrap_or(""),
        "location",
        0,
    );
    pane.append(&field_row("location", &location));

    let guest_emails = existing
        .as_ref()
        .map(|e| e.attendees.iter().map(|a| a.email.clone()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default();
    let guests = entry(&guest_emails, "guest emails, comma separated", 0);
    pane.append(&field_row("guests", &guests));

    let extras = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    // Meet rooms can only be attached at creation time from here.
    let meet = gtk4::CheckButton::with_label("meet link");
    if existing.is_none() {
        extras.append(&meet);
    }
    let notify = gtk4::CheckButton::with_label("email guests");
    notify.set_active(true);
    extras.append(&notify);
    pane.append(&field_row("", &extras));

    let (desc_sw, desc_view) = text_area(existing.as_ref().and_then(|e| e.description.as_deref()).unwrap_or(""));
    pane.append(&field_row("notes", &desc_sw));

    let save = gtk4::Button::with_label(if existing.is_some() { "save" } else { "create" });
    {
        let app = app.clone();
        save.connect_clicked(move |_| {
            let summary = title.text().trim().to_string();
            if summary.is_empty() {
                app.set_status("title is required", true);
                return;
            }
            let Ok(sd) = NaiveDate::parse_from_str(start_date.text().trim(), "%Y-%m-%d") else {
                app.set_status("bad start date (YYYY-MM-DD)", true);
                return;
            };
            let Ok(ed) = NaiveDate::parse_from_str(end_date.text().trim(), "%Y-%m-%d") else {
                app.set_status("bad end date (YYYY-MM-DD)", true);
                return;
            };

            let (start, end) = if all_day.is_active() {
                if ed < sd {
                    app.set_status("end before start", true);
                    return;
                }
                (
                    json!({"date": sd.format("%Y-%m-%d").to_string()}),
                    json!({"date": (ed + Days::new(1)).format("%Y-%m-%d").to_string()}),
                )
            } else {
                let Ok(st) = NaiveTime::parse_from_str(start_time.text().trim(), "%H:%M") else {
                    app.set_status("bad start time (HH:MM)", true);
                    return;
                };
                let Ok(et) = NaiveTime::parse_from_str(end_time.text().trim(), "%H:%M") else {
                    app.set_status("bad end time (HH:MM)", true);
                    return;
                };
                let (Some(sdt), Some(edt)) = (
                    Local.from_local_datetime(&sd.and_time(st)).earliest(),
                    Local.from_local_datetime(&ed.and_time(et)).earliest(),
                ) else {
                    app.set_status("invalid local time", true);
                    return;
                };
                if edt <= sdt {
                    app.set_status("end before start", true);
                    return;
                }
                (json!({"dateTime": sdt.to_rfc3339()}), json!({"dateTime": edt.to_rfc3339()}))
            };

            let guest_list: Vec<String> = guests
                .text()
                .split([',', ';', ' '])
                .map(str::trim)
                .filter(|g| !g.is_empty())
                .map(str::to_string)
                .collect();
            if let Some(bad) = guest_list.iter().find(|g| !g.contains('@')) {
                app.set_status(&format!("bad guest email: {bad}"), true);
                return;
            }

            let mut body = Map::new();
            body.insert("summary".into(), json!(summary));
            body.insert("start".into(), start);
            body.insert("end".into(), end);
            body.insert("location".into(), json!(location.text().trim()));
            body.insert("description".into(), json!(buffer_text(&desc_view)));
            // On edit the list replaces the current guests (empty clears them);
            // on create it's only sent when there are guests.
            if existing.is_some() || !guest_list.is_empty() {
                let attendees: Vec<Value> = guest_list.iter().map(|g| json!({"email": g})).collect();
                body.insert("attendees".into(), json!(attendees));
            }
            if existing.is_none() && meet.is_active() {
                body.insert(
                    "conferenceData".into(),
                    json!({"createRequest": {
                        "requestId": format!("waycal-{}", chrono::Local::now().timestamp_millis()),
                        "conferenceSolutionKey": {"type": "hangoutsMeet"},
                    }}),
                );
            }
            let body = Value::Object(body);
            let send_updates = notify.is_active();

            match &existing {
                Some(ev) => {
                    let Some(account) = app.account(&ev.account) else { return };
                    let (cal_id, ev_id) = (ev.calendar_id.clone(), ev.id.clone());
                    app.spawn_mut(move || {
                        gws::patch_event(&account, &cal_id, &ev_id, &body, send_updates)
                            .map(|_| format!("saved: {summary}"))
                    });
                }
                None => {
                    let idx = account_dd.as_ref().map(|d| d.selected() as usize).unwrap_or(0);
                    let Some(account) = app.cfg.accounts.get(idx).cloned() else { return };
                    let cal_idx = cal_dd.as_ref().map(|d| d.selected() as usize).unwrap_or(0);
                    let Some(cal) = cals.borrow().get(cal_idx).cloned() else {
                        app.set_status("no writable calendar (still syncing?)", true);
                        return;
                    };
                    app.spawn_mut(move || {
                        gws::insert_event(&account, &cal.id, &body, send_updates)
                            .map(|_| format!("created: {summary}"))
                    });
                }
            }
        });
    }
    pane.append(&buttons_row(app, save));
    pane
}

pub fn task(app: &Rc<App>, existing: Option<Task>) -> gtk4::Box {
    let pane = gtk4::Box::new(gtk4::Orientation::Vertical, 5);
    header(&pane, if existing.is_some() { "Edit task" } else { "New task" });

    let account_names: Vec<String> = app.cfg.accounts.iter().map(|a| a.name.clone()).collect();
    let lists_for = {
        let app = app.clone();
        move |account: &str| -> Vec<TaskList> {
            app.cache
                .borrow()
                .accounts
                .get(account)
                .map(|d| d.tasklists.clone())
                .unwrap_or_default()
        }
    };

    let mut account_dd = None;
    let mut list_dd = None;
    let lists: Rc<RefCell<Vec<TaskList>>> = Rc::new(RefCell::new(Vec::new()));

    match &existing {
        Some(t) => {
            let source = gtk4::Label::new(Some(&format!("{} \u{00B7} {}", t.account, t.tasklist_title)));
            source.add_css_class(&app.acct_class(&t.account));
            source.add_css_class("waycal-dim");
            source.set_xalign(0.0);
            pane.append(&source);
        }
        None => {
            let names: Vec<&str> = account_names.iter().map(String::as_str).collect();
            let add = gtk4::DropDown::from_strings(&names);
            let ld = gtk4::DropDown::from_strings(&[]);
            ld.set_hexpand(true);

            let refill = {
                let lists = lists.clone();
                let ld = ld.clone();
                let lists_for = lists_for.clone();
                move |account: &str| {
                    let found = lists_for(account);
                    let names: Vec<String> = found.iter().map(|l| l.title.clone()).collect();
                    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                    ld.set_model(Some(&gtk4::StringList::new(&refs)));
                    ld.set_selected(0);
                    *lists.borrow_mut() = found;
                }
            };
            if let Some(first) = account_names.first() {
                refill(first);
            }
            if lists.borrow().is_empty() {
                let refill = refill.clone();
                let add = add.clone();
                let account_names = account_names.clone();
                let lists = lists.clone();
                *app.on_sync.borrow_mut() = Some(Box::new(move || {
                    if lists.borrow().is_empty() {
                        if let Some(name) = account_names.get(add.selected() as usize) {
                            refill(name);
                        }
                    }
                }));
            }
            {
                let account_names = account_names.clone();
                add.connect_selected_notify(move |dd| {
                    if let Some(name) = account_names.get(dd.selected() as usize) {
                        refill(name);
                    }
                });
            }
            pane.append(&field_row("account", &add));
            pane.append(&field_row("list", &ld));
            account_dd = Some(add);
            list_dd = Some(ld);
        }
    }

    let title = entry(existing.as_ref().map(|t| t.title.as_str()).unwrap_or(""), "title", 0);
    pane.append(&field_row("title", &title));

    let due_text = existing
        .as_ref()
        .and_then(|t| t.due)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let due = entry(&due_text, "YYYY-MM-DD (optional)", 0);
    pane.append(&field_row("due", &due));

    let (notes_sw, notes_view) = text_area(existing.as_ref().and_then(|t| t.notes.as_deref()).unwrap_or(""));
    pane.append(&field_row("notes", &notes_sw));

    let save = gtk4::Button::with_label(if existing.is_some() { "save" } else { "create" });
    {
        let app = app.clone();
        save.connect_clicked(move |_| {
            let text = title.text().trim().to_string();
            if text.is_empty() {
                app.set_status("title is required", true);
                return;
            }
            let due_raw = due.text().trim().to_string();
            let due_value = if due_raw.is_empty() {
                None
            } else {
                match NaiveDate::parse_from_str(&due_raw, "%Y-%m-%d") {
                    Ok(d) => Some(json!(format!("{}T00:00:00.000Z", d.format("%Y-%m-%d")))),
                    Err(_) => {
                        app.set_status("bad due date (YYYY-MM-DD)", true);
                        return;
                    }
                }
            };

            // gws rejects explicit nulls, so empty fields are omitted; edits
            // go through `update` (full replace) where omission clears them.
            let mut body = Map::new();
            body.insert("title".into(), json!(text));
            let notes = buffer_text(&notes_view);
            if !notes.is_empty() {
                body.insert("notes".into(), json!(notes));
            }
            if let Some(due) = due_value {
                body.insert("due".into(), due);
            }

            match &existing {
                Some(t) => {
                    body.insert("id".into(), json!(t.id));
                    body.insert("status".into(), json!("needsAction"));
                    let body = Value::Object(body);
                    let Some(account) = app.account(&t.account) else { return };
                    let (list_id, task_id) = (t.tasklist_id.clone(), t.id.clone());
                    app.spawn_mut(move || {
                        gws::update_task(&account, &list_id, &task_id, &body).map(|_| format!("saved: {text}"))
                    });
                }
                None => {
                    let body = Value::Object(body);
                    let idx = account_dd.as_ref().map(|d| d.selected() as usize).unwrap_or(0);
                    let Some(account) = app.cfg.accounts.get(idx).cloned() else { return };
                    let list_idx = list_dd.as_ref().map(|d| d.selected() as usize).unwrap_or(0);
                    let Some(list) = lists.borrow().get(list_idx).cloned() else {
                        app.set_status("no task list (still syncing?)", true);
                        return;
                    };
                    app.spawn_mut(move || {
                        gws::insert_task(&account, &list.id, &body).map(|_| format!("created: {text}"))
                    });
                }
            }
        });
    }
    pane.append(&buttons_row(app, save));
    pane
}
