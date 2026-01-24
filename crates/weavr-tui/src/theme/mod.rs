//! Theme system for the TUI.
//!
//! This module provides theming support with 19 built-in themes:
//!
//! - **Default**: Dark, Light
//! - **Catppuccin**: Latte, Frappe, Macchiato, Mocha
//! - **Dracula**
//! - **Gruvbox**: Dark, Light
//! - **Nord**
//! - **Tokyo Night**: Default, Storm, Light
//! - **Solarized**: Dark, Light
//! - **One Dark**
//! - **Rose Pine**: Default, Moon, Dawn
//!
//! # Example
//!
//! ```
//! use weavr_tui::theme::{ThemeName, Theme};
//!
//! let theme_name = ThemeName::default(); // Dark
//! let theme = Theme::from(theme_name);
//! ```

pub mod builtin;
mod types;

pub use types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

use std::fmt;
use std::str::FromStr;

/// Available theme names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeName {
    /// Default dark theme.
    #[default]
    Dark,
    /// Default light theme.
    Light,
    /// Catppuccin Latte (light).
    CatppuccinLatte,
    /// Catppuccin Frappe (medium dark).
    CatppuccinFrappe,
    /// Catppuccin Macchiato (dark).
    CatppuccinMacchiato,
    /// Catppuccin Mocha (darkest).
    CatppuccinMocha,
    /// Dracula theme.
    Dracula,
    /// Gruvbox dark theme.
    GruvboxDark,
    /// Gruvbox light theme.
    GruvboxLight,
    /// Nord theme.
    Nord,
    /// Tokyo Night theme.
    TokyoNight,
    /// Tokyo Night Storm theme.
    TokyoNightStorm,
    /// Tokyo Night Light theme.
    TokyoNightLight,
    /// Solarized Dark theme.
    SolarizedDark,
    /// Solarized Light theme.
    SolarizedLight,
    /// One Dark theme.
    OneDark,
    /// Rose Pine theme.
    RosePine,
    /// Rose Pine Moon theme.
    RosePineMoon,
    /// Rose Pine Dawn theme (light).
    RosePineDawn,
}

impl ThemeName {
    /// Returns all available theme names.
    #[must_use]
    pub const fn all() -> &'static [ThemeName] {
        &[
            ThemeName::Dark,
            ThemeName::Light,
            ThemeName::CatppuccinLatte,
            ThemeName::CatppuccinFrappe,
            ThemeName::CatppuccinMacchiato,
            ThemeName::CatppuccinMocha,
            ThemeName::Dracula,
            ThemeName::GruvboxDark,
            ThemeName::GruvboxLight,
            ThemeName::Nord,
            ThemeName::TokyoNight,
            ThemeName::TokyoNightStorm,
            ThemeName::TokyoNightLight,
            ThemeName::SolarizedDark,
            ThemeName::SolarizedLight,
            ThemeName::OneDark,
            ThemeName::RosePine,
            ThemeName::RosePineMoon,
            ThemeName::RosePineDawn,
        ]
    }

    /// Returns the string identifier for this theme.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::CatppuccinLatte => "catppuccin-latte",
            Self::CatppuccinFrappe => "catppuccin-frappe",
            Self::CatppuccinMacchiato => "catppuccin-macchiato",
            Self::CatppuccinMocha => "catppuccin-mocha",
            Self::Dracula => "dracula",
            Self::GruvboxDark => "gruvbox-dark",
            Self::GruvboxLight => "gruvbox-light",
            Self::Nord => "nord",
            Self::TokyoNight => "tokyo-night",
            Self::TokyoNightStorm => "tokyo-night-storm",
            Self::TokyoNightLight => "tokyo-night-light",
            Self::SolarizedDark => "solarized-dark",
            Self::SolarizedLight => "solarized-light",
            Self::OneDark => "one-dark",
            Self::RosePine => "rose-pine",
            Self::RosePineMoon => "rose-pine-moon",
            Self::RosePineDawn => "rose-pine-dawn",
        }
    }
}

impl fmt::Display for ThemeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing an invalid theme name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseThemeNameError {
    input: String,
}

impl fmt::Display for ParseThemeNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown theme name: '{}'", self.input)
    }
}

