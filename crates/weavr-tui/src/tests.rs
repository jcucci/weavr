use super::*;

#[test]
fn app_creation() {
    let app = App::new();
    assert!(!app.should_quit());
    assert!(app.session().is_none());
}

#[test]
fn app_default() {
    let app = App::default();
    assert!(!app.should_quit());
}

#[test]
fn app_quit() {
    let mut app = App::new();
    assert!(!app.should_quit());
    app.quit();
    assert!(app.should_quit());
}

#[test]
fn app_set_session() {
    use std::path::PathBuf;

    let mut app = App::new();
    assert!(app.session().is_none());

    let input = weavr_core::MergeInput {
        left: weavr_core::FileVersion {
            path: PathBuf::from("test.rs"),
            content: String::from("left"),
        },
        right: weavr_core::FileVersion {
            path: PathBuf::from("test.rs"),
            content: String::from("right"),
        },
        base: None,
    };
    let session = weavr_core::MergeSession::new(input).unwrap();
    app.set_session(session);

    assert!(app.session().is_some());
}

#[test]
fn focused_pane_default_is_left() {
    let app = App::new();
    assert_eq!(app.focused_pane(), FocusedPane::Left);
}

#[test]
fn cycle_focus_forward() {
    let mut app = App::new();
    assert_eq!(app.focused_pane(), FocusedPane::Left);

    app.cycle_focus();
    assert_eq!(app.focused_pane(), FocusedPane::Right);

    app.cycle_focus();
    assert_eq!(app.focused_pane(), FocusedPane::Result);

    app.cycle_focus();
    assert_eq!(app.focused_pane(), FocusedPane::Left);
}

#[test]
fn cycle_focus_backward() {
    let mut app = App::new();
    assert_eq!(app.focused_pane(), FocusedPane::Left);

    app.cycle_focus_back();
    assert_eq!(app.focused_pane(), FocusedPane::Result);

    app.cycle_focus_back();
    assert_eq!(app.focused_pane(), FocusedPane::Right);

    app.cycle_focus_back();
    assert_eq!(app.focused_pane(), FocusedPane::Left);
}

#[test]
fn focused_pane_titles() {
    assert_eq!(FocusedPane::Left.title(), "Left (Ours)");
    assert_eq!(FocusedPane::Right.title(), "Right (Theirs)");
    assert_eq!(FocusedPane::Result.title(), "Result");
}

#[test]
fn app_with_theme() {
    let app = App::with_theme(ThemeName::Light);
    // Verify theme is set by checking a known color
    assert_eq!(
        app.theme().base.background,
        ratatui::style::Color::Rgb(250, 250, 250)
    );
}

#[test]
fn app_set_theme() {
    let mut app = App::new();
    app.set_theme(ThemeName::Dracula);
    // Dracula background is Rgb(40, 42, 54)
    assert_eq!(
        app.theme().base.background,
        ratatui::style::Color::Rgb(40, 42, 54)
    );
}

#[test]
fn layout_config_default() {
    let config = LayoutConfig::default();
    assert_eq!(config.top_ratio_percent, 60);
}

#[test]
fn app_initial_hunk_state() {
    let app = App::new();
    assert_eq!(app.current_hunk_index(), 0);
    assert_eq!(app.total_hunks(), 0);
    assert!(app.current_hunk().is_none());
}

#[test]
fn app_hunk_navigation_without_session() {
    let mut app = App::new();
    // Should not panic with no session
    app.next_hunk();
    app.prev_hunk();
    app.go_to_hunk(5);
    app.next_unresolved_hunk();
    app.prev_unresolved_hunk();
    assert_eq!(app.current_hunk_index(), 0);
}

#[test]
fn focus_result_sets_pane() {
    let mut app = App::new();
    assert_eq!(app.focused_pane(), FocusedPane::Left);

    app.focus_result();
    assert_eq!(app.focused_pane(), FocusedPane::Result);
}

#[test]
fn pending_key_sequence() {
    use crossterm::event::KeyCode;

    let mut app = App::new();

    // Initially no pending key
    assert!(!app.check_pending_key(KeyCode::Char('g')));

    // Set a pending key
    app.set_pending_key(KeyCode::Char('g'));

    // Check matching key returns true
    assert!(app.check_pending_key(KeyCode::Char('g')));

    // Check non-matching key returns false
    assert!(!app.check_pending_key(KeyCode::Char('x')));

    // Clear pending key
    app.clear_pending_key();
    assert!(!app.check_pending_key(KeyCode::Char('g')));
}

#[test]
fn app_scroll_state() {
    let mut app = App::new();
    assert_eq!(app.left_right_scroll(), 0);
    assert_eq!(app.result_scroll(), 0);

    // Left pane focused by default, scroll affects left_right
    app.scroll_down(5);
    assert_eq!(app.left_right_scroll(), 5);
    assert_eq!(app.result_scroll(), 0);

    app.scroll_up(2);
    assert_eq!(app.left_right_scroll(), 3);

    // Switch to result pane
    app.cycle_focus();
    app.cycle_focus(); // Now on Result
    app.scroll_down(10);
    assert_eq!(app.left_right_scroll(), 3);
    assert_eq!(app.result_scroll(), 10);
}

#[test]
fn app_scroll_saturates() {
    let mut app = App::new();
    // Scroll up from 0 should stay at 0
    app.scroll_up(100);
    assert_eq!(app.left_right_scroll(), 0);
}
