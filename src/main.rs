mod cache;
mod config;
mod daemon;
mod gws;
mod model;
mod ui;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("daemon") => daemon::run(),
        Some("dump") => dump(),
        Some("--help" | "-h") => {
            println!("waycal            open the calendar popup");
            println!("waycal daemon     run the notification daemon");
            println!("waycal dump       print fetched events/tasks (debug)");
        }
        _ => ui::run(),
    }
}

/// Debug helper: fetch everything for the configured window and print it.
fn dump() {
    let Some(cfg) = config::load() else {
        eprintln!("waycal dump: needs a config with accounts");
        std::process::exit(1);
    };
    let today = chrono::Local::now().date_naive();
    let from = today - chrono::Days::new(7);
    let to = today + chrono::Days::new(45);
    let mut errors = Vec::new();
    for account in &cfg.accounts {
        let data = gws::fetch_account(account, from, to, &cfg.hide_event_types, &mut errors);
        println!(
            "== {} — {} calendars, {} events, {} tasklists, {} tasks",
            account.name,
            data.calendars.len(),
            data.events.len(),
            data.tasklists.len(),
            data.tasks.len()
        );
        for e in &data.events {
            println!(
                "  {} {:<13} {}  [{}]{}{}",
                e.start_date,
                e.time_label(),
                e.summary,
                e.calendar_name,
                e.meet_url.as_deref().map(|u| format!("  meet: {u}")).unwrap_or_default(),
                if e.attendees.is_empty() {
                    String::new()
                } else {
                    format!("  ({} guests)", e.attendees.len())
                }
            );
        }
        for t in &data.tasks {
            println!(
                "  task: {} (due {}) [{}]",
                t.title,
                t.due.map(|d| d.to_string()).unwrap_or_else(|| "-".into()),
                t.tasklist_title
            );
        }
    }
    for e in &errors {
        eprintln!("error: {e}");
    }
}
