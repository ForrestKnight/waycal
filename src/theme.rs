use std::path::PathBuf;

pub struct Palette {
    pub background: String,
    pub background_rgba: String,
    pub foreground: String,
    pub accent: String,
    pub accent_alpha: String,
    pub muted: String,
}

impl Palette {
    fn fallback() -> Self {
        Self {
            background: "#1a2125".into(),
            background_rgba: "rgba(26, 33, 37, 0.96)".into(),
            foreground: "#c9d1d9".into(),
            accent: "#8FBC8F".into(),
            accent_alpha: "rgba(143, 188, 143, 0.18)".into(),
            muted: "#6a7a71".into(),
        }
    }

    pub fn from_omarchy() -> Self {
        let Some(path) = omarchy_colors_path() else {
            return Self::fallback();
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::fallback();
        };

        let mut fb = Self::fallback();
        let mut background = None;
        let mut foreground = None;
        let mut accent = None;
        let mut muted = None;

        for line in text.lines() {
            let Some((key, value)) = parse_toml_string(line) else {
                continue;
            };
            match key {
                "background" => background = Some(value),
                "foreground" => foreground = Some(value),
                "accent" => accent = Some(value),
                "color8" => muted = Some(value),
                _ => {}
            }
        }

        if let Some(v) = background {
            fb.background_rgba =
                hex_to_rgba(&v, 0.96).unwrap_or_else(|| fb.background_rgba.clone());
            fb.background = v;
        }
        if let Some(v) = foreground {
            fb.foreground = v;
        }
        if let Some(v) = accent {
            fb.accent_alpha =
                hex_to_rgba(&v, 0.18).unwrap_or_else(|| fb.accent_alpha.clone());
            fb.accent = v;
        }
        if let Some(v) = muted {
            fb.muted = v;
        }
        fb
    }
}

pub fn build_css(p: &Palette) -> String {
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
    background-color: {bg_rgba};
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
    color: {bg};
    border-radius: 0;
    font-weight: bold;
}}
.waycal-root.rounded .waycal-day.today {{
    border-radius: 8px;
}}
.waycal-footer {{
    color: {muted};
    font-size: 10px;
    padding-top: 8px;
    margin-top: 6px;
    border-top: 1px solid {accent_alpha};
}}
"#,
        bg = p.background,
        bg_rgba = p.background_rgba,
        fg = p.foreground,
        accent = p.accent,
        accent_alpha = p.accent_alpha,
        muted = p.muted,
    )
}

fn omarchy_colors_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/omarchy/current/theme/colors.toml"))
}

fn parse_toml_string(line: &str) -> Option<(&str, String)> {
    let (key, rest) = line.trim().split_once('=')?;
    let key = key.trim();
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some((key, rest[..end].to_string()))
}

fn hex_to_rgba(hex: &str, alpha: f32) -> Option<String> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some(format!("rgba({r}, {g}, {b}, {alpha})"))
}
