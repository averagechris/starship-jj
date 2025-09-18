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
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Style {
    /// Text Color
    pub color: Option<Color>,
    /// Background Color
    pub bg_color: Option<Color>,
}

impl Style {
    pub fn print(
        &self,
        io: &mut impl Write,
        fallback: impl Into<Option<Style>>,
    ) -> Result<(), CommandError> {
        write!(io, "\x1B[")?;

        let fallback = fallback.into().unwrap_or_default();

        if let Some(color) = self.color.or(fallback.color) {
            write!(io, "{}", colored::Color::from(color).to_fg_str())?;
        } else {
            write!(io, "39")?;
        }
        if let Some(color) = self.bg_color.or(fallback.bg_color) {
            write!(io, ";{}", colored::Color::from(color).to_bg_str())?;
        } else {
            write!(io, ";49")?;
        }

        write!(io, "m")?;
        Ok(())
    }

    pub fn format(&self, fallback: impl Into<Option<Style>>) -> String {
        let mut s = "\x1B[".to_string();

        let fallback = fallback.into().unwrap_or_default();

        if let Some(color) = self.color.or(fallback.color) {
            s.push_str(colored::Color::from(color).to_fg_str().as_ref());
        } else {
            s.push_str("39");
        }
        s.push(';');
        if let Some(color) = self.bg_color.or(fallback.bg_color) {
            s.push_str(colored::Color::from(color).to_bg_str().as_ref());
        } else {
            s.push_str("49");
        }

        s.push('m');
        s
    }
}

#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
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

impl From<Color> for colored::Color {
    fn from(value: Color) -> Self {
        match value {
            Color::Black => colored::Color::Black,
            Color::Red => colored::Color::Red,
            Color::Green => colored::Color::Green,
            Color::Yellow => colored::Color::Yellow,
            Color::Blue => colored::Color::Blue,
            Color::Magenta => colored::Color::Magenta,
            Color::Cyan => colored::Color::Cyan,
            Color::White => colored::Color::White,
            Color::BrightBlack => colored::Color::BrightBlack,
            Color::BrightRed => colored::Color::BrightRed,
            Color::BrightGreen => colored::Color::BrightGreen,
            Color::BrightYellow => colored::Color::BrightYellow,
            Color::BrightBlue => colored::Color::BrightBlue,
            Color::BrightMagenta => colored::Color::BrightMagenta,
            Color::BrightCyan => colored::Color::BrightCyan,
            Color::BrightWhite => colored::Color::BrightWhite,
            Color::TrueColor { r, g, b } => colored::Color::TrueColor { r, g, b },
        }
    }
}

impl From<colored::Color> for Color {
    fn from(value: colored::Color) -> Self {
        match value {
            colored::Color::Black => Color::Black,
            colored::Color::Red => Color::Red,
            colored::Color::Green => Color::Green,
            colored::Color::Yellow => Color::Yellow,
            colored::Color::Blue => Color::Blue,
            colored::Color::Magenta => Color::Magenta,
            colored::Color::Cyan => Color::Cyan,
            colored::Color::White => Color::White,
            colored::Color::BrightBlack => Color::BrightBlack,
            colored::Color::BrightRed => Color::BrightRed,
            colored::Color::BrightGreen => Color::BrightGreen,
            colored::Color::BrightYellow => Color::BrightYellow,
            colored::Color::BrightBlue => Color::BrightBlue,
            colored::Color::BrightMagenta => Color::BrightMagenta,
            colored::Color::BrightCyan => Color::BrightCyan,
            colored::Color::BrightWhite => Color::BrightWhite,
            colored::Color::TrueColor { r, g, b } => Color::TrueColor { r, g, b },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_print_defaults_are_reset_codes() {
        let mut out = Vec::new();
        Style::default().print(&mut out, None).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "\x1B[39;49m");
    }

    #[test]
    fn style_format_uses_fallback_when_fields_missing() {
        let s = Style::default();
        let fallback = Style {
            color: Some(Color::Red),
            bg_color: None,
        };
        let formatted = s.format(fallback);
        assert_eq!(formatted, "\x1B[31;49m");
    }

    #[test]
    fn glob_matches_and_invalid_patterns() {
        let g = Glob::try_from("r/*").unwrap();
        assert!(g.matches("r/x"));
        assert!(!g.matches("x/r"));
        assert!(Glob::try_from("[").is_err());
    }

    #[test]
    fn color_roundtrip_basic_and_truecolor() {
        // Basic color mapping via pattern match
        let c = Color::from(colored::Color::Red);
        match c {
            Color::Red => {}
            other => panic!("Unexpected color: {:?}", other),
        }
        // TrueColor roundtrip
        let cc: colored::Color = Color::TrueColor { r: 1, g: 2, b: 3 }.into();
        let back: Color = cc.into();
        match back {
            Color::TrueColor { r: 1, g: 2, b: 3 } => {}
            other => panic!("Unexpected color: {:?}", other),
        }
    }

    #[test]
    fn style_print_uses_self_and_fallback_colors() {
        let s = Style {
            color: Some(Color::Green),
            bg_color: None,
        };
        let fallback = Style {
            color: Some(Color::Red),
            bg_color: Some(Color::Red),
        };
        let mut out = Vec::new();
        s.print(&mut out, fallback).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "\x1B[32;41m");
    }

    #[test]
    fn style_format_prefers_self_over_fallback() {
        let s = Style {
            color: Some(Color::Blue),
            bg_color: Some(Color::BrightYellow),
        };
        let fallback = Style {
            color: Some(Color::Red),
            bg_color: Some(Color::Green),
        };
        let formatted = s.format(fallback);
        assert_eq!(formatted, "\x1B[34;103m");
    }

    #[test]
    fn glob_into_string_roundtrip() {
        let g = Glob::try_from("src/*.rs").unwrap();
        let s: String = g.into();
        assert_eq!(s, "src/*.rs");
    }
}
