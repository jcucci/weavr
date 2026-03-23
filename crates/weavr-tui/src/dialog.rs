//! Dialog management for modal overlays.
//!
//! This module handles:
//! - Help dialog
//! - `AcceptBoth` options dialog
//! - File list dialog with search/filter/sort

use weavr_core::{AcceptBothOptions, BothOrder, Resolution};

use crate::input::{
    AcceptBothOptionsState, Dialog, FileListFilter, FileListMode, FileListSort, FileListState,
    HelpState, InputMode,
};
use crate::resolution;
use crate::workspace::{FileStatus, Workspace};
use crate::App;

/// Shows the help dialog.
pub fn show_help(app: &mut App) {
    app.active_dialog = Some(Dialog::Help(HelpState::default()));
    app.input_mode = InputMode::Dialog;
}

/// Scrolls the help dialog down by the given number of lines.
pub fn help_scroll_down(app: &mut App, lines: usize) {
    if let Some(Dialog::Help(ref mut state)) = app.active_dialog {
        state.scroll = state.scroll.saturating_add(lines);
    }
}

/// Scrolls the help dialog up by the given number of lines.
pub fn help_scroll_up(app: &mut App, lines: usize) {
    if let Some(Dialog::Help(ref mut state)) = app.active_dialog {
        state.scroll = state.scroll.saturating_sub(lines);
    }
}

/// Closes any open dialog and returns to normal mode.
pub fn close_dialog(app: &mut App) {
    app.active_dialog = None;
    app.input_mode = InputMode::Normal;
}

/// Shows the `AcceptBoth` options dialog.
pub fn show_accept_both_dialog(app: &mut App) {
    app.active_dialog = Some(Dialog::AcceptBothOptions(AcceptBothOptionsState::default()));
    app.input_mode = InputMode::Dialog;
}

/// Toggles the order in the `AcceptBoth` options dialog.
pub fn toggle_accept_both_order(app: &mut App) {
    if let Some(Dialog::AcceptBothOptions(ref mut state)) = app.active_dialog {
        state.order = match state.order {
            BothOrder::LeftThenRight => BothOrder::RightThenLeft,
            BothOrder::RightThenLeft => BothOrder::LeftThenRight,
        };
    }
}

/// Toggles the deduplicate option in the `AcceptBoth` options dialog.
pub fn toggle_accept_both_dedupe(app: &mut App) {
    if let Some(Dialog::AcceptBothOptions(ref mut state)) = app.active_dialog {
        state.deduplicate = !state.deduplicate;
    }
}

/// Shows the staging prompt dialog.
pub fn show_staging_prompt(app: &mut App) {
    app.active_dialog = Some(Dialog::StagingPrompt);
    app.input_mode = InputMode::Dialog;
}

/// Confirms staging in the staging prompt dialog.
pub fn confirm_staging(app: &mut App) {
    let multi = app.is_multi_file();
    app.stage_requested = true;
    if let Some(ref mut ws) = app.workspace {
        ws.current_mut().stage_requested = true;
    }
    close_dialog(app);
    if multi {
        app.complete_current_and_advance();
    } else {
        app.quit();
    }
}

/// Denies staging in the staging prompt dialog.
pub fn deny_staging(app: &mut App) {
    let multi = app.is_multi_file();
    close_dialog(app);
    if multi {
        app.complete_current_and_advance();
    } else {
        app.quit();
    }
}

/// Shows the file list dialog.
pub fn show_file_list(app: &mut App) {
    let current = app.current_file_index();
    let file_count = app.file_count();
    let filtered_indices: Vec<usize> = (0..file_count).collect();
    let selected_index = filtered_indices
        .iter()
        .position(|&i| i == current)
        .unwrap_or(0);
    app.active_dialog = Some(Dialog::FileList(FileListState {
        selected_index,
        filtered_indices,
        mode: FileListMode::Navigate,
        search_query: String::new(),
        sort: FileListSort::default(),
        filter: FileListFilter::default(),
    }));
    app.input_mode = InputMode::Dialog;
}

