mod agenda;
mod detail;
mod form;
mod month;
mod plain;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use chrono::{Datelike, Days, Local, NaiveDate};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::cache::{self, Cache};
use crate::config::{self, Account, Config};
use crate::gws::{self, AccountData};
use crate::model::{Event, Task};

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
    border-bottom: 2px solid transparent;
}
.waycal-day.dim {
    opacity: 0.3;
}
.waycal-day.busy {
    border-bottom-color: rgba(143, 188, 143, 0.55);
}
.waycal-day.selected {
    background-color: rgba(143, 188, 143, 0.22);
}
.waycal-day.today {
    background-color: #8FBC8F;
    color: #1a2125;
    border-radius: 0;
    font-weight: bold;
}
.waycal-root.rounded .waycal-day.selected,
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
.waycal-side {
    min-width: 320px;
}
.waycal-side separator {
    background: rgba(143, 188, 143, 0.18);
    min-height: 1px;
    margin: 4px 0;
}
.waycal-vsep {
    background: rgba(143, 188, 143, 0.18);
    min-width: 1px;
    margin: 0 6px;
}
.waycal-section {
    color: #8FBC8F;
    font-weight: bold;
    font-size: 12px;
}
.waycal-title {
    font-weight: bold;
    font-size: 14px;
}
.waycal-dim {
    color: #6a7a71;
    font-size: 11px;
}
.waycal-overdue {
    color: #e06c75;
}
.waycal-ok {
    color: #8FBC8F;
}
.waycal-error {
    color: #e06c75;
}
.waycal-meet {
    color: #8FBC8F;
    font-size: 10px;
    border: 1px solid rgba(143, 188, 143, 0.4);
    padding: 0 4px;
}
.waycal-status {
    color: #6a7a71;
    font-size: 10px;
    padding-top: 4px;
}
.waycal-list, .waycal-list row {
    background: transparent;
    padding: 0;
}
.waycal-list row:hover {
    background: rgba(143, 188, 143, 0.10);
}
.waycal-item {
    padding: 3px 4px;
}
.waycal-side scrolledwindow {
    background: transparent;
}
.waycal-side button {
    background: rgba(143, 188, 143, 0.12);
    border: 1px solid rgba(143, 188, 143, 0.4);
    border-radius: 0;
    box-shadow: none;
    color: #c9d1d9;
    font-size: 11px;
    min-height: 0;
    padding: 2px 10px;
}
.waycal-side button:hover {
    background: rgba(143, 188, 143, 0.25);
}
.waycal-side button.destructive {
    border-color: rgba(224, 108, 117, 0.6);
    color: #e06c75;
}
.waycal-root.rounded .waycal-side button {
    border-radius: 8px;
}
.waycal-side checkbutton check {
    background: #232c31;
    border: 1px solid rgba(143, 188, 143, 0.5);
    min-width: 12px;
    min-height: 12px;
    -gtk-icon-size: 10px;
}
.waycal-side checkbutton check:checked {
    background: #8FBC8F;
    color: #1a2125;
}
.waycal-side entry {
    background: #232c31;
    border: 1px solid rgba(143, 188, 143, 0.3);
    border-radius: 0;
    box-shadow: none;
    color: #c9d1d9;
    font-size: 12px;
    min-height: 0;
    padding: 3px 6px;
    caret-color: #8FBC8F;
}
.waycal-root.rounded .waycal-side entry {
    border-radius: 6px;
}
.waycal-side textview, .waycal-side textview text {
    background: #232c31;
    color: #c9d1d9;
    font-size: 12px;
}
.waycal-side dropdown button {
    padding: 2px 6px;
}
"#;

pub(crate) fn days_in_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let first = NaiveDate::from_ymd_opt(y, m, 1).unwrap();
    let next = NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
    next.signed_duration_since(first).num_days() as u32
}

pub(crate) fn add_months(d: NaiveDate, delta: i32) -> NaiveDate {
    let total = d.year() * 12 + d.month0() as i32 + delta;
    let year = total.div_euclid(12);
    let month = total.rem_euclid(12) as u32 + 1;
    let day = d.day().min(days_in_month(year, month));
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn style_state_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(base.join("waycal").join("style"))
}

pub(crate) fn load_rounded() -> bool {
    style_state_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim() == "rounded")
        .unwrap_or(false)
}

pub(crate) fn save_rounded(rounded: bool) {
    if let Some(path) = style_state_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, if rounded { "rounded" } else { "sharp" });
    }
}

pub(crate) fn month_name(m: u32) -> &'static str {
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

