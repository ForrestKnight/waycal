use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use chrono::{Datelike, Local, NaiveDate};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use serde::Deserialize;

const APP_ID: &str = "com.forrestknight.waycal";

const CSS: &str = r#"
window.waycal {
    background: transparent;
}
.waycal-root {
    background-color: #1a2125;
    border: 2px solid #8FBC8F;
    border-radius: 0;
    padding: 14px 18px;
    color: #c9d1d9;
    font-family: "CaskaydiaMono Nerd Font", monospace;
    font-size: 13px;
    min-width: 260px;
}
.waycal-root.rounded {
    background-color: rgba(26, 33, 37, 0.96);
    border: 2px solid transparent;
    border-radius: 16px;
}
.waycal-header {
    font-weight: bold;
    font-size: 15px;
    padding-bottom: 6px;
}
.waycal-weekday {
    color: #8FBC8F;
    font-weight: bold;
    padding: 2px 6px;
}
.waycal-day {
    padding: 4px 7px;
    min-width: 18px;
}
.waycal-day.dim {
    opacity: 0.3;
}
.waycal-day.today {
    background-color: #8FBC8F;
    color: #1a2125;
    border-radius: 0;
    font-weight: bold;
}
.waycal-root.rounded .waycal-day.today {
    border-radius: 8px;
}
.waycal-footer {
    color: #6a7a71;
    font-size: 10px;
    padding-top: 8px;
    margin-top: 6px;
    border-top: 1px solid rgba(143, 188, 143, 0.18);
}
"#;

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

fn days_in_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let first = NaiveDate::from_ymd_opt(y, m, 1).unwrap();
    let next = NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
    next.signed_duration_since(first).num_days() as u32
}

fn style_state_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(base.join("waycal").join("style"))
}

fn load_rounded() -> bool {
    style_state_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim() == "rounded")
        .unwrap_or(false)
}

fn save_rounded(rounded: bool) {
    if let Some(path) = style_state_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, if rounded { "rounded" } else { "sharp" });
    }
}

#[derive(Deserialize)]
struct Keymaps {
    #[serde(default = "default_month_prev")]
    month_prev: String,
    #[serde(default = "default_month_next")]
    month_next: String,
    #[serde(default = "default_year_prev")]
    year_prev: String,
    #[serde(default = "default_year_next")]
    year_next: String,
    #[serde(default = "default_today")]
    today: String,
    #[serde(default = "default_style_toggle")]
    style_toggle: String,
    #[serde(default = "default_close")]
    close: String,
}

fn default_month_prev() -> String { "Left".into() }
fn default_month_next() -> String { "Right".into() }
fn default_year_prev() -> String { "Up".into() }
fn default_year_next() -> String { "Down".into() }
fn default_today() -> String { "Return".into() }
fn default_style_toggle() -> String { "s".into() }
fn default_close() -> String { "Escape".into() }

impl Default for Keymaps {
    fn default() -> Self {
        Self {
            month_prev: default_month_prev(),
            month_next: default_month_next(),
            year_prev: default_year_prev(),
            year_next: default_year_next(),
            today: default_today(),
            style_toggle: default_style_toggle(),
            close: default_close(),
        }
    }
}

#[derive(Deserialize, Default)]
struct Config {
    #[serde(default)]
    keymaps: Keymaps,
}

fn config_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("waycal").join("config.yaml"))
}

fn load_config() -> Config {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_yml::from_str(&s).ok())
        .unwrap_or_default()
}

fn parse_key(name: &str) -> Option<gdk::Key> {
    gdk::Key::from_name(name)
}

fn month_name(m: u32) -> &'static str {
    match m {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    }
}

fn main() -> glib::ExitCode {
    let app = gtk4::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);
    app.run()
}

fn load_css() {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_ui(app: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(app);
    window.set_decorated(false);
    window.set_resizable(false);
    window.add_css_class("waycal");

    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    window.set_anchor(Edge::Top, true);
    window.set_margin(Edge::Top, 0);

    let header = gtk4::Label::new(None);
    header.add_css_class("waycal-header");
    header.set_halign(gtk4::Align::Center);

    let grid = gtk4::Grid::new();
    grid.set_row_spacing(2);
    grid.set_column_spacing(2);
    grid.set_halign(gtk4::Align::Center);

    let config = load_config();
    let km = &config.keymaps;

    let footer_text = format!(
        "{}/{} mo   {}/{} yr   {} today   {} style",
        km.month_prev, km.month_next, km.year_prev, km.year_next,
        km.today, km.style_toggle
    );
    let footer = gtk4::Label::new(Some(&footer_text));
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
    let key_month_prev = parse_key(&km.month_prev);
    let key_month_next = parse_key(&km.month_next);
    let key_year_prev = parse_key(&km.year_prev);
    let key_year_next = parse_key(&km.year_next);
    let key_today = parse_key(&km.today);
    let key_style_toggle = parse_key(&km.style_toggle);
    let key_close = parse_key(&km.close);

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
                k if key_month_prev == Some(k) => current.shift_month(-1),
                k if key_month_next == Some(k) => current.shift_month(1),
                k if key_year_prev == Some(k) => current.shift_year(-1),
                k if key_year_next == Some(k) => current.shift_year(1),
                k if key_today == Some(k) => ViewDate::today(),
                k if key_close == Some(k) => {
                    window.close();
                    return glib::Propagation::Stop;
                }
                k if key_style_toggle == Some(k) => {
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
