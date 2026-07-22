# waycal

A tiny calendar popup for Waybar. Click an icon in the bar, a small month view drops down under the bar, arrow keys navigate, Esc closes. That's it.

Written in Rust with GTK4 and `gtk4-layer-shell` so the popup anchors itself to the top of the screen via the Wayland layer-shell protocol — no compositor config needed.

<p align="center">
  <img src="screenshot.png" alt="waycal sharp style" width="360">
  &nbsp;&nbsp;
  <img src="screenshot-rounded.png" alt="waycal rounded style" width="360">
</p>

## Features

- **Month view** with today highlighted, leading/trailing days dimmed
- **Google Calendar & Tasks** (optional): agenda and task panel for multiple
  accounts via the [`gws` CLI](https://github.com/googleworkspace/cli) —
  create/edit/delete events and tasks, complete tasks, join Meet links, plus a
  notification daemon for reminders and a daily due-task digest. See
  [Google Calendar & Tasks integration](#google-calendar--tasks-integration)
- **Keyboard nav:** `←`/`→` month, `↑`/`↓` year, `Enter` today, `s` toggle style, `Esc` close
- **Two looks:** press `s` to swap between a sharp-cornered, bordered "Omarchy" style and a soft rounded style. Your choice is remembered between launches
- **Toggle-click:** clicking the Waybar icon while the popup is open closes it
- **Anchored** just below the bar, horizontally centered, no config file hacks
- **Dark theme** with a sage-green accent, monospace font. Self-contained CSS — no theme integration or external dependencies to worry about.

## Requirements

waycal is a small native app, not a Waybar plugin. It runs on any Linux desktop that has:

- A **Wayland compositor supporting `wlr-layer-shell`**
  — Hyprland, Sway, river, Wayfire, Hikari, LabWC, etc. (not GNOME or KDE — those don't implement layer-shell)
- **Waybar** (for the click-to-launch integration)
- **GTK4** and **gtk4-layer-shell** shared libraries (already pulled in by most of the above compositors' package sets)
- A **Nerd Font** installed as a system font, so the Waybar icon glyph renders. CaskaydiaMono Nerd Font is the default in Omarchy and works out of the box.

It is distribution-agnostic. The instructions below use `cargo`, which works on Arch, Fedora, Debian/Ubuntu, NixOS, etc.

## Install

### Arch / Omarchy (AUR)

```sh
yay -S waycal
```

Or with any other AUR helper (`paru -S waycal`), or manually via `git clone https://aur.archlinux.org/waycal.git && cd waycal && makepkg -si`.

### Any distro with Rust installed

```sh
cargo install waycal
```

This pulls the latest release from [crates.io](https://crates.io/crates/waycal), builds it, and drops the binary into `~/.cargo/bin/waycal`. Make sure that directory is on your `$PATH`. You'll also need the GTK4 + `gtk4-layer-shell` development headers installed so cargo can link against them:

| Distro             | Install command                                                                   |
| ------------------ | --------------------------------------------------------------------------------- |
| Arch / Omarchy     | `sudo pacman -S --needed gtk4 gtk4-layer-shell pkgconf`                           |
| Fedora             | `sudo dnf install gtk4-devel gtk4-layer-shell-devel pkgconf`                      |
| Debian / Ubuntu    | `sudo apt install libgtk-4-dev libgtk4-layer-shell-dev pkg-config`                |

### Prebuilt binary

Each tagged release ships a prebuilt x86_64 Linux tarball on the [releases page](https://github.com/forrestknight/waycal/releases). Extract it and drop `waycal` anywhere on your `$PATH`.

### Build from source

```sh
git clone https://github.com/forrestknight/waycal.git
cd waycal
cargo build --release
install -Dm755 target/release/waycal ~/.local/bin/waycal
```

## Waybar integration

Add a custom module to your `~/.config/waybar/config.jsonc`:

```jsonc
"custom/waycal": {
  "format": "󰃭",
  "on-click": "pkill -x waycal || waycal",
  "tooltip-format": "Calendar"
}
```

Reference it in one of your `modules-*` lists, e.g. right after the clock:

```jsonc
"modules-center": ["clock", "custom/waycal", ...]
```

Optional styling in `~/.config/waybar/style.css`:

```css
#custom-waycal {
    background-color: @background;
    color: @foreground;
    padding: 0 10px;
    margin: 5px 0;
    border-radius: 16px;
    font-size: 12px;
}
#custom-waycal:hover {
    background-color: alpha(@background, 0.7);
}
```

Restart Waybar (`pkill -x waybar && setsid waybar &`) and click the icon.

## Controls

Without a config file (plain calendar):

| Key          | Action                                     |
| ------------ | ------------------------------------------ |
| `←` / `→`    | Previous / next month                      |
| `↑` / `↓`    | Previous / next year                       |
| `Enter`      | Jump back to today                         |
| `s`          | Toggle sharp / rounded style (persisted)   |
| `Esc` / `q`  | Close the popup                            |

With Google accounts configured (see below), arrow keys move the day selection instead:

| Key                | Action                                    |
| ------------------ | ----------------------------------------- |
| `←` / `→`          | Previous / next day                       |
| `↑` / `↓`          | Previous / next week                      |
| `PgUp` / `PgDn`    | Previous / next month (`Shift`: year)     |
| `Enter`            | Jump back to today                        |
| `n` / `t`          | New event / new task                      |
| `r`                | Refresh from Google                       |
| `s`                | Toggle sharp / rounded style (persisted)  |
| `Esc` / `q`        | Back / close the popup (`q` not in forms) |

Clicking the Waybar icon a second time also closes the popup (the `pkill -x waycal || waycal` command toggles).

## Google Calendar & Tasks integration

waycal can show — and edit — events and tasks from one or more Google accounts
through the [`gws` CLI](https://github.com/googleworkspace/cli). With accounts
configured, the popup grows a side panel: the selected day's agenda (with
join-Meet buttons) on top, pending tasks below, and buttons/keys to create,
edit, delete and complete items. Days with events get a small underline in the
month grid, and each account gets its own accent color.

Setup:

1. Install `gws` and authenticate each account once, e.g.:

   ```sh
   export GOOGLE_WORKSPACE_CLI_CLIENT_ID=""
   export GOOGLE_WORKSPACE_CLI_CLIENT_SECRET="
   gws auth login --services tasks,calendar --scopes https://www.googleapis.com/auth/tasks,https://www.googleapis.com/auth/calendar
   gws auth export --unmasked > ~/.config/gws-conf/work-credentials.json
   ```

2. Create `~/.config/waycal/config.toml`:

   ```toml
   poll_interval_secs = 300        # daemon poll interval
   default_reminder_mins = 10      # fallback when an event has no reminders
   task_digest_time = "09:00"      # daily due-tasks notification; omit to disable
   hide_event_types = ["workingLocation", "birthday"]

   [[accounts]]
   name = "personal"
   config_dir = "~/.config/gws-personal"
   credentials_file = "~/.config/gws-conf/personal-credentials.json"
   color = "#8FBC8F"

   [[accounts]]
   name = "work"
   config_dir = "~/.config/gws-work"
   credentials_file = "~/.config/gws-conf/work-credentials.json"
   color = "#7aa2f7"
   ```

   `config_dir` and `credentials_file` map to gws' `GOOGLE_WORKSPACE_CLI_CONFIG_DIR`
   and `GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE` environment variables — one pair per
   account. Without this file, waycal behaves exactly like the plain calendar above.

The popup paints instantly from a local cache (`~/.cache/waycal/`) and refreshes
in the background. `waycal dump` prints everything it would fetch, for debugging.

### Notifications

`waycal daemon` is a headless process that polls your accounts and sends
desktop notifications (via `notify-send`) for:

- **Event reminders**, honoring each event's own reminders and each calendar's
  default popup reminders, with `default_reminder_mins` as the fallback. The
  notification body includes the Meet link when there is one.
- **A daily digest** of tasks due (or overdue) today, at `task_digest_time`.

Start it with your session — either Hyprland:

```
exec-once = waycal daemon
```

or the bundled systemd user unit:

```sh
cp packaging/waycal-daemon.service ~/.config/systemd/user/
systemctl --user enable --now waycal-daemon.service
```

(Edit `ExecStart` if your binary isn't at `/usr/bin/waycal`.) Notification
dedup state lives in `~/.local/state/waycal/`, so restarting the daemon never
re-fires old reminders.

## Why not just use the Waybar clock tooltip?

The built-in `clock` tooltip shows a calendar, but it's an HTML label tooltip — not focusable, not keyboard-navigable, and shares the clock module's click action. waycal is a real window you can interact with, and leaves your clock's click behavior untouched.

## License

MIT.