/// What the right-hand panel is currently showing.
pub enum Pane {
    Main,
    EventDetail(Event),
    TaskDetail(Task),
    EventForm(Option<Event>),
    TaskForm(Option<Task>),
}

/// Results sent back from worker threads to the GTK main loop.
pub enum Msg {
    Fetched {
        account: String,
        data: AccountData,
        from: NaiveDate,
        to: NaiveDate,
        errors: Vec<String>,
    },
    Mutated(Result<String, String>),
}

pub struct App {
    pub cfg: Config,
    pub window: gtk4::ApplicationWindow,
    pub header: gtk4::Label,
    pub grid: gtk4::Grid,
    pub right_content: gtk4::Box,
    pub status: gtk4::Label,
    pub root: gtk4::Box,
    pub cache: RefCell<Cache>,
    pub selected: Cell<NaiveDate>,
    pub pane: RefCell<Pane>,
    pub tx: async_channel::Sender<Msg>,
    pub syncing: Cell<i32>,
    /// One-shot hook run after each sync; forms use it to fill pickers that
    /// were empty because data hadn't arrived yet.
    pub on_sync: RefCell<Option<Box<dyn Fn()>>>,
}

impl App {
    pub fn account(&self, name: &str) -> Option<Account> {
        self.cfg.accounts.iter().find(|a| a.name == name).cloned()
    }

    /// CSS class carrying the account's accent color.
    pub fn acct_class(&self, name: &str) -> String {
        let idx = self.cfg.accounts.iter().position(|a| a.name == name).unwrap_or(0);
        format!("acct-{idx}")
    }

    /// Events covering `day`, all accounts merged, all-day first then by time.
    /// Duplicates (e.g. a holiday calendar subscribed on both accounts) collapse by event id.
    pub fn events_on(&self, day: NaiveDate) -> Vec<Event> {
        let cache = self.cache.borrow();
        let mut seen = std::collections::HashSet::new();
        let mut evs: Vec<Event> = cache
            .accounts
            .values()
            .flat_map(|d| d.events.iter())
            .filter(|e| e.covers(day) && seen.insert(e.id.clone()))
            .cloned()
            .collect();
        evs.sort_by_key(|e| (!e.all_day, e.start_time, e.summary.clone()));
        evs
    }

    /// Pending tasks across accounts: overdue/nearest due first, no due date last.
    pub fn all_tasks(&self) -> Vec<Task> {
        let cache = self.cache.borrow();
        let mut tasks: Vec<Task> = cache
            .accounts
            .values()
            .flat_map(|d| d.tasks.iter())
            .filter(|t| !t.completed)
            .cloned()
            .collect();
        tasks.sort_by_key(|t| (t.due.is_none(), t.due, t.title.clone()));
        tasks
    }

    pub fn set_status(&self, text: &str, error: bool) {
        self.status.set_text(text);
        if error {
            self.status.add_css_class("waycal-error");
        } else {
            self.status.remove_css_class("waycal-error");
        }
    }

    fn window_range(&self) -> (NaiveDate, NaiveDate) {
        let cache = self.cache.borrow();
        match (cache.from, cache.to) {
            (Some(f), Some(t)) => (f, t),
            _ => {
                let today = Local::now().date_naive();
                (today - Days::new(7), today + Days::new(45))
            }
        }
    }
}

pub trait AppExt {
    fn select(&self, day: NaiveDate);
    fn set_pane(&self, pane: Pane);
    fn render(&self);
    fn render_right(&self);
    fn refresh(&self);
    fn spawn_fetch(&self, from: NaiveDate, to: NaiveDate);
    fn spawn_mut<F>(&self, op: F)
    where
        F: FnOnce() -> Result<String, String> + Send + 'static;
}

impl AppExt for Rc<App> {
    fn select(&self, day: NaiveDate) {
        self.selected.set(day);
        // Grow the fetch window when navigation leaves the cached range.
        let month_start = day.with_day(1).unwrap();
        let month_end = month_start + Days::new(days_in_month(day.year(), day.month()) as u64 - 1);
        let need_from = month_start - Days::new(7);
        let need_to = month_end + Days::new(8);
        let covered = {
            let c = self.cache.borrow();
            c.covers(need_from) && c.covers(need_to - Days::new(1))
        };
        if !covered {
            let (cur_from, cur_to) = self.window_range();
            self.spawn_fetch(cur_from.min(need_from), cur_to.max(need_to));
        }
        self.render();
    }

    fn set_pane(&self, pane: Pane) {
        *self.on_sync.borrow_mut() = None;
        *self.pane.borrow_mut() = pane;
        self.render_right();
    }

