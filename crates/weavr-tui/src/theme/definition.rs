//! Data-driven theme definitions deserializable from TOML.
//!
//! [`ThemeDefinition`] is the TOML-friendly counterpart of [`Theme`](super::Theme).
//! Colors are represented as `"#RRGGBB"` hex strings; styles are objects with
//! optional `fg` and `bg` hex-color fields.

use ratatui::style::{Color, Style};
use serde::Deserialize;

use super::types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

// ---------------------------------------------------------------------------
// Hex color
// ---------------------------------------------------------------------------

/// An RGB color parsed from a `"#RRGGBB"` hex string.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HexColor(Color);

impl HexColor {
    fn into_color(self) -> Color {
        self.0
    }
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_hex_color(&s)
            .map(HexColor)
            .map_err(serde::de::Error::custom)
    }
}

/// Converts a single hex digit byte to its numeric value.
fn hex_nibble(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err("non-hex digit".to_string()),
    }
}

/// Parses a `"#RRGGBB"` string into a [`Color::Rgb`].
///
/// Uses byte-level parsing to avoid panics on non-ASCII input.
fn parse_hex_color(s: &str) -> Result<Color, String> {
    let s = s
        .strip_prefix('#')
        .ok_or_else(|| format!("color must start with '#', got '{s}'"))?;
    let bytes = s.as_bytes();
    if bytes.len() != 6 {
        return Err(format!(
            "expected 6 hex digits after '#', got {}",
            bytes.len()
        ));
    }

    let parse_component = |hi: u8, lo: u8, name: &str| -> Result<u8, String> {
        let hi = hex_nibble(hi).map_err(|e| format!("invalid {name}: {e}"))?;
        let lo = hex_nibble(lo).map_err(|e| format!("invalid {name}: {e}"))?;
        Ok((hi << 4) | lo)
    };

    let r = parse_component(bytes[0], bytes[1], "red")?;
    let g = parse_component(bytes[2], bytes[3], "green")?;
    let b = parse_component(bytes[4], bytes[5], "blue")?;
    Ok(Color::Rgb(r, g, b))
}

// ---------------------------------------------------------------------------
// Style definition
// ---------------------------------------------------------------------------

/// A style with optional foreground and background hex colors.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleDef {
    pub fg: Option<HexColor>,
    pub bg: Option<HexColor>,
}

impl StyleDef {
    fn into_style(self) -> Style {
        let mut style = Style::default();
        if let Some(fg) = self.fg {
            style = style.fg(fg.into_color());
        }
        if let Some(bg) = self.bg {
            style = style.bg(bg.into_color());
        }
        style
    }
}

// ---------------------------------------------------------------------------
// Theme definition (TOML schema)
// ---------------------------------------------------------------------------

/// A complete theme definition deserializable from TOML.
///
/// This mirrors the structure of [`Theme`] but uses hex-color strings
/// instead of `ratatui` types, making it suitable for data files.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeDefinition {
    /// Base color palette.
    pub base: BaseDef,
    /// Conflict visualization colors.
    pub conflict: ConflictDef,
    /// Diff visualization colors.
    pub diff: DiffDef,
    /// UI element colors.
    pub ui: UiDef,
}

/// Base color palette definition.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseDef {
    pub background: HexColor,
    pub foreground: HexColor,
    pub muted: HexColor,
    pub accent: HexColor,
    pub secondary: HexColor,
}

/// Conflict colors definition.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConflictDef {
    pub left: StyleDef,
    pub right: StyleDef,
    pub both: StyleDef,
    pub base: StyleDef,
    pub unresolved: StyleDef,
    pub resolved: StyleDef,
}

/// Diff colors definition.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffDef {
    pub added: StyleDef,
    pub removed: StyleDef,
    pub modified: StyleDef,
    pub context: StyleDef,
}

/// UI colors definition.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiDef {
    pub border_focused: HexColor,
    pub border_unfocused: HexColor,
    pub title: StyleDef,
    pub status: StyleDef,
    pub selection: StyleDef,
}

// ---------------------------------------------------------------------------
// Conversion
// ---------------------------------------------------------------------------

