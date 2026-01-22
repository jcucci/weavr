//! Layout calculation for the three-pane TUI.
//!
//! The layout consists of:
//! - Title bar (1 line)
//! - Top row: Left and Right panes side by side
//! - Bottom row: Result pane
//! - Status bar (1 line)

use ratatui::layout::{Constraint, Layout, Rect};

/// Areas for each UI component.
#[derive(Debug, Clone, Copy)]
pub struct PaneAreas {
    /// Title bar area at the top.
    pub title_bar: Rect,
    /// Left pane (ours).
    pub left_pane: Rect,
    /// Right pane (theirs).
    pub right_pane: Rect,
    /// Result pane (merged output).
    pub result_pane: Rect,
    /// Status bar area at the bottom.
    pub status_bar: Rect,
}

/// Calculates the layout areas for the given terminal size.
///
/// ```text
/// +------------------------------------------+
/// |              Title Bar                   |  <- Length(1)
/// +---------------------+--------------------+
/// |        Left         |       Right        |  <- Ratio(1,2), split horizontally
/// +---------------------+--------------------+
/// |                Result                    |  <- Ratio(1,2)
/// +------------------------------------------+
/// |              Status Bar                  |  <- Length(1)
/// +------------------------------------------+
/// ```
#[must_use]
pub fn calculate_layout(area: Rect) -> PaneAreas {
    // Vertical split: title, main, status
    let [title_bar, main_area, status_bar] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Split main area into top (left/right) and bottom (result)
    let [top_row, result_pane] = Layout::vertical([
        Constraint::Ratio(1, 2),
        Constraint::Ratio(1, 2),
    ])
    .areas(main_area);

    // Horizontal split for top row: left, right
    let [left_pane, right_pane] = Layout::horizontal([
        Constraint::Ratio(1, 2),
        Constraint::Ratio(1, 2),
    ])
    .areas(top_row);

    PaneAreas {
        title_bar,
        left_pane,
        right_pane,
        result_pane,
        status_bar,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_layout_returns_non_zero_areas() {
        let area = Rect::new(0, 0, 80, 24);
        let areas = calculate_layout(area);

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
        let areas = calculate_layout(area);

        assert_eq!(areas.title_bar.height, 1);
        assert_eq!(areas.status_bar.height, 1);
    }

    #[test]
    fn left_and_right_are_side_by_side() {
        let area = Rect::new(0, 0, 80, 24);
        let areas = calculate_layout(area);

        // Left and right should have the same y position
        assert_eq!(areas.left_pane.y, areas.right_pane.y);
        // Left should be to the left of right
        assert!(areas.left_pane.x < areas.right_pane.x);
    }

    #[test]
    fn result_is_below_left_and_right() {
        let area = Rect::new(0, 0, 80, 24);
        let areas = calculate_layout(area);

        // Result should be below both left and right panes
        assert!(areas.result_pane.y > areas.left_pane.y);
        assert!(areas.result_pane.y > areas.right_pane.y);
    }

    #[test]
    fn result_spans_full_width() {
        let area = Rect::new(0, 0, 80, 24);
        let areas = calculate_layout(area);

        // Result pane should span the full width
        assert_eq!(areas.result_pane.width, area.width);
    }

    #[test]
    fn handles_minimum_terminal_size() {
        // Very small terminal
        let area = Rect::new(0, 0, 10, 5);
        let areas = calculate_layout(area);

        // Should not panic
        let _ = areas;
    }
}