    fn render(&self) {
        month::render(self);
        // Don't clobber an open form/detail when background data lands.
        if matches!(*self.pane.borrow(), Pane::Main) {
            self.render_right();
        }
    }

    fn render_right(&self) {
        while let Some(child) = self.right_content.first_child() {
            self.right_content.remove(&child);
        }
        let pane = match &*self.pane.borrow() {
            Pane::Main => agenda::build(self),
            Pane::EventDetail(e) => detail::event(self, e),
            Pane::TaskDetail(t) => detail::task(self, t),
            Pane::EventForm(existing) => form::event(self, existing.clone()),
            Pane::TaskForm(existing) => form::task(self, existing.clone()),
        };
        self.right_content.append(&pane);
    }

    fn refresh(&self) {
        let (from, to) = self.window_range();
        self.spawn_fetch(from, to);
    }

    fn spawn_fetch(&self, from: NaiveDate, to: NaiveDate) {
        self.syncing.set(self.syncing.get() + self.cfg.accounts.len() as i32);
        self.set_status("syncing…", false);
        for account in self.cfg.accounts.clone() {
            let tx = self.tx.clone();
            let hide = self.cfg.hide_event_types.clone();
            std::thread::spawn(move || {
                let mut errors = Vec::new();
                let data = gws::fetch_account(&account, from, to, &hide, &mut errors);
                let _ = tx.send_blocking(Msg::Fetched { account: account.name, data, from, to, errors });
            });
        }
    }

    fn spawn_mut<F>(&self, op: F)
    where
        F: FnOnce() -> Result<String, String> + Send + 'static,
    {
        self.set_status("saving…", false);
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send_blocking(Msg::Mutated(op()));
        });
    }
}

pub fn run() {
    let app = gtk4::Application::builder().application_id(APP_ID).build();
    let cfg = Rc::new(config::load());
    {
        let cfg = cfg.clone();
        app.connect_startup(move |_| {
            if let Some(settings) = gtk4::Settings::default() {
                settings.set_gtk_application_prefer_dark_theme(true);
            }
            load_css(cfg.as_ref().as_ref());
        });
    }
    app.connect_activate(move |gtk_app| match cfg.as_ref() {
        Some(cfg) => build_full(gtk_app, cfg.clone()),
        None => plain::build(gtk_app),
    });
    app.run_with_args::<&str>(&[]);
}

