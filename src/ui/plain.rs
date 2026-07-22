//! The original config-less waycal: month view only, classic key scheme.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{Datelike, Local, NaiveDate};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;

use super::{build_window, days_in_month, load_rounded, month_name, save_rounded};

#[derive(Clone, Copy)]
struct ViewDate {
    year: i32,
    month: u32,
}

impl ViewDate {
    fn today() -> Self {
        let now = Local::now().date_naive();
        Self { year: now.year(), month: now.month() }
    }

    fn shift_month(self, delta: i32) -> Self {
        let total = self.year * 12 + (self.month as i32 - 1) + delta;
        let year = total.div_euclid(12);
        let month = total.rem_euclid(12) as u32 + 1;
        Self { year, month }
    }

    fn shift_year(self, delta: i32) -> Self {
        Self { year: self.year + delta, month: self.month }
    }
}

pub fn build(app: &gtk4::Application) {
    let window = build_window(app);

    let header = gtk4::Label::new(None);
    header.add_css_class("waycal-header");
    header.set_halign(gtk4::Align::Center);

    let grid = gtk4::Grid::new();
    grid.set_row_spacing(2);
    grid.set_column_spacing(2);
    grid.set_halign(gtk4::Align::Center);

    let footer = gtk4::Label::new(Some("\u{2190}\u{2192} mo   \u{2191}\u{2193} yr   \u{23CE} today   s style"));
    footer.add_css_class("waycal-footer");
    footer.set_halign(gtk4::Align::Center);

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    root.add_css_class("waycal-root");
    if load_rounded() {
        root.add_css_class("rounded");
    }
    root.append(&header);
    root.append(&grid);
    root.append(&footer);
    window.set_child(Some(&root));

    let state = Rc::new(RefCell::new(ViewDate::today()));
    render(&grid, &header, *state.borrow());

    let key = gtk4::EventControllerKey::new();
    {
        let state = state.clone();
        let grid = grid.clone();
        let header = header.clone();
        let window = window.clone();
        let root = root.clone();
        key.connect_key_pressed(move |_, keyval, _, _| {
            let current = *state.borrow();
            let next = match keyval {
                gdk::Key::Left => current.shift_month(-1),
                gdk::Key::Right => current.shift_month(1),
                gdk::Key::Up => current.shift_year(-1),
                gdk::Key::Down => current.shift_year(1),
                gdk::Key::Return | gdk::Key::KP_Enter => ViewDate::today(),
                gdk::Key::Escape | gdk::Key::q | gdk::Key::Q => {
                    window.close();
                    return glib::Propagation::Stop;
                }
                gdk::Key::s | gdk::Key::S => {
                    let now_rounded = !root.has_css_class("rounded");
                    if now_rounded {
                        root.add_css_class("rounded");
                    } else {
                        root.remove_css_class("rounded");
                    }
                    save_rounded(now_rounded);
                    return glib::Propagation::Stop;
                }
                _ => return glib::Propagation::Proceed,
            };
            *state.borrow_mut() = next;
            render(&grid, &header, next);
            glib::Propagation::Stop
        });
    }
    window.add_controller(key);

    window.present();
}

fn render(grid: &gtk4::Grid, header: &gtk4::Label, v: ViewDate) {
    header.set_text(&format!("{} {}", month_name(v.month), v.year));

    while let Some(child) = grid.first_child() {
        grid.remove(&child);
    }

    let weekdays = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
    for (i, name) in weekdays.iter().enumerate() {
        let lbl = gtk4::Label::new(Some(name));
        lbl.add_css_class("waycal-weekday");
        grid.attach(&lbl, i as i32, 0, 1, 1);
    }

    let first = NaiveDate::from_ymd_opt(v.year, v.month, 1).unwrap();
    let lead = first.weekday().num_days_from_monday() as i32;
    let days = days_in_month(v.year, v.month) as i32;

    let today = Local::now().date_naive();
    let is_current = today.year() == v.year && today.month() == v.month;
    let today_day = today.day() as i32;

    let prev = v.shift_month(-1);
    let prev_days = days_in_month(prev.year, prev.month) as i32;
    for i in 0..lead {
        let day = prev_days - lead + 1 + i;
        let lbl = gtk4::Label::new(Some(&day.to_string()));
        lbl.add_css_class("waycal-day");
        lbl.add_css_class("dim");
        grid.attach(&lbl, i, 1, 1, 1);
    }

    for d in 1..=days {
        let idx = lead + d - 1;
        let col = idx % 7;
        let row = idx / 7 + 1;
        let lbl = gtk4::Label::new(Some(&d.to_string()));
        lbl.add_css_class("waycal-day");
        if is_current && d == today_day {
            lbl.add_css_class("today");
        }
        grid.attach(&lbl, col, row, 1, 1);
    }

    let total = lead + days;
    let trailing = (7 - total % 7) % 7;
    for i in 0..trailing {
        let day = i + 1;
        let idx = total + i;
        let col = idx % 7;
        let row = idx / 7 + 1;
        let lbl = gtk4::Label::new(Some(&day.to_string()));
        lbl.add_css_class("waycal-day");
        lbl.add_css_class("dim");
        grid.attach(&lbl, col, row, 1, 1);
    }
}