/// Moves the file list selection down.
pub fn file_list_move_down(app: &mut App) {
    if let Some(Dialog::FileList(ref mut state)) = app.active_dialog {
        if state.selected_index + 1 < state.filtered_indices.len() {
            state.selected_index += 1;
        }
    }
}

/// Moves the file list selection up.
pub fn file_list_move_up(app: &mut App) {
    if let Some(Dialog::FileList(ref mut state)) = app.active_dialog {
        state.selected_index = state.selected_index.saturating_sub(1);
    }
}

/// Selects the current item in the file list dialog and switches to that file.
/// No-ops when the filtered list is empty so the dialog isn't dismissed on Enter
/// with no matching files.
pub fn file_list_select(app: &mut App) {
    let workspace_index = if let Some(Dialog::FileList(ref state)) = app.active_dialog {
        state.filtered_indices.get(state.selected_index).copied()
    } else {
        return;
    };
    if let Some(idx) = workspace_index {
        close_dialog(app);
        app.go_to_file(idx);
    }
}

/// Enters search mode in the file list dialog.
pub fn file_list_enter_search(app: &mut App) {
    if let Some(Dialog::FileList(ref mut state)) = app.active_dialog {
        state.mode = FileListMode::Search;
    }
}

/// Exits search mode (back to navigate), keeping the current query.
pub fn file_list_exit_search(app: &mut App) {
    if let Some(Dialog::FileList(ref mut state)) = app.active_dialog {
        state.mode = FileListMode::Navigate;
    }
}

/// Appends a character to the search query and recomputes the filtered list.
pub fn file_list_search_append(app: &mut App, c: char) {
    let workspace = app.workspace.as_ref();
    if let (Some(Dialog::FileList(ref mut state)), Some(ws)) = (&mut app.active_dialog, workspace) {
        state.search_query.push(c);
        recompute_filtered_indices(state, ws);
    }
}

/// Removes the last character from the search query and recomputes.
pub fn file_list_search_backspace(app: &mut App) {
    let workspace = app.workspace.as_ref();
    if let (Some(Dialog::FileList(ref mut state)), Some(ws)) = (&mut app.active_dialog, workspace) {
        state.search_query.pop();
        recompute_filtered_indices(state, ws);
    }
}

/// Cycles the sort order and recomputes the filtered list.
pub fn file_list_cycle_sort(app: &mut App) {
    let workspace = app.workspace.as_ref();
    if let (Some(Dialog::FileList(ref mut state)), Some(ws)) = (&mut app.active_dialog, workspace) {
        state.sort = state.sort.next();
        recompute_filtered_indices(state, ws);
    }
}

/// Cycles the filter predicate and recomputes the filtered list.
pub fn file_list_cycle_filter(app: &mut App) {
    let workspace = app.workspace.as_ref();
    if let (Some(Dialog::FileList(ref mut state)), Some(ws)) = (&mut app.active_dialog, workspace) {
        state.filter = state.filter.next();
        recompute_filtered_indices(state, ws);
    }
}

