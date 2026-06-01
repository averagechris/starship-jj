use glob::Pattern;
use jj_cli::command_error::CommandError;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(try_from = "&str", into = "String")]
pub struct Glob(glob::Pattern);
impl TryFrom<&str> for Glob {
    type Error = glob::PatternError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self(Pattern::new(value)?))
    }
}
impl From<Glob> for String {
    fn from(value: Glob) -> Self {
        value.0.as_str().to_string()
    }
}

impl Glob {
    pub fn matches(&self, haystack: &str) -> bool {
        self.0.matches(haystack)
    }
}

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Style {
    /// Text Color
    pub color: Option<Color>,
    /// Background Color
    pub bg_color: Option<Color>,
    /// Text attributes such as bold, italic, and underline.
    #[serde(flatten)]
    pub attributes: TextAttributes,
}

impl Style {
    fn merge_with_fallback(&self, fallback: Option<Self>) -> Self {
        let Some(fallback) = fallback else {
            return self.clone();
        };

        Self {
            color: self.color.or(fallback.color),
            bg_color: self.bg_color.or(fallback.bg_color),
            attributes: self.attributes.merge_with_fallback(fallback.attributes),
        }
    }

    pub fn print(
        &self,
        io: &mut impl Write,
        fallback: impl Into<Option<Style>>,
    ) -> Result<(), CommandError> {
        write!(io, "{}", self.format(fallback))?;
        Ok(())
    }

    pub fn format(&self, fallback: impl Into<Option<Style>>) -> String {
        let style = self.merge_with_fallback(fallback.into());
        let mut codes = vec!["0".to_string()];
        codes.push(
            style
                .color
                .map(|color| color.fg_code())
                .unwrap_or_else(|| "39".to_string()),
        );
        codes.push(
            style
                .bg_color
                .map(|color| color.bg_code())
                .unwrap_or_else(|| "49".to_string()),
        );
        codes.extend(style.attributes.enabled_codes());

        let mut s = "\x1B[".to_string();
        s.push_str(&codes.join(";"));
        s.push('m');
        s
    }
}

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug, Clone, Copy, Default)]
pub struct TextAttributes {
    #[serde(default)]
    bold: Option<bool>,
    #[serde(default)]
    dimmed: Option<bool>,
    #[serde(default)]
    italic: Option<bool>,
    #[serde(default)]
    underline: Option<bool>,
    #[serde(default)]
    blink: Option<bool>,
    #[serde(default)]
    reverse: Option<bool>,
    #[serde(default)]
    hidden: Option<bool>,
    #[serde(default)]
    strikethrough: Option<bool>,
}

impl TextAttributes {
    fn merge_with_fallback(self, fallback: Self) -> Self {
        Self {
            bold: self.bold.or(fallback.bold),
            dimmed: self.dimmed.or(fallback.dimmed),
            italic: self.italic.or(fallback.italic),
            underline: self.underline.or(fallback.underline),
            blink: self.blink.or(fallback.blink),
            reverse: self.reverse.or(fallback.reverse),
            hidden: self.hidden.or(fallback.hidden),
            strikethrough: self.strikethrough.or(fallback.strikethrough),
        }
    }

    fn enabled_codes(self) -> impl Iterator<Item = String> {
        [
            (self.bold, "1"),
            (self.dimmed, "2"),
            (self.italic, "3"),
            (self.underline, "4"),
            (self.blink, "5"),
            (self.reverse, "7"),
            (self.hidden, "8"),
            (self.strikethrough, "9"),
        ]
        .into_iter()
        .filter(|(enabled, _)| enabled.unwrap_or_default())
        .map(|(_, code)| code.to_string())
    }
}

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(try_from = "ColorConfig")]
#[allow(clippy::enum_variant_names)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    TrueColor { r: u8, g: u8, b: u8 },
}

impl Color {
    fn fg_code(self) -> String {
        match self {
            Color::Black => "30".to_string(),
            Color::Red => "31".to_string(),
            Color::Green => "32".to_string(),
            Color::Yellow => "33".to_string(),
            Color::Blue => "34".to_string(),
            Color::Magenta => "35".to_string(),
            Color::Cyan => "36".to_string(),
            Color::White => "37".to_string(),
            Color::BrightBlack => "90".to_string(),
            Color::BrightRed => "91".to_string(),
            Color::BrightGreen => "92".to_string(),
            Color::BrightYellow => "93".to_string(),
            Color::BrightBlue => "94".to_string(),
            Color::BrightMagenta => "95".to_string(),
            Color::BrightCyan => "96".to_string(),
            Color::BrightWhite => "97".to_string(),
            Color::TrueColor { r, g, b } => format!("38;2;{r};{g};{b}"),
        }
    }