impl std::error::Error for ParseThemeNameError {}

impl FromStr for ThemeName {
    type Err = ParseThemeNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_lowercase().replace('_', "-");
        match normalized.as_str() {
            "dark" => Ok(Self::Dark),
            "light" => Ok(Self::Light),
            "catppuccin-latte" | "latte" => Ok(Self::CatppuccinLatte),
            "catppuccin-frappe" | "frappe" => Ok(Self::CatppuccinFrappe),
            "catppuccin-macchiato" | "macchiato" => Ok(Self::CatppuccinMacchiato),
            "catppuccin-mocha" | "mocha" => Ok(Self::CatppuccinMocha),
            "dracula" => Ok(Self::Dracula),
            "gruvbox-dark" | "gruvbox" => Ok(Self::GruvboxDark),
            "gruvbox-light" => Ok(Self::GruvboxLight),
            "nord" => Ok(Self::Nord),
            "tokyo-night" | "tokyonight" => Ok(Self::TokyoNight),
            "tokyo-night-storm" | "tokyonight-storm" => Ok(Self::TokyoNightStorm),
            "tokyo-night-light" | "tokyonight-light" => Ok(Self::TokyoNightLight),
            "solarized-dark" | "solarized" => Ok(Self::SolarizedDark),
            "solarized-light" => Ok(Self::SolarizedLight),
            "one-dark" | "onedark" => Ok(Self::OneDark),
            "rose-pine" | "rosepine" => Ok(Self::RosePine),
            "rose-pine-moon" | "rosepine-moon" => Ok(Self::RosePineMoon),
            "rose-pine-dawn" | "rosepine-dawn" => Ok(Self::RosePineDawn),
            _ => Err(ParseThemeNameError {
                input: s.to_string(),
            }),
        }
    }
}

impl From<ThemeName> for Theme {
    fn from(name: ThemeName) -> Self {
        builtin::get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_name_default_is_dark() {
        assert_eq!(ThemeName::default(), ThemeName::Dark);
    }

    #[test]
    fn theme_name_all_returns_19_themes() {
        assert_eq!(ThemeName::all().len(), 19);
    }

    #[test]
    fn theme_name_display() {
        assert_eq!(ThemeName::Dark.to_string(), "dark");
        assert_eq!(ThemeName::CatppuccinMocha.to_string(), "catppuccin-mocha");
        assert_eq!(ThemeName::TokyoNightStorm.to_string(), "tokyo-night-storm");
    }

    #[test]
    fn theme_name_from_str() {
        assert_eq!("dark".parse::<ThemeName>().unwrap(), ThemeName::Dark);
        assert_eq!("DARK".parse::<ThemeName>().unwrap(), ThemeName::Dark);
        assert_eq!(
            "catppuccin-mocha".parse::<ThemeName>().unwrap(),
            ThemeName::CatppuccinMocha
        );
        assert_eq!(
            "mocha".parse::<ThemeName>().unwrap(),
            ThemeName::CatppuccinMocha
        );
    }

    #[test]
    fn theme_name_from_str_with_underscores() {
        assert_eq!(
            "tokyo_night".parse::<ThemeName>().unwrap(),
            ThemeName::TokyoNight
        );
        assert_eq!(
            "rose_pine_moon".parse::<ThemeName>().unwrap(),
            ThemeName::RosePineMoon
        );
    }

    #[test]
    fn theme_name_from_str_invalid() {
        let result = "invalid".parse::<ThemeName>();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "unknown theme name: 'invalid'"
        );
    }

    #[test]
    fn theme_from_theme_name() {
        let theme = Theme::from(ThemeName::Dark);
        // Just verify it doesn't panic and returns a theme
        assert_eq!(
            theme.ui.border_focused,
            ratatui::style::Color::Rgb(255, 215, 0)
        );
    }

    #[test]
    fn all_themes_can_be_created() {
        for name in ThemeName::all() {
            let _theme = Theme::from(*name);
        }
    }

    #[test]
    fn theme_name_roundtrip() {
        for name in ThemeName::all() {
            let s = name.to_string();
            let parsed: ThemeName = s.parse().unwrap();
            assert_eq!(*name, parsed);
        }
    }
}
