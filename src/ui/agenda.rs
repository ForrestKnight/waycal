//! Right-hand main pane: selected day's agenda + pending tasks + action buttons.

use std::rc::Rc;

use chrono::Local;
use gtk4::prelude::*;

use super::{App, AppExt, Pane};
use crate::model::{Event, Task};

pub fn build(app: &Rc<App>) -> gtk4::Box {
    let pane = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

    let selected = app.selected.get();
    let day_header = gtk4::Label::new(Some(&selected.format("%a %-d %b").to_string()));
    day_header.add_css_class("waycal-section");
    day_header.set_halign(gtk4::Align::Start);
    pane.append(&day_header);

    let events = app.events_on(selected);
    if events.is_empty() {
        let empty = gtk4::Label::new(Some("no events"));
        empty.add_css_class("waycal-dim");
        empty.set_halign(gtk4::Align::Start);
        pane.append(&empty);
    } else {
        pane.append(&event_list(app, events, 200));
    }

    pane.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

    let tasks_header = gtk4::Label::new(Some("Tasks"));
    tasks_header.add_css_class("waycal-section");
    tasks_header.set_halign(gtk4::Align::Start);
    pane.append(&tasks_header);

    let tasks = app.all_tasks();
    if tasks.is_empty() {
        let empty = gtk4::Label::new(Some("no pending tasks"));
        empty.add_css_class("waycal-dim");
        empty.set_halign(gtk4::Align::Start);
        pane.append(&empty);
    } else {
        pane.append(&task_list(app, tasks, 170));
    }

    let spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    pane.append(&spacer);

    let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let new_event = gtk4::Button::with_label("+ event");
    let new_task = gtk4::Button::with_label("+ task");
    {
        let app = app.clone();
        new_event.connect_clicked(move |_| app.set_pane(Pane::EventForm(None)));
    }
    {
        let app = app.clone();
        new_task.connect_clicked(move |_| app.set_pane(Pane::TaskForm(None)));
    }
    buttons.append(&new_event);
    buttons.append(&new_task);
    pane.append(&buttons);

    pane
}

fn scrolled(child: &impl IsA<gtk4::Widget>, max_height: i32) -> gtk4::ScrolledWindow {
    let sw = gtk4::ScrolledWindow::new();
    sw.set_child(Some(child));
    sw.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    sw.set_max_content_height(max_height);
    sw.set_propagate_natural_height(true);
    sw
}

fn event_list(app: &Rc<App>, events: Vec<Event>, max_height: i32) -> gtk4::ScrolledWindow {
    let list = gtk4::ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    list.add_css_class("waycal-list");

    for event in &events {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        row.add_css_class("waycal-item");

        let dot = gtk4::Label::new(Some("\u{25CF}"));
        dot.add_css_class(&app.acct_class(&event.account));
        row.append(&dot);

        let time = gtk4::Label::new(Some(&event.time_label()));
        time.add_css_class("waycal-dim");
        time.set_width_chars(11);
        time.set_xalign(0.0);
        row.append(&time);

        let title = gtk4::Label::new(Some(&event.summary));
        title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title.set_hexpand(true);
        title.set_xalign(0.0);
        row.append(&title);

        if event.meet_url.is_some() {
            let meet = gtk4::Label::new(Some("meet"));
            meet.add_css_class("waycal-meet");
            row.append(&meet);
        }
        list.append(&row);
    }

    let events = Rc::new(events);
    {
        let app = app.clone();
        list.connect_row_activated(move |_, row| {
            if let Some(event) = events.get(row.index() as usize) {
                app.set_pane(Pane::EventDetail(event.clone()));
            }
        });
    }

    scrolled(&list, max_height)
}

fn task_list(app: &Rc<App>, tasks: Vec<Task>, max_height: i32) -> gtk4::ScrolledWindow {
    let list = gtk4::ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    list.add_css_class("waycal-list");
    let today = Local::now().date_naive();

    for task in &tasks {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        row.add_css_class("waycal-item");

        let check = gtk4::CheckButton::new();
        check.add_css_class(&app.acct_class(&task.account));
        {
            let app = app.clone();
            let task = task.clone();
            check.connect_toggled(move |c| {
                if c.is_active() {
                    complete(&app, &task);
                }
            });
        }
        row.append(&check);

        let title = gtk4::Label::new(Some(&task.title));
        title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title.set_hexpand(true);
        title.set_xalign(0.0);
        row.append(&title);

        if let Some(due) = task.due {
            let text = if due == today { "today".to_string() } else { due.format("%-d %b").to_string() };
            let lbl = gtk4::Label::new(Some(&text));
            lbl.add_css_class(if due < today { "waycal-overdue" } else { "waycal-dim" });
            row.append(&lbl);
        }
        list.append(&row);
    }

    let tasks = Rc::new(tasks);
    {
        let app = app.clone();
        list.connect_row_activated(move |_, row| {
            if let Some(task) = tasks.get(row.index() as usize) {
                app.set_pane(Pane::TaskDetail(task.clone()));
            }
        });
    }

    scrolled(&list, max_height)
}

/// Optimistically hides the task, then patches it as completed.
pub fn complete(app: &Rc<App>, task: &Task) {
    if let Some(data) = app.cache.borrow_mut().accounts.get_mut(&task.account)
        && let Some(t) = data.tasks.iter_mut().find(|t| t.id == task.id) {
            t.completed = true;
        }
    app.render();

    let Some(account) = app.account(&task.account) else { return };
    let (list_id, task_id, title) = (task.tasklist_id.clone(), task.id.clone(), task.title.clone());
    app.spawn_mut(move || {
        crate::gws::complete_task(&account, &list_id, &task_id)
            .map(|_| format!("completed: {title}"))
    });
}