    fn bg_code(self) -> String {
        match self {
            Color::Black => "40".to_string(),
            Color::Red => "41".to_string(),
            Color::Green => "42".to_string(),
            Color::Yellow => "43".to_string(),
            Color::Blue => "44".to_string(),
            Color::Magenta => "45".to_string(),
            Color::Cyan => "46".to_string(),
            Color::White => "47".to_string(),
            Color::BrightBlack => "100".to_string(),
            Color::BrightRed => "101".to_string(),
            Color::BrightGreen => "102".to_string(),
            Color::BrightYellow => "103".to_string(),
            Color::BrightBlue => "104".to_string(),
            Color::BrightMagenta => "105".to_string(),
            Color::BrightCyan => "106".to_string(),
            Color::BrightWhite => "107".to_string(),
            Color::TrueColor { r, g, b } => format!("48;2;{r};{g};{b}"),
        }
    }
}

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum ColorConfig {
    Color(TerminalColor),
    #[cfg_attr(
        feature = "json-schema",
        schemars(example = "#ff00ff", transform = hex_color)
    )]
    Hex(String),
}

#[cfg(feature = "json-schema")]
fn hex_color(schema: &mut schemars::Schema) {
    schema.ensure_object().insert(
        "pattern".to_string(),
        serde_json::Value::String(r"^#[0-9a-fA-F]{6}$".to_string()),
    );
}

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[cfg_attr(feature = "json-schema", schemars(inline))]
#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[allow(clippy::enum_variant_names)]
pub enum TerminalColor {
    #[serde(alias = "black")]
    Black,
    #[serde(alias = "red")]
    Red,
    #[serde(alias = "green")]
    Green,
    #[serde(alias = "yellow")]
    Yellow,
    #[serde(alias = "blue")]
    Blue,
    #[serde(alias = "magenta")]
    Magenta,
    #[serde(alias = "cyan")]
    Cyan,
    #[serde(alias = "white")]
    White,
    #[serde(alias = "bright_black")]
    BrightBlack,
    #[serde(alias = "bright_red")]
    BrightRed,
    #[serde(alias = "bright_green")]
    BrightGreen,
    #[serde(alias = "bright_yellow")]
    BrightYellow,
    #[serde(alias = "bright_blue")]
    BrightBlue,
    #[serde(alias = "bright_magenta")]
    BrightMagenta,
    #[serde(alias = "bright_cyan")]
    BrightCyan,
    #[serde(alias = "bright_white")]
    BrightWhite,
    #[serde(alias = "true_color")]
    TrueColor { r: u8, g: u8, b: u8 },
}

#[derive(Debug)]
pub enum HexColorError {
    MissingHash,
    WrongLength,
    InvalidCharacter(std::num::ParseIntError),
}

impl std::fmt::Display for HexColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHash => write!(f, "hex colors need to start with a #"),
            Self::WrongLength => write!(f, "hex colors must contain 6 hex digits"),
            Self::InvalidCharacter(err) => write!(f, "invalid hexadecimal color: {err}"),
        }
    }
}

impl std::error::Error for HexColorError {}

impl From<std::num::ParseIntError> for HexColorError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::InvalidCharacter(value)
    }
}

impl TryFrom<ColorConfig> for Color {
    type Error = HexColorError;

    fn try_from(value: ColorConfig) -> Result<Self, Self::Error> {
        match value {
            ColorConfig::Color(color) => Ok(color.into()),
            ColorConfig::Hex(hex) => parse_hex_color(&hex),
        }
    }
}

impl From<TerminalColor> for Color {
    fn from(value: TerminalColor) -> Self {
        match value {
            TerminalColor::Black => Color::Black,
            TerminalColor::Red => Color::Red,
            TerminalColor::Green => Color::Green,
            TerminalColor::Yellow => Color::Yellow,
            TerminalColor::Blue => Color::Blue,
            TerminalColor::Magenta => Color::Magenta,
            TerminalColor::Cyan => Color::Cyan,
            TerminalColor::White => Color::White,
            TerminalColor::BrightBlack => Color::BrightBlack,
            TerminalColor::BrightRed => Color::BrightRed,
            TerminalColor::BrightGreen => Color::BrightGreen,
            TerminalColor::BrightYellow => Color::BrightYellow,
            TerminalColor::BrightBlue => Color::BrightBlue,
            TerminalColor::BrightMagenta => Color::BrightMagenta,
            TerminalColor::BrightCyan => Color::BrightCyan,
            TerminalColor::BrightWhite => Color::BrightWhite,
            TerminalColor::TrueColor { r, g, b } => Color::TrueColor { r, g, b },
        }
    }
}

