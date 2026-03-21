//! Layout calculation for the three-pane TUI.
//!
//! The layout consists of:
//! - Title bar (1 line)
//! - Top row: Left and Right panes side by side
//! - Bottom row: Result pane
//! - Status bar (1 line)

use ratatui::layout::{Constraint, Layout, Rect};

use crate::LayoutConfig;

/// Areas for each UI component.
#[derive(Debug, Clone, Copy)]
pub struct PaneAreas {
    /// Title bar area at the top.
    pub title_bar: Rect,
    /// Left pane (ours).
    pub left_pane: Rect,
    /// Base pane (ancestor), present only when diff3 base display is enabled.
    pub base_pane: Option<Rect>,
    /// Right pane (theirs).
    pub right_pane: Rect,
    /// Result pane (merged output).
    pub result_pane: Rect,
    /// Status bar area at the bottom.
    pub status_bar: Rect,
}

/// Calculates the layout areas for the given terminal size and configuration.
///
/// The `config` parameter controls the top/bottom split ratio (default 60/40).
/// When `show_base` is true, the top row is split into 3 equal columns
/// (Left | Base | Right) instead of 2.
///
/// ```text
/// +------------------------------------------+
/// |              Title Bar                   |  <- Length(1)
/// +---------------------+--------------------+
/// |        Left         |       Right        |  <- top_ratio_percent (default 60%)
/// +---------------------+--------------------+
/// |                Result                    |  <- remaining (default 40%)
/// +------------------------------------------+
/// |              Status Bar                  |  <- Length(1)
/// +------------------------------------------+
/// ```
#[must_use]
pub fn calculate_layout(area: Rect, config: &LayoutConfig, show_base: bool) -> PaneAreas {
    // Vertical split: title, main, status
    let [title_bar, main_area, status_bar] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Split main area into top (left/right) and bottom (result) using config ratio
    let top_percent = config.top_ratio_percent;
    let bottom_percent = 100 - top_percent;
    let [top_row, result_pane] = Layout::vertical([
        Constraint::Percentage(top_percent),
        Constraint::Percentage(bottom_percent),
    ])
    .areas(main_area);

    if show_base {
        // 3-column layout: Left | Base | Right
        let [left_pane, base_pane, right_pane] = Layout::horizontal([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .areas(top_row);

        PaneAreas {
            title_bar,
            left_pane,
            base_pane: Some(base_pane),
            right_pane,
            result_pane,
            status_bar,
        }
    } else {
        // 2-column layout: Left | Right
        let [left_pane, right_pane] =
            Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).areas(top_row);

        PaneAreas {
            title_bar,
            left_pane,
            base_pane: None,
            right_pane,
            result_pane,
            status_bar,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> LayoutConfig {
        LayoutConfig::default()
    }

    #[test]
    fn calculate_layout_returns_non_zero_areas() {
        let area = Rect::new(0, 0, 80, 24);
        let areas = calculate_layout(area, &default_config(), false);

        // All areas should have non-zero dimensions
        assert!(areas.title_bar.width > 0);
        assert!(areas.title_bar.height > 0);
        assert!(areas.left_pane.width > 0);
        assert!(areas.left_pane.height > 0);
        assert!(areas.right_pane.width > 0);
        assert!(areas.right_pane.height > 0);
        assert!(areas.result_pane.width > 0);
        assert!(areas.result_pane.height > 0);
        assert!(areas.status_bar.width > 0);
        assert!(areas.status_bar.height > 0);
    }

    #[test]
    fn title_and_status_bars_are_one_line() {
        let area = Rect::new(0, 0, 80, 24);
        let areas = calculate_layout(area, &default_config(), false);

        assert_eq!(areas.title_bar.height, 1);
        assert_eq!(areas.status_bar.height, 1);
    }

    #[test]
    fn left_and_right_are_side_by_side() {
        let area = Rect::new(0, 0, 80, 24);
        let areas = calculate_layout(area, &default_config(), false);

        // Left and right should have the same y position
        assert_eq!(areas.left_pane.y, areas.right_pane.y);
        // Left should be to the left of right
        assert!(areas.left_pane.x < areas.right_pane.x);
    }

    #[test]
    fn result_is_below_left_and_right() {
        let area = Rect::new(0, 0, 80, 24);
        let areas = calculate_layout(area, &default_config(), false);

        // Result should be below both left and right panes
        assert!(areas.result_pane.y > areas.left_pane.y);
        assert!(areas.result_pane.y > areas.right_pane.y);
    }

    #[test]
    fn result_spans_full_width() {
        let area = Rect::new(0, 0, 80, 24);
        let areas = calculate_layout(area, &default_config(), false);

        // Result pane should span the full width
        assert_eq!(areas.result_pane.width, area.width);
    }

    #[test]
    fn handles_minimum_terminal_size() {
        // Very small terminal
        let area = Rect::new(0, 0, 10, 5);
        let areas = calculate_layout(area, &default_config(), false);

        // Should not panic
        let _ = areas;
    }

    #[test]
    fn respects_custom_ratio() {
        let area = Rect::new(0, 0, 80, 24);
        let config = LayoutConfig {
            top_ratio_percent: 70,
        };
        let areas = calculate_layout(area, &config, false);

        // Main area is 22 lines (24 - title - status)
        // Top should be ~70% = ~15 lines, bottom ~30% = ~7 lines
        let main_height = 22;
        let expected_top = (main_height * 70) / 100;
        let expected_bottom = main_height - expected_top;

        assert_eq!(areas.left_pane.height, expected_top);
        assert_eq!(areas.result_pane.height, expected_bottom);
    }

    #[test]
    fn base_pane_none_when_hidden() {
        let area = Rect::new(0, 0, 80, 24);
        let areas = calculate_layout(area, &default_config(), false);
        assert!(areas.base_pane.is_none());
    }

    #[test]
    fn base_pane_present_when_shown() {
        let area = Rect::new(0, 0, 90, 24);
        let areas = calculate_layout(area, &default_config(), true);
        let base = areas.base_pane.expect("base pane should be present");
        assert!(base.width > 0);
        assert!(base.height > 0);
        // Base should be between left and right
        assert!(base.x > areas.left_pane.x);
        assert!(base.x < areas.right_pane.x);
    }

    #[test]
    fn three_column_layout_same_height() {
        let area = Rect::new(0, 0, 90, 24);
        let areas = calculate_layout(area, &default_config(), true);
        let base = areas.base_pane.unwrap();
        assert_eq!(areas.left_pane.y, base.y);
        assert_eq!(base.y, areas.right_pane.y);
        assert_eq!(areas.left_pane.height, base.height);
        assert_eq!(base.height, areas.right_pane.height);
    }
}
