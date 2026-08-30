use clap::Parser;
use std::str::FromStr;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Position of the calendar: top, bottom, or x,y (e.g., 100,200)
    #[arg(long, default_value = "top")]
    pub position: Position,
}

#[derive(Debug, Clone)]
pub enum Position {
    Top,
    Bottom,
    Coordinates(i32, i32),
}

impl FromStr for Position {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "top" => Ok(Position::Top),
            "bottom" => Ok(Position::Bottom),
            other => {
                let parts: Vec<&str> = other.split(',').collect();
                if parts.len() == 2 {
                    let x = parts[0]
                        .trim()
                        .parse::<i32>()
                        .map_err(|_| format!("Invalid x coordinate: {}", parts[0]))?;
                    let y = parts[1]
                        .trim()
                        .parse::<i32>()
                        .map_err(|_| format!("Invalid y coordinate: {}", parts[1]))?;
                    Ok(Position::Coordinates(x, y))
                } else {
                    Err(format!(
                        "Invalid position: '{}'. Expected 'top', 'bottom', or 'x,y'",
                        other
                    ))
                }
            }
        }
    }
}
