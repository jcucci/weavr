//! Custom keybinding support.
//!
//! This module provides configurable keybinding mappings for normal mode.
//! Users can override default bindings via the `[keybindings]` section
//! in their TOML config. Dialog mode and command mode keybindings remain
//! hardcoded.

mod action;
mod map;
mod notation;

pub use action::Action;
pub use map::{build_from_config, KeybindingMap};
pub use notation::{display_key_notation, parse_key_notation, KeyInput, KeyNotationError};
