//! Month grid for the full UI: day selection, click-to-select, busy dots.

use std::rc::Rc;

use chrono::{Datelike, Days, Local, NaiveDate};
use gtk4::prelude::*;

use super::{App, AppExt, days_in_month, month_name};

pub fn render(app: &Rc<App>) {
    let selected = app.selected.get();
    app.header
        .set_text(&format!("{} {}", month_name(selected.month()), selected.year()));

    while let Some(child) = app.grid.first_child() {
        app.grid.remove(&child);
    }

    let weekdays = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
    for (i, name) in weekdays.iter().enumerate() {
        let lbl = gtk4::Label::new(Some(name));
        lbl.add_css_class("waycal-weekday");
        app.grid.attach(&lbl, i as i32, 0, 1, 1);
    }

    let first = selected.with_day(1).unwrap();
    let lead = first.weekday().num_days_from_monday() as i64;
    let days = days_in_month(selected.year(), selected.month()) as i64;
    let total = lead + days;
    let cells = total + (7 - total % 7) % 7;
    let grid_start = first - Days::new(lead as u64);
    let today = Local::now().date_naive();

    for idx in 0..cells {
        let date = grid_start + Days::new(idx as u64);
        let col = (idx % 7) as i32;
        let row = (idx / 7 + 1) as i32;
        let lbl = gtk4::Label::new(Some(&date.day().to_string()));
        lbl.add_css_class("waycal-day");
        if date.month() != selected.month() {
            lbl.add_css_class("dim");
        }
        if has_events(app, date) {
            lbl.add_css_class("busy");
        }
        if date == selected {
            lbl.add_css_class("selected");
        }
        if date == today {
            lbl.add_css_class("today");
        }

        let click = gtk4::GestureClick::new();
        {
            let app = app.clone();
            click.connect_released(move |_, _, _, _| app.select(date));
        }
        lbl.add_controller(click);
        app.grid.attach(&lbl, col, row, 1, 1);
    }
}

fn has_events(app: &App, day: NaiveDate) -> bool {
    let cache = app.cache.borrow();
    cache.accounts.values().flat_map(|d| d.events.iter()).any(|e| e.covers(day))
}
