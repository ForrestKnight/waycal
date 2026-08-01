use std::cell::Cell;
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::Edge;

const MARGIN: i32 = 8;

const VALID_VALUES: &str =
    "top-left, top, top-right, left, center, right, bottom-left, bottom, bottom-right";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl Position {
    pub const DEFAULT: Position = Position { top: true, bottom: false, left: false, right: false };

    fn parse(s: &str) -> Option<Position> {
        let (top, bottom, left, right) = match s {
            "top-left" => (true, false, true, false),
            "top" => (true, false, false, false),
            "top-right" => (true, false, false, true),
            "left" => (false, false, true, false),
            "center" => (false, false, false, false),
            "right" => (false, false, false, true),
            "bottom-left" => (false, true, true, false),
            "bottom" => (false, true, false, false),
            "bottom-right" => (false, true, false, true),
            _ => return None,
        };
        Some(Position { top, bottom, left, right })
    }

    pub fn anchors(&self) -> [(Edge, bool); 4] {
        [
            (Edge::Top, self.top),
            (Edge::Bottom, self.bottom),
            (Edge::Left, self.left),
            (Edge::Right, self.right),
        ]
    }

    pub fn margin(&self, edge: Edge) -> i32 {
        // Preserve the original flush-with-bar look for the default top anchor.
        if *self == Position::DEFAULT && edge == Edge::Top {
            return 0;
        }
        match edge {
            Edge::Top if self.top => MARGIN,
            Edge::Bottom if self.bottom => MARGIN,
            Edge::Left if self.left => MARGIN,
            Edge::Right if self.right => MARGIN,
            _ => 0,
        }
    }
}

/// Registers `--position` with GLib's option parser (so it shows up in
/// `--help`) and returns a cell that holds the parsed value once the
/// application's `handle-local-options` signal has fired.
pub fn install(app: &gtk4::Application) -> Rc<Cell<Position>> {
    app.add_main_option(
        "position",
        glib::Char::from(0u8),
        glib::OptionFlags::NONE,
        glib::OptionArg::String,
        &format!("Where the popup anchors on screen: {VALID_VALUES}"),
        Some("VALUE"),
    );

    let position = Rc::new(Cell::new(Position::DEFAULT));
    {
        let position = position.clone();
        app.connect_handle_local_options(move |_, options| {
            if let Some(value) = options.lookup::<String>("position").ok().flatten() {
                match Position::parse(&value) {
                    Some(pos) => position.set(pos),
                    None => eprintln!(
                        "waycal: invalid --position value '{value}', expected one of: {VALID_VALUES}. Falling back to 'top'."
                    ),
                }
            }
            -1
        });
    }
    position
}