/// Converts a [`ThemeDefinition`] into a [`Theme`].
#[must_use]
pub fn theme_from_definition(def: &ThemeDefinition) -> Theme {
    let base = ColorPalette::new(
        def.base.background.into_color(),
        def.base.foreground.into_color(),
        def.base.muted.into_color(),
        def.base.accent.into_color(),
        def.base.secondary.into_color(),
    );

    let conflict = ConflictColors::new(
        def.conflict.left.clone().into_style(),
        def.conflict.right.clone().into_style(),
        def.conflict.both.clone().into_style(),
        def.conflict.base.clone().into_style(),
        def.conflict.unresolved.clone().into_style(),
        def.conflict.resolved.clone().into_style(),
    );

    let diff = DiffColors::new(
        def.diff.added.clone().into_style(),
        def.diff.removed.clone().into_style(),
        def.diff.modified.clone().into_style(),
        def.diff.context.clone().into_style(),
    );

    let ui = UiColors::new(
        def.ui.border_focused.into_color(),
        def.ui.border_unfocused.into_color(),
        def.ui.title.clone().into_style(),
        def.ui.status.clone().into_style(),
        def.ui.selection.clone().into_style(),
    );

    Theme::new(base, conflict, diff, ui)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TOML: &str = r##"
[base]
background = "#2E3440"
foreground = "#D8DEE9"
muted = "#4C566A"
accent = "#EBCB8B"
secondary = "#88C0D0"

[conflict]
left = { fg = "#88C0D0" }
right = { fg = "#D08770" }
both = { fg = "#A3BE8C" }
base = { fg = "#B48EAD" }
unresolved = { fg = "#BF616A" }
resolved = { fg = "#A3BE8C" }

[diff]
added = { fg = "#A3BE8C", bg = "#3B4252" }
removed = { fg = "#BF616A", bg = "#3B4252" }
modified = { fg = "#EBCB8B", bg = "#3B4252" }
context = { fg = "#4C566A" }

[ui]
border_focused = "#EBCB8B"
border_unfocused = "#434C5E"
title = { fg = "#88C0D0" }
status = { fg = "#4C566A" }
selection = { fg = "#ECEFF4", bg = "#434C5E" }
"##;

    #[test]
    fn deserialize_theme_definition() {
        let def: ThemeDefinition = toml::from_str(SAMPLE_TOML).unwrap();
        assert_eq!(def.base.background.into_color(), Color::Rgb(46, 52, 64));
        assert_eq!(def.base.accent.into_color(), Color::Rgb(235, 203, 139));
    }

    #[test]
    fn theme_from_definition_produces_valid_theme() {
        let def: ThemeDefinition = toml::from_str(SAMPLE_TOML).unwrap();
        let theme = theme_from_definition(&def);
        assert_eq!(theme.base.background, Color::Rgb(46, 52, 64));
        assert_eq!(theme.ui.border_focused, Color::Rgb(235, 203, 139));
    }

    #[test]
    fn hex_color_parsing_valid() {
        assert_eq!(parse_hex_color("#000000").unwrap(), Color::Rgb(0, 0, 0));
        assert_eq!(
            parse_hex_color("#FFFFFF").unwrap(),
            Color::Rgb(255, 255, 255)
        );
        assert_eq!(parse_hex_color("#ff0000").unwrap(), Color::Rgb(255, 0, 0));
    }

    #[test]
    fn hex_color_parsing_invalid() {
        assert!(parse_hex_color("000000").is_err());
        assert!(parse_hex_color("#0000").is_err());
        assert!(parse_hex_color("#GGGGGG").is_err());
    }

    #[test]
    fn style_def_fg_only() {
        let style = StyleDef {
            fg: Some(HexColor(Color::Rgb(255, 0, 0))),
            bg: None,
        };
        let result = style.into_style();
        assert_eq!(result, Style::default().fg(Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn style_def_fg_and_bg() {
        let style = StyleDef {
            fg: Some(HexColor(Color::Rgb(255, 0, 0))),
            bg: Some(HexColor(Color::Rgb(0, 0, 255))),
        };
        let result = style.into_style();
        assert_eq!(
            result,
            Style::default()
                .fg(Color::Rgb(255, 0, 0))
                .bg(Color::Rgb(0, 0, 255))
        );
    }
}