fn parse_hex_color(hex: &str) -> Result<Color, HexColorError> {
    if !hex.starts_with('#') {
        return Err(HexColorError::MissingHash);
    }
    if hex.len() != 7 {
        return Err(HexColorError::WrongLength);
    }

    let r = u8::from_str_radix(&hex[1..3], 16)?;
    let g = u8::from_str_radix(&hex[3..5], 16)?;
    let b = u8::from_str_radix(&hex[5..7], 16)?;

    Ok(Color::TrueColor { r, g, b })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_print_defaults_are_reset_codes() {
        let mut out = Vec::new();
        Style::default().print(&mut out, None).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "\x1B[0;39;49m");
    }

    #[test]
    fn style_format_uses_fallback_when_fields_missing() {
        let s = Style::default();
        let fallback = Style {
            color: Some(Color::Red),
            bg_color: None,
            ..Default::default()
        };
        let formatted = s.format(fallback);
        assert_eq!(formatted, "\x1B[0;31;49m");
    }

    #[test]
    fn glob_matches_and_invalid_patterns() {
        let g = Glob::try_from("r/*").unwrap();
        assert!(g.matches("r/x"));
        assert!(!g.matches("x/r"));
        assert!(Glob::try_from("[").is_err());
    }

    #[test]
    fn color_codes_basic_and_truecolor() {
        assert_eq!(Color::Red.fg_code(), "31");
        assert_eq!(Color::BrightYellow.bg_code(), "103");
        assert_eq!(
            Color::TrueColor { r: 1, g: 2, b: 3 }.fg_code(),
            "38;2;1;2;3"
        );
    }

    #[test]
    fn style_print_uses_self_and_fallback_colors() {
        let s = Style {
            color: Some(Color::Green),
            bg_color: None,
            ..Default::default()
        };
        let fallback = Style {
            color: Some(Color::Red),
            bg_color: Some(Color::Red),
            ..Default::default()
        };
        let mut out = Vec::new();
        s.print(&mut out, fallback).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "\x1B[0;32;41m");
    }

    #[test]
    fn style_format_prefers_self_over_fallback() {
        let s = Style {
            color: Some(Color::Blue),
            bg_color: Some(Color::BrightYellow),
            ..Default::default()
        };
        let fallback = Style {
            color: Some(Color::Red),
            bg_color: Some(Color::Green),
            ..Default::default()
        };
        let formatted = s.format(fallback);
        assert_eq!(formatted, "\x1B[0;34;103m");
    }

    #[test]
    fn glob_into_string_roundtrip() {
        let g = Glob::try_from("src/*.rs").unwrap();
        let s: String = g.into();
        assert_eq!(s, "src/*.rs");
    }

    #[derive(Deserialize)]
    struct ColorHolder {
        color: Color,
    }

    #[test]
    fn color_deserializes_snake_case_alias() {
        let holder: ColorHolder = toml::from_str(r##"color = "bright_blue""##).unwrap();
        assert!(matches!(holder.color, Color::BrightBlue));
    }

    #[test]
    fn color_deserializes_hex() {
        let holder: ColorHolder = toml::from_str(r##"color = "#0a1Bff""##).unwrap();
        assert!(matches!(
            holder.color,
            Color::TrueColor {
                r: 0x0a,
                g: 0x1b,
                b: 0xff
            }
        ));
    }

    #[test]
    fn hex_color_rejects_wrong_lengths_without_panicking() {
        assert!(matches!(
            parse_hex_color("#fff"),
            Err(HexColorError::WrongLength)
        ));
        assert!(matches!(
            parse_hex_color("#00112233"),
            Err(HexColorError::WrongLength)
        ));
    }

    #[test]
    fn style_format_includes_text_attributes() {
        let s = Style {
            color: Some(Color::Red),
            attributes: TextAttributes {
                bold: Some(true),
                underline: Some(true),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(s.format(None), "\x1B[0;31;49;1;4m");
    }
}
