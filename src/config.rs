use std::path::PathBuf;
use serde::{Deserialize, Deserializer, de};

// ---------------------------------------------------------------------------
// Public runtime types
// ---------------------------------------------------------------------------

pub struct Config {
    pub notes_dir: PathBuf,
    pub theme: Theme,
}

pub struct Theme {
    pub heading:     ElementStyle,
    pub open_todo:   ElementStyle,
    pub in_progress: ElementStyle,
    pub done:        ElementStyle,
    pub code:        ElementStyle,
    pub mention:     ElementStyle,
    pub tag:         ElementStyle,
}

#[derive(Clone)]
pub struct ElementStyle {
    pub color:  Option<ThemeColor>,
    pub styles: Vec<TextStyle>,
}

#[derive(Clone)]
pub enum ThemeColor {
    Named(NamedColor),
    Rgb { r: u8, g: u8, b: u8 },
}

#[derive(Clone, Copy)]
pub enum NamedColor {
    Black, Red, Green, Yellow, Blue, Magenta, Cyan, White,
    DarkGrey, DarkRed, DarkGreen, DarkYellow, DarkBlue, DarkMagenta, DarkCyan,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TextStyle { Bold, Underline, Italic, Dimmed }

// ---------------------------------------------------------------------------
// Deserialization layer
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LoadedConfig {
    #[serde(default)]
    theme: LoadedTheme,
}

#[derive(Deserialize, Default)]
struct LoadedTheme {
    heading:     Option<LoadedElementStyle>,
    open_todo:   Option<LoadedElementStyle>,
    in_progress: Option<LoadedElementStyle>,
    done:        Option<LoadedElementStyle>,
    code:        Option<LoadedElementStyle>,
    mention:     Option<LoadedElementStyle>,
    tag:         Option<LoadedElementStyle>,
}

#[derive(Deserialize, Default)]
struct LoadedElementStyle {
    color:  Option<ThemeColor>,
    styles: Option<Vec<TextStyle>>,
}

impl<'de> Deserialize<'de> for ThemeColor {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if let Some(hex) = s.strip_prefix('#') {
            if hex.len() != 6 {
                return Err(de::Error::custom(format!(
                    "hex color must be 6 digits, e.g. #ffc9c9, got: #{hex}"
                )));
            }
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(de::Error::custom)?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(de::Error::custom)?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(de::Error::custom)?;
            return Ok(ThemeColor::Rgb { r, g, b });
        }
        let named = match s.as_str() {
            "black"        => NamedColor::Black,
            "red"          => NamedColor::Red,
            "green"        => NamedColor::Green,
            "yellow"       => NamedColor::Yellow,
            "blue"         => NamedColor::Blue,
            "magenta"      => NamedColor::Magenta,
            "cyan"         => NamedColor::Cyan,
            "white"        => NamedColor::White,
            "dark_grey"    => NamedColor::DarkGrey,
            "dark_red"     => NamedColor::DarkRed,
            "dark_green"   => NamedColor::DarkGreen,
            "dark_yellow"  => NamedColor::DarkYellow,
            "dark_blue"    => NamedColor::DarkBlue,
            "dark_magenta" => NamedColor::DarkMagenta,
            "dark_cyan"    => NamedColor::DarkCyan,
            other          => return Err(de::Error::custom(format!("unknown color: {other}"))),
        };
        Ok(ThemeColor::Named(named))
    }
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

impl Default for Theme {
    fn default() -> Self {
        Theme {
            heading:     ElementStyle { color: None,                                          styles: vec![TextStyle::Bold] },
            open_todo:   ElementStyle { color: None,                                          styles: vec![] },
            in_progress: ElementStyle { color: Some(ThemeColor::Named(NamedColor::Yellow)),   styles: vec![] },
            done:        ElementStyle { color: Some(ThemeColor::Named(NamedColor::DarkGrey)), styles: vec![TextStyle::Dimmed] },
            code:        ElementStyle { color: None,                                          styles: vec![TextStyle::Bold] },
            mention:     ElementStyle { color: Some(ThemeColor::Named(NamedColor::Magenta)),  styles: vec![] },
            tag:         ElementStyle { color: Some(ThemeColor::Named(NamedColor::Cyan)),     styles: vec![] },
        }
    }
}

// ---------------------------------------------------------------------------
// Merge
// ---------------------------------------------------------------------------

fn merge_element(loaded: Option<LoadedElementStyle>, default: ElementStyle) -> ElementStyle {
    match loaded {
        None    => default,
        Some(l) => ElementStyle {
            color:  l.color.or(default.color),
            styles: l.styles.unwrap_or(default.styles),
        },
    }
}

fn merge_theme(loaded: LoadedTheme) -> Theme {
    let d = Theme::default();
    Theme {
        heading:     merge_element(loaded.heading,     d.heading),
        open_todo:   merge_element(loaded.open_todo,   d.open_todo),
        in_progress: merge_element(loaded.in_progress, d.in_progress),
        done:        merge_element(loaded.done,         d.done),
        code:        merge_element(loaded.code,         d.code),
        mention:     merge_element(loaded.mention,      d.mention),
        tag:         merge_element(loaded.tag,          d.tag),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn load_config() -> anyhow::Result<Config> {
    let notes_dir = notes_dir();
    let theme = load_theme()?;
    Ok(Config { notes_dir, theme })
}

fn notes_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TODONE_DIR") {
        return PathBuf::from(dir);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".todone").join("notes")
}

fn load_theme() -> anyhow::Result<Theme> {
    let path = config_path();
    if !path.exists() {
        return Ok(Theme::default());
    }
    let text = std::fs::read_to_string(&path)?;
    let loaded: LoadedConfig = serde_saphyr::from_str(&text)
        .map_err(|e| anyhow::anyhow!("invalid config at {}: {e}", path.display()))?;
    Ok(merge_theme(loaded.theme))
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("todone").join("config.yaml")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_color_parses() {
        let style: LoadedElementStyle = serde_saphyr::from_str("color: \"#ff8800\"").unwrap();
        assert!(matches!(style.color, Some(ThemeColor::Rgb { r: 255, g: 136, b: 0 })));
    }

    #[test]
    fn named_color_parses() {
        let style: LoadedElementStyle = serde_saphyr::from_str("color: magenta").unwrap();
        assert!(matches!(style.color, Some(ThemeColor::Named(NamedColor::Magenta))));
    }

    #[test]
    fn invalid_color_errors() {
        let result: Result<LoadedElementStyle, _> = serde_saphyr::from_str("color: notacolor");
        assert!(result.is_err());
    }

    #[test]
    fn bad_hex_errors() {
        let result: Result<LoadedElementStyle, _> = serde_saphyr::from_str("color: \"#gg0000\"");
        assert!(result.is_err());
    }

    #[test]
    fn styles_parse() {
        let style: LoadedElementStyle =
            serde_saphyr::from_str("styles: [bold, dimmed]").unwrap();
        assert_eq!(style.styles, Some(vec![TextStyle::Bold, TextStyle::Dimmed]));
    }

    #[test]
    fn merge_uses_loaded_values() {
        let loaded = LoadedElementStyle {
            color: Some(ThemeColor::Named(NamedColor::Red)),
            styles: Some(vec![TextStyle::Underline]),
        };
        let default = ElementStyle {
            color: Some(ThemeColor::Named(NamedColor::Cyan)),
            styles: vec![TextStyle::Bold],
        };
        let merged = merge_element(Some(loaded), default);
        assert!(matches!(merged.color, Some(ThemeColor::Named(NamedColor::Red))));
        assert_eq!(merged.styles, vec![TextStyle::Underline]);
    }

    #[test]
    fn merge_falls_back_to_default() {
        let default = ElementStyle {
            color: Some(ThemeColor::Named(NamedColor::Yellow)),
            styles: vec![TextStyle::Bold],
        };
        let merged = merge_element(None, default);
        assert!(matches!(merged.color, Some(ThemeColor::Named(NamedColor::Yellow))));
        assert_eq!(merged.styles, vec![TextStyle::Bold]);
    }
}