/// Recomputes `filtered_indices` from the current search, filter, and sort
/// settings. Tries to preserve the previously-selected workspace index.
fn recompute_filtered_indices(state: &mut FileListState, workspace: &Workspace) {
    // Remember which workspace index was selected before recompute.
    let prev_workspace_idx = state.filtered_indices.get(state.selected_index).copied();

    let files = workspace.files();
    let query_lower = state.search_query.to_lowercase();

    // 1. Gather indices that pass filter + search.
    let mut indices: Vec<usize> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| match state.filter {
            FileListFilter::All => true,
            FileListFilter::Unresolved => f.status() != FileStatus::FullyResolved,
            FileListFilter::Resolved => f.status() == FileStatus::FullyResolved,
        })
        .filter(|(_, f)| {
            if query_lower.is_empty() {
                return true;
            }
            f.path
                .to_string_lossy()
                .to_lowercase()
                .contains(&query_lower)
        })
        .map(|(i, _)| i)
        .collect();

    // 2. Sort.
    match state.sort {
        FileListSort::Path => {
            indices.sort_by(|&a, &b| files[a].path.cmp(&files[b].path));
        }
        FileListSort::ConflictCount => {
            indices.sort_by(|&a, &b| {
                let (ra, ta) = files[a].resolution_counts();
                let (rb, tb) = files[b].resolution_counts();
                let unresolved_a = ta.saturating_sub(ra);
                let unresolved_b = tb.saturating_sub(rb);
                unresolved_b
                    .cmp(&unresolved_a)
                    .then(files[a].path.cmp(&files[b].path))
            });
        }
        FileListSort::FileType => {
            indices.sort_by(|&a, &b| {
                let ext_a = files[a]
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let ext_b = files[b]
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                ext_a.cmp(ext_b).then(files[a].path.cmp(&files[b].path))
            });
        }
    }

    state.filtered_indices = indices;

    // 3. Try to preserve the previously-selected workspace index.
    if let Some(prev) = prev_workspace_idx {
        if let Some(pos) = state.filtered_indices.iter().position(|&i| i == prev) {
            state.selected_index = pos;
            return;
        }
    }

    // 4. Clamp selected_index to new bounds.
    if state.filtered_indices.is_empty() {
        state.selected_index = 0;
    } else if state.selected_index >= state.filtered_indices.len() {
        state.selected_index = state.filtered_indices.len() - 1;
    }
}

