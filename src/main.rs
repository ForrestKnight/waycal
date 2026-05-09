use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use chrono::{Datelike, Local, NaiveDate};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

const APP_ID: &str = "com.forrestknight.waycal";

struct ThemeColors {
    bg: String,
    fg: String,
    accent: String,
    on_accent: String,
}

fn load_theme_colors() -> Option<ThemeColors> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    let path = base.join("omarchy/current/theme/colors.toml");
    let content = std::fs::read_to_string(path).ok()?;
    let mut bg = None;
    let mut fg = None;
    let mut accent = None;
    for line in content.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let key = k.trim();
        let val = v.trim().trim_matches('"').to_string();
        match key {
            "background" => bg = Some(val),
            "foreground" => fg = Some(val),
            "accent" => accent = Some(val),
            _ => {}
        }
    }
    let bg = bg?;
    Some(ThemeColors { on_accent: bg.clone(), bg, fg: fg?, accent: accent? })
}

fn build_css() -> String {
    let (bg, fg, accent, on_accent) = match load_theme_colors() {
        Some(c) => (c.bg, c.fg, c.accent, c.on_accent),
        None => (
            "@theme_bg_color".into(),
            "@theme_fg_color".into(),
            "@theme_selected_bg_color".into(),
            "@theme_selected_fg_color".into(),
        ),
    };
    format!(
        r#"
window.waycal {{
    background: transparent;
}}
.waycal-root {{
    background-color: {bg};
    border: 2px solid {accent};
    border-radius: 0;
    padding: 14px 18px;
    color: {fg};
    font-family: "CaskaydiaMono Nerd Font", monospace;
    font-size: 13px;
    min-width: 260px;
}}
.waycal-root.rounded {{
    background-color: alpha({bg}, 0.96);
    border: 2px solid transparent;
    border-radius: 16px;
}}
.waycal-header {{
    font-weight: bold;
    font-size: 15px;
    padding-bottom: 6px;
}}
.waycal-weekday {{
    color: {accent};
    font-weight: bold;
    padding: 2px 6px;
}}
.waycal-day {{
    padding: 4px 7px;
    min-width: 18px;
}}
.waycal-day.dim {{
    opacity: 0.3;
}}
.waycal-day.today {{
    background-color: {accent};
    color: {on_accent};
    border-radius: 0;
    font-weight: bold;
}}
.waycal-root.rounded .waycal-day.today {{
    border-radius: 8px;
}}
.waycal-footer {{
    color: alpha({fg}, 0.55);
    font-size: 10px;
    padding-top: 8px;
    margin-top: 6px;
    border-top: 1px solid alpha({accent}, 0.25);
}}
"#
    )
}

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
    provider.load_from_string(&build_css());
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
                gdk::Key::Escape => {
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
