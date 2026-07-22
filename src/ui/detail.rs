//! Detail panes for one event or task: full info + actions.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;

use super::{App, AppExt, Pane, open_url};
use crate::model::{Event, Task};

fn title_label(text: &str) -> gtk4::Label {
    let lbl = gtk4::Label::new(Some(text));
    lbl.add_css_class("waycal-title");
    lbl.set_wrap(true);
    lbl.set_xalign(0.0);
    lbl.set_max_width_chars(36);
    lbl
}

fn dim_label(text: &str) -> gtk4::Label {
    let lbl = gtk4::Label::new(Some(text));
    lbl.add_css_class("waycal-dim");
    lbl.set_wrap(true);
    lbl.set_xalign(0.0);
    lbl
}

fn body_text(text: &str, max_height: i32) -> gtk4::ScrolledWindow {
    let lbl = gtk4::Label::new(Some(text));
    lbl.set_wrap(true);
    lbl.set_xalign(0.0);
    lbl.set_yalign(0.0);
    lbl.set_selectable(true);
    lbl.set_max_width_chars(38);
    let sw = gtk4::ScrolledWindow::new();
    sw.set_child(Some(&lbl));
    sw.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    sw.set_max_content_height(max_height);
    sw.set_propagate_natural_height(true);
    sw
}

/// Delete button that asks for a second click before acting.
fn confirm_delete_button(on_confirm: impl Fn() + 'static) -> gtk4::Button {
    let btn = gtk4::Button::with_label("delete");
    btn.add_css_class("destructive");
    let armed = Cell::new(false);
    btn.connect_clicked(move |b| {
        if armed.replace(true) {
            on_confirm();
        } else {
            b.set_label("sure?");
        }
    });
    btn
}

fn back_button(app: &Rc<App>) -> gtk4::Button {
    let btn = gtk4::Button::with_label("back");
    let app = app.clone();
    btn.connect_clicked(move |_| app.set_pane(Pane::Main));
    btn
}

