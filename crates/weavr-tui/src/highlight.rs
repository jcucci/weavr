//! Syntax highlighting for conflict panes.
//!
//! Uses syntect to tokenize source code and map syntax colors into ratatui
//! `Style` values. Only foreground colors are emitted so that diff background
//! colors always show through.

use std::path::PathBuf;

use syntect::highlighting::{Theme as SyntectTheme, ThemeSet};
use syntect::parsing::SyntaxSet;
use weavr_core::Language;

/// Syntax highlighter backed by syntect.
///
/// Loads the default syntax set and a single theme once, then reuses them
/// for every highlight call.
pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: SyntectTheme,
}

/// A fully-highlighted document — one entry per source line.
pub struct HighlightedDocument {
    lines: Vec<Vec<(syntect::highlighting::Style, String)>>,
}

/// Caches highlighted documents for the current file so we avoid
/// re-highlighting on every frame.
pub struct HighlightCache {
    /// The path this cache was built for.
    pub path: PathBuf,
    /// Highlighted left (ours) content.
    pub left: Option<HighlightedDocument>,
    /// Highlighted right (theirs) content.
    pub right: Option<HighlightedDocument>,
    /// Highlighted base (ancestor) content.
    pub base: Option<HighlightedDocument>,
    /// Highlighted result content (for resolved/clean regions).
    pub result: Option<HighlightedDocument>,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter {
    /// Creates a new highlighter with the default syntax definitions and theme.
    ///
    /// Prefers "base16-ocean.dark" from the bundled theme set. Falls back to
    /// the first available theme, then to a default `Theme` instance.
    #[must_use]
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let mut theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .remove("base16-ocean.dark")
            .or_else(|| {
                theme_set
                    .themes
                    .keys()
                    .next()
                    .cloned()
                    .and_then(|k| theme_set.themes.remove(&k))
            })
            .unwrap_or_default();
        Self { syntax_set, theme }
    }

    /// Highlights the given text using the syntax for `lang`.
    ///
    /// Returns `None` for `Language::Unknown` or if no matching syntax is found.
    #[must_use]
    pub fn highlight(&self, text: &str, lang: Language) -> Option<HighlightedDocument> {
        let syntax_name = syntect_syntax_name(lang)?;
        let syntax = self.syntax_set.find_syntax_by_name(syntax_name)?;
        let mut h = syntect::easy::HighlightLines::new(syntax, &self.theme);
        let mut lines = Vec::new();
        let mut with_nl = String::new();
        for line in text.lines() {
            // `line` lacks a trailing newline, but syntect expects one for
            // correct state tracking. Append it for highlighting then strip
            // the newline from the last token.
            with_nl.clear();
            with_nl.push_str(line);
            with_nl.push('\n');
            let ranges = h
                .highlight_line(&with_nl, &self.syntax_set)
                .unwrap_or_default();
            // Strip trailing newline from the last span
            let mut spans: Vec<(syntect::highlighting::Style, String)> = ranges
                .into_iter()
                .map(|(style, s)| (style, s.to_string()))
                .collect();
            if let Some(last) = spans.last_mut() {
                if last.1.ends_with('\n') {
                    last.1.pop();
                }
            }
            lines.push(spans);
        }
        Some(HighlightedDocument { lines })
    }
}

impl HighlightedDocument {
    /// Returns the highlighted spans for a given 0-based line index.
    #[must_use]
    pub fn get_line_spans(
        &self,
        line_idx: usize,
    ) -> Option<&[(syntect::highlighting::Style, String)]> {
        self.lines.get(line_idx).map(Vec::as_slice)
    }
}

/// Maps a `Language` variant to the corresponding syntect syntax name.
///
/// Returns `None` for `Language::Unknown`.
#[must_use]
fn syntect_syntax_name(lang: Language) -> Option<&'static str> {
    match lang {
        Language::Rust => Some("Rust"),
        Language::CSharp => Some("C#"),
        Language::TypeScript => Some("TypeScript"),
        Language::JavaScript => Some("JavaScript"),
        Language::Go => Some("Go"),
        Language::Python => Some("Python"),
        Language::Ruby => Some("Ruby"),
        Language::Java => Some("Java"),
        Language::Kotlin => Some("Kotlin"),
        Language::Swift => Some("Swift"),
        Language::C => Some("C"),
        Language::Cpp => Some("C++"),
        Language::Json => Some("JSON"),
        Language::Yaml => Some("YAML"),
        Language::Toml => Some("TOML"),
        Language::Markdown => Some("Markdown"),
        Language::Unknown => None,
    }
}

/// Converts a syntect `Style` to a ratatui `Style`.
///
/// Only the foreground color is mapped; background is left unset so that
/// diff coloring (added/removed backgrounds) always shows through.
#[must_use]
pub fn syntect_style_to_ratatui(style: syntect::highlighting::Style) -> ratatui::style::Style {
    let fg = style.foreground;
    ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(fg.r, fg.g, fg.b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlighter_new_does_not_panic() {
        let _h = Highlighter::new();
    }

    #[test]
    fn highlight_rust_returns_some() {
        let h = Highlighter::new();
        let doc = h.highlight("fn main() {}\n", Language::Rust);
        assert!(doc.is_some());
    }

    #[test]
    fn highlight_unknown_returns_none() {
        let h = Highlighter::new();
        let doc = h.highlight("hello world", Language::Unknown);
        assert!(doc.is_none());
    }

    #[test]
    fn highlighted_document_line_count() {
        let h = Highlighter::new();
        let doc = h
            .highlight("fn main() {\n    println!(\"hi\");\n}\n", Language::Rust)
            .unwrap();
        assert_eq!(doc.lines.len(), 3);
    }

    #[test]
    fn get_line_spans_out_of_bounds_returns_none() {
        let h = Highlighter::new();
        let doc = h.highlight("let x = 1;", Language::Rust).unwrap();
        assert!(doc.get_line_spans(100).is_none());
    }

    #[test]
    fn syntect_style_to_ratatui_sets_fg() {
        let style = syntect::highlighting::Style {
            foreground: syntect::highlighting::Color {
                r: 255,
                g: 128,
                b: 0,
                a: 255,
            },
            ..Default::default()
        };
        let ratatui_style = syntect_style_to_ratatui(style);
        assert_eq!(
            ratatui_style.fg,
            Some(ratatui::style::Color::Rgb(255, 128, 0))
        );
    }

    #[test]
    fn syntect_syntax_name_known_languages() {
        assert_eq!(syntect_syntax_name(Language::Rust), Some("Rust"));
        assert_eq!(syntect_syntax_name(Language::CSharp), Some("C#"));
        assert_eq!(syntect_syntax_name(Language::Go), Some("Go"));
        assert_eq!(syntect_syntax_name(Language::Unknown), None);
    }
}
