//! Built-in theme definitions loaded from embedded TOML files.
//!
//! This module provides 19 built-in themes organized by family:
//!
//! | Theme | Variants |
//! |-------|----------|
//! | Default | Dark, Light |
//! | Catppuccin | Latte, Frappe, Macchiato, Mocha |
//! | Dracula | Single |
//! | Gruvbox | Dark, Light |
//! | Nord | Single |
//! | Tokyo Night | Default, Storm, Light |
//! | Solarized | Dark, Light |
//! | One Dark | Single |
//! | Rose Pine | Default, Moon, Dawn |

use super::definition::{theme_from_definition, ThemeDefinition};
use super::types::Theme;
use super::ThemeName;

/// Parses a builtin TOML theme string into a [`Theme`].
fn load(toml_str: &str) -> Theme {
    let def: ThemeDefinition =
        toml::from_str(toml_str).expect("builtin theme TOML is invalid — this is a bug");
    theme_from_definition(&def)
}

/// Returns the theme for the given theme name.
#[must_use]
pub fn get(name: ThemeName) -> Theme {
    match name {
        ThemeName::Dark => load(include_str!("../themes/dark.toml")),
        ThemeName::Light => load(include_str!("../themes/light.toml")),
        ThemeName::CatppuccinLatte => load(include_str!("../themes/catppuccin-latte.toml")),
        ThemeName::CatppuccinFrappe => load(include_str!("../themes/catppuccin-frappe.toml")),
        ThemeName::CatppuccinMacchiato => load(include_str!("../themes/catppuccin-macchiato.toml")),
        ThemeName::CatppuccinMocha => load(include_str!("../themes/catppuccin-mocha.toml")),
        ThemeName::Dracula => load(include_str!("../themes/dracula.toml")),
        ThemeName::GruvboxDark => load(include_str!("../themes/gruvbox-dark.toml")),
        ThemeName::GruvboxLight => load(include_str!("../themes/gruvbox-light.toml")),
        ThemeName::Nord => load(include_str!("../themes/nord.toml")),
        ThemeName::TokyoNight => load(include_str!("../themes/tokyo-night.toml")),
        ThemeName::TokyoNightStorm => load(include_str!("../themes/tokyo-night-storm.toml")),
        ThemeName::TokyoNightLight => load(include_str!("../themes/tokyo-night-light.toml")),
        ThemeName::SolarizedDark => load(include_str!("../themes/solarized-dark.toml")),
        ThemeName::SolarizedLight => load(include_str!("../themes/solarized-light.toml")),
        ThemeName::OneDark => load(include_str!("../themes/one-dark.toml")),
        ThemeName::RosePine => load(include_str!("../themes/rose-pine.toml")),
        ThemeName::RosePineMoon => load(include_str!("../themes/rose-pine-moon.toml")),
        ThemeName::RosePineDawn => load(include_str!("../themes/rose-pine-dawn.toml")),
    }
}