pub fn event(app: &Rc<App>, ev: &Event) -> gtk4::Box {
    let pane = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    pane.append(&title_label(&ev.summary));

    let when = if ev.start_date == ev.end_date {
        format!("{} \u{00B7} {}", ev.start_date.format("%a %-d %b %Y"), ev.time_label())
    } else if ev.all_day {
        format!("{} \u{2013} {}", ev.start_date.format("%-d %b"), ev.end_date.format("%-d %b %Y"))
    } else {
        format!(
            "{} {} \u{2013} {} {}",
            ev.start_date.format("%-d %b"),
            ev.start_time.map(|t| t.format("%H:%M").to_string()).unwrap_or_default(),
            ev.end_date.format("%-d %b"),
            ev.end_time.map(|t| t.format("%H:%M").to_string()).unwrap_or_default(),
        )
    };
    pane.append(&dim_label(&when));

    let source = gtk4::Label::new(Some(&format!("{} \u{00B7} {}", ev.account, ev.calendar_name)));
    source.add_css_class(&app.acct_class(&ev.account));
    source.set_xalign(0.0);
    source.add_css_class("waycal-dim");
    pane.append(&source);

    if let Some(loc) = &ev.location {
        pane.append(&dim_label(loc));
    }

    if ev.meet_url.is_some() || ev.html_link.is_some() {
        let links = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        if let Some(url) = ev.meet_url.clone() {
            let btn = gtk4::Button::with_label("join meet");
            btn.connect_clicked(move |_| open_url(&url));
            links.append(&btn);
        }
        if let Some(url) = ev.html_link.clone() {
            let btn = gtk4::Button::with_label("browser");
            btn.connect_clicked(move |_| open_url(&url));
            links.append(&btn);
        }
        pane.append(&links);
    }

    if !ev.attendees.is_empty() {
        let section = gtk4::Label::new(Some(&format!("Attendees ({})", ev.attendees.len())));
        section.add_css_class("waycal-section");
        section.set_xalign(0.0);
        pane.append(&section);

        let list = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        for guest in &ev.attendees {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
            let (mark, class) = match guest.status.as_str() {
                "accepted" => ("\u{2713}", "waycal-ok"),
                "declined" => ("\u{2717}", "waycal-overdue"),
                "tentative" => ("~", "waycal-dim"),
                _ => ("\u{00B7}", "waycal-dim"),
            };
            let status = gtk4::Label::new(Some(mark));
            status.add_css_class(class);
            status.set_width_chars(1);
            row.append(&status);

            let mut text = guest.label().to_string();
            if guest.organizer {
                text.push_str(" (org)");
            }
            let who = gtk4::Label::new(Some(&text));
            who.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            who.set_xalign(0.0);
            if guest.is_self {
                who.add_css_class(&app.acct_class(&ev.account));
            }
            row.append(&who);
            list.append(&row);
        }
        let sw = gtk4::ScrolledWindow::new();
        sw.set_child(Some(&list));
        sw.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        sw.set_max_content_height(120);
        sw.set_propagate_natural_height(true);
        pane.append(&sw);
    }

    if let Some(desc) = &ev.description {
        pane.append(&body_text(desc, 140));
    }

    let spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    pane.append(&spacer);

    let writable = app
        .cache
        .borrow()
        .accounts
        .get(&ev.account)
        .and_then(|d| d.calendars.iter().find(|c| c.id == ev.calendar_id))
        .map(|c| c.writable())
        .unwrap_or(false);

    let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    if writable {
        let edit = gtk4::Button::with_label("edit");
        {
            let app = app.clone();
            let ev = ev.clone();
            edit.connect_clicked(move |_| app.set_pane(Pane::EventForm(Some(ev.clone()))));
        }
        buttons.append(&edit);

        let app2 = app.clone();
        let (account_name, cal_id, ev_id, summary) =
            (ev.account.clone(), ev.calendar_id.clone(), ev.id.clone(), ev.summary.clone());
        buttons.append(&confirm_delete_button(move || {
            let Some(account) = app2.account(&account_name) else { return };
            let (cal_id, ev_id, summary) = (cal_id.clone(), ev_id.clone(), summary.clone());
            app2.spawn_mut(move || {
                crate::gws::delete_event(&account, &cal_id, &ev_id).map(|_| format!("deleted: {summary}"))
            });
        }));
    }
    buttons.append(&back_button(app));
    pane.append(&buttons);

    pane
}

pub fn task(app: &Rc<App>, t: &Task) -> gtk4::Box {
    let pane = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    pane.append(&title_label(&t.title));

    if let Some(due) = t.due {
        pane.append(&dim_label(&format!("due {}", due.format("%a %-d %b %Y"))));
    }

    let source = gtk4::Label::new(Some(&format!("{} \u{00B7} {}", t.account, t.tasklist_title)));
    source.add_css_class(&app.acct_class(&t.account));
    source.add_css_class("waycal-dim");
    source.set_xalign(0.0);
    pane.append(&source);

    if let Some(notes) = &t.notes {
        pane.append(&body_text(notes, 140));
    }

    let spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    pane.append(&spacer);

    let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);

    let complete = gtk4::Button::with_label("complete");
    {
        let app = app.clone();
        let t = t.clone();
        complete.connect_clicked(move |_| super::agenda::complete(&app, &t));
    }
    buttons.append(&complete);

    let edit = gtk4::Button::with_label("edit");
    {
        let app = app.clone();
        let t = t.clone();
        edit.connect_clicked(move |_| app.set_pane(Pane::TaskForm(Some(t.clone()))));
    }
    buttons.append(&edit);

    let app2 = app.clone();
    let (account_name, list_id, task_id, title) =
        (t.account.clone(), t.tasklist_id.clone(), t.id.clone(), t.title.clone());
    buttons.append(&confirm_delete_button(move || {
        let Some(account) = app2.account(&account_name) else { return };
        let (list_id, task_id, title) = (list_id.clone(), task_id.clone(), title.clone());
        app2.spawn_mut(move || {
            crate::gws::delete_task(&account, &list_id, &task_id).map(|_| format!("deleted: {title}"))
        });
    }));

    buttons.append(&back_button(app));
    pane.append(&buttons);

    pane
}
