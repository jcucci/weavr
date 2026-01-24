//! Built-in theme definitions.
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

pub mod catppuccin;
pub mod dark;
pub mod dracula;
pub mod gruvbox;
pub mod light;
pub mod nord;
pub mod one_dark;
pub mod rose_pine;
pub mod solarized;
pub mod tokyo_night;

use super::types::Theme;
use super::ThemeName;

/// Returns the theme for the given theme name.
#[must_use]
pub fn get(name: ThemeName) -> Theme {
    match name {
        ThemeName::Dark => dark::theme(),
        ThemeName::Light => light::theme(),
        ThemeName::CatppuccinLatte => catppuccin::latte(),
        ThemeName::CatppuccinFrappe => catppuccin::frappe(),
        ThemeName::CatppuccinMacchiato => catppuccin::macchiato(),
        ThemeName::CatppuccinMocha => catppuccin::mocha(),
        ThemeName::Dracula => dracula::theme(),
        ThemeName::GruvboxDark => gruvbox::dark(),
        ThemeName::GruvboxLight => gruvbox::light(),
        ThemeName::Nord => nord::theme(),
        ThemeName::TokyoNight => tokyo_night::night(),
        ThemeName::TokyoNightStorm => tokyo_night::storm(),
        ThemeName::TokyoNightLight => tokyo_night::light(),
        ThemeName::SolarizedDark => solarized::dark(),
        ThemeName::SolarizedLight => solarized::light(),
        ThemeName::OneDark => one_dark::theme(),
        ThemeName::RosePine => rose_pine::main(),
        ThemeName::RosePineMoon => rose_pine::moon(),
        ThemeName::RosePineDawn => rose_pine::dawn(),
    }
}