fn load_css(cfg: Option<&Config>) {
    let mut css = CSS.to_string();
    if let Some(cfg) = cfg {
        for (i, account) in cfg.accounts.iter().enumerate() {
            css.push_str(&format!(".acct-{i} {{ color: {}; }}\n", account.color));
        }
    }
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(&css);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_window(app: &gtk4::Application) -> gtk4::ApplicationWindow {
    let window = gtk4::ApplicationWindow::new(app);
    window.set_decorated(false);
    window.set_resizable(false);
    window.add_css_class("waycal");
    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    window.set_anchor(Edge::Top, true);
    window.set_margin(Edge::Top, 0);
    window
}

fn build_full(gtk_app: &gtk4::Application, cfg: Config) {
    let window = build_window(gtk_app);

    let header = gtk4::Label::new(None);
    header.add_css_class("waycal-header");
    header.set_halign(gtk4::Align::Center);

    let grid = gtk4::Grid::new();
    grid.set_row_spacing(2);
    grid.set_column_spacing(2);
    grid.set_halign(gtk4::Align::Center);
    grid.set_valign(gtk4::Align::Start);

    let footer = gtk4::Label::new(Some(
        "\u{2190}\u{2192} day  \u{2191}\u{2193} wk  PgUp/Dn mo  \u{23CE} today\nn event  t task  r sync  s style",
    ));
    footer.add_css_class("waycal-footer");
    footer.set_halign(gtk4::Align::Center);
    footer.set_justify(gtk4::Justification::Center);

    let left = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    left.append(&header);
    left.append(&grid);
    let spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    left.append(&spacer);
    left.append(&footer);

    let right_content = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    right_content.set_vexpand(true);

    let status = gtk4::Label::new(None);
    status.add_css_class("waycal-status");
    status.set_halign(gtk4::Align::Start);
    status.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    status.set_max_width_chars(44);

    let right = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    right.add_css_class("waycal-side");
    right.append(&right_content);
    right.append(&status);

    let vsep = gtk4::Separator::new(gtk4::Orientation::Vertical);
    vsep.add_css_class("waycal-vsep");

    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    hbox.append(&left);
    hbox.append(&vsep);
    hbox.append(&right);

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    root.add_css_class("waycal-root");
    if load_rounded() {
        root.add_css_class("rounded");
    }
    root.append(&hbox);
    window.set_child(Some(&root));

    let (tx, rx) = async_channel::unbounded::<Msg>();
    let app = Rc::new(App {
        cfg,
        window: window.clone(),
        header,
        grid,
        right_content,
        status,
        root: root.clone(),
        cache: RefCell::new(cache::load()),
        selected: Cell::new(Local::now().date_naive()),
        pane: RefCell::new(Pane::Main),
        tx,
        syncing: Cell::new(0),
        on_sync: RefCell::new(None),
    });

    // Paint from cache immediately, then refresh in the background.
    app.render();
    app.refresh();

    {
        let app = app.clone();
        glib::spawn_future_local(async move {
            while let Ok(msg) = rx.recv().await {
                handle_msg(&app, msg);
            }
        });
    }

    let key = gtk4::EventControllerKey::new();
    {
        let app = app.clone();
        key.connect_key_pressed(move |_, keyval, _, modifier| on_key(&app, keyval, modifier));
    }
    window.add_controller(key);

    window.present();
}

fn handle_msg(app: &Rc<App>, msg: Msg) {
    match msg {
        Msg::Fetched { account, data, from, to, errors } => {
            {
                let mut cache = app.cache.borrow_mut();
                cache.accounts.insert(account, data);
                cache.from = Some(from);
                cache.to = Some(to);
                cache.fetched_at = Some(Local::now());
            }
            cache::save(&app.cache.borrow());
            app.syncing.set((app.syncing.get() - 1).max(0));
            if let Some(err) = errors.first() {
                app.set_status(err, true);
            } else if app.syncing.get() == 0 {
                app.set_status(&format!("synced {}", Local::now().format("%H:%M")), false);
            }
            app.render();
            if let Some(hook) = &*app.on_sync.borrow() {
                hook();
            }
        }
        Msg::Mutated(result) => match result {
            Ok(desc) => {
                app.set_status(&desc, false);
                app.set_pane(Pane::Main);
                app.refresh();
            }
            Err(err) => {
                app.set_status(&err, true);
                // Refetch so any optimistic UI change is reverted by real data.
                app.refresh();
            }
        },
    }
}

fn on_key(app: &Rc<App>, keyval: gdk::Key, modifier: gdk::ModifierType) -> glib::Propagation {
    let in_main = matches!(*app.pane.borrow(), Pane::Main);
    let in_form = matches!(*app.pane.borrow(), Pane::EventForm(_) | Pane::TaskForm(_));

    // `q` mirrors Esc, except in forms where it must type the letter.
    if keyval == gdk::Key::Escape
        || (matches!(keyval, gdk::Key::q | gdk::Key::Q) && !in_form)
    {
        if in_main {
            app.window.close();
        } else {
            app.set_pane(Pane::Main);
        }
        return glib::Propagation::Stop;
    }
    if !in_main {
        // Let entries/text views in forms handle their own keys.
        return glib::Propagation::Proceed;
    }

    let sel = app.selected.get();
    let shift = modifier.contains(gdk::ModifierType::SHIFT_MASK);
    match keyval {
        gdk::Key::Left => app.select(sel - Days::new(1)),
        gdk::Key::Right => app.select(sel + Days::new(1)),
        gdk::Key::Up => app.select(sel - Days::new(7)),
        gdk::Key::Down => app.select(sel + Days::new(7)),
        gdk::Key::Page_Up => app.select(add_months(sel, if shift { -12 } else { -1 })),
        gdk::Key::Page_Down => app.select(add_months(sel, if shift { 12 } else { 1 })),
        gdk::Key::Return | gdk::Key::KP_Enter => app.select(Local::now().date_naive()),
        gdk::Key::n | gdk::Key::N => app.set_pane(Pane::EventForm(None)),
        gdk::Key::t | gdk::Key::T => app.set_pane(Pane::TaskForm(None)),
        gdk::Key::r | gdk::Key::R => app.refresh(),
        gdk::Key::s | gdk::Key::S => {
            let rounded = !app.root.has_css_class("rounded");
            if rounded {
                app.root.add_css_class("rounded");
            } else {
                app.root.remove_css_class("rounded");
            }
            save_rounded(rounded);
        }
        _ => return glib::Propagation::Proceed,
    }
    glib::Propagation::Stop
}

pub(crate) fn open_url(url: &str) {
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}