/// Confirms the `AcceptBoth` options and applies the resolution.
pub fn confirm_accept_both(app: &mut App) {
    // Extract options from dialog
    let options = if let Some(Dialog::AcceptBothOptions(ref state)) = app.active_dialog {
        AcceptBothOptions {
            order: state.order,
            deduplicate: state.deduplicate,
            trim_whitespace: false,
        }
    } else {
        return;
    };

    // Close dialog first
    close_dialog(app);

    // Apply resolution with extracted options
    resolution::apply_resolution(app, "Accept both", |hunk| {
        Resolution::accept_both(hunk, &options)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use weavr_core::MergeSession;

    use crate::workspace::{FileState, Workspace};

    fn make_session(content: &str) -> MergeSession {
        MergeSession::from_conflicted(content, PathBuf::from("test.rs")).unwrap()
    }

    fn conflicted_session() -> MergeSession {
        make_session("<<<<<<< HEAD\nleft\n=======\nright\n>>>>>>> branch\n")
    }

    fn clean_session() -> MergeSession {
        make_session("no conflicts\n")
    }

    fn make_workspace() -> Workspace {
        Workspace::new(vec![
            FileState::new(PathBuf::from("src/alpha.rs"), conflicted_session()),
            FileState::new(PathBuf::from("src/beta.ts"), conflicted_session()),
            FileState::new(PathBuf::from("lib/gamma.rs"), clean_session()),
            FileState::new(PathBuf::from("src/delta.py"), conflicted_session()),
        ])
    }

    fn default_state(file_count: usize) -> FileListState {
        FileListState {
            selected_index: 0,
            filtered_indices: (0..file_count).collect(),
            mode: FileListMode::Navigate,
            search_query: String::new(),
            sort: FileListSort::default(),
            filter: FileListFilter::default(),
        }
    }

    #[test]
    fn recompute_no_filter_no_search_returns_all() {
        let ws = make_workspace();
        let mut state = default_state(ws.file_count());
        recompute_filtered_indices(&mut state, &ws);
        assert_eq!(state.filtered_indices.len(), 4);
    }

    #[test]
    fn recompute_search_filters_by_path() {
        let ws = make_workspace();
        let mut state = default_state(ws.file_count());
        state.search_query = "alpha".into();
        recompute_filtered_indices(&mut state, &ws);
        assert_eq!(state.filtered_indices, vec![0]);
    }

    #[test]
    fn recompute_search_case_insensitive() {
        let ws = make_workspace();
        let mut state = default_state(ws.file_count());
        state.search_query = "BETA".into();
        recompute_filtered_indices(&mut state, &ws);
        assert_eq!(state.filtered_indices, vec![1]);
    }

    #[test]
    fn recompute_filter_unresolved() {
        let ws = make_workspace();
        let mut state = default_state(ws.file_count());
        state.filter = FileListFilter::Unresolved;
        recompute_filtered_indices(&mut state, &ws);
        // gamma.rs is clean (fully resolved), so excluded
        assert_eq!(state.filtered_indices.len(), 3);
        assert!(!state.filtered_indices.contains(&2));
    }

    #[test]
    fn recompute_filter_resolved() {
        let ws = make_workspace();
        let mut state = default_state(ws.file_count());
        state.filter = FileListFilter::Resolved;
        recompute_filtered_indices(&mut state, &ws);
        // Only gamma.rs is fully resolved
        assert_eq!(state.filtered_indices, vec![2]);
    }

    #[test]
    fn recompute_sort_by_file_type() {
        let ws = make_workspace();
        let mut state = default_state(ws.file_count());
        state.sort = FileListSort::FileType;
        recompute_filtered_indices(&mut state, &ws);
        // Extensions: .py, .rs, .rs, .ts
        let exts: Vec<&str> = state
            .filtered_indices
            .iter()
            .map(|&i| ws.files()[i].path.extension().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(exts, vec!["py", "rs", "rs", "ts"]);
    }

    #[test]
    fn recompute_empty_results() {
        let ws = make_workspace();
        let mut state = default_state(ws.file_count());
        state.search_query = "nonexistent".into();
        recompute_filtered_indices(&mut state, &ws);
        assert!(state.filtered_indices.is_empty());
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn recompute_clamps_selected_index() {
        let ws = make_workspace();
        let mut state = default_state(ws.file_count());
        state.selected_index = 3; // last item
        state.search_query = "alpha".into(); // only 1 result
        recompute_filtered_indices(&mut state, &ws);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn recompute_preserves_selected_workspace_index() {
        let ws = make_workspace();
        let mut state = default_state(ws.file_count());
        // Select beta.ts (workspace index 1, display index 1)
        state.selected_index = 1;
        // Search for "src" — matches indices 0, 1, 3
        state.search_query = "src".into();
        recompute_filtered_indices(&mut state, &ws);
        // beta.ts should still be selected (now at display index 1)
        assert_eq!(
            state.filtered_indices[state.selected_index], 1,
            "should preserve workspace index 1 (beta.ts)"
        );
    }

    #[test]
    fn recompute_combined_filter_and_search() {
        let ws = make_workspace();
        let mut state = default_state(ws.file_count());
        state.filter = FileListFilter::Unresolved;
        state.search_query = ".rs".into();
        recompute_filtered_indices(&mut state, &ws);
        // Only alpha.rs matches (gamma.rs is resolved)
        assert_eq!(state.filtered_indices, vec![0]);
    }

    #[test]
    fn sort_cycle_round_trips() {
        let s = FileListSort::Path;
        assert_eq!(s.next(), FileListSort::ConflictCount);
        assert_eq!(s.next().next(), FileListSort::FileType);
        assert_eq!(s.next().next().next(), FileListSort::Path);
    }

    #[test]
    fn filter_cycle_round_trips() {
        let f = FileListFilter::All;
        assert_eq!(f.next(), FileListFilter::Unresolved);
        assert_eq!(f.next().next(), FileListFilter::Resolved);
        assert_eq!(f.next().next().next(), FileListFilter::All);
    }
}
