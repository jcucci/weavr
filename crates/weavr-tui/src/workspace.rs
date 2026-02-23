//! Multi-file workspace management.
//!
//! This module provides types for managing multiple conflicted files
//! in a single TUI session, enabling navigation between files and
//! tracking per-file resolution state.

use std::path::PathBuf;

use weavr_core::{ActionHistory, HunkState, MergeSession};

use crate::FocusedPane;

/// Resolution status of a file in the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// No hunks resolved yet.
    NotStarted,
    /// Some hunks resolved, but not all.
    InProgress,
    /// All hunks resolved.
    FullyResolved,
}

/// Per-file state saved/restored when switching between files.
pub struct FileState {
    /// Path to the conflicted file.
    pub path: PathBuf,
    /// The merge session for this file.
    pub session: MergeSession,
    /// Current hunk index.
    pub current_hunk_index: usize,
    /// Scroll offset for left/right panes.
    pub left_right_scroll: u16,
    /// Scroll offset for result pane.
    pub result_scroll: u16,
    /// Undo/redo history for this file.
    pub action_history: ActionHistory,
    /// Whether the user requested staging for this file.
    pub stage_requested: bool,
    /// Which pane has focus.
    pub focused_pane: FocusedPane,
    /// Whether the file has been marked as saved/completed.
    pub written: bool,
}

impl FileState {
    /// Creates a new file state from a path and merge session.
    #[must_use]
    pub fn new(path: PathBuf, session: MergeSession) -> Self {
        Self {
            path,
            session,
            current_hunk_index: 0,
            left_right_scroll: 0,
            result_scroll: 0,
            action_history: ActionHistory::new(),
            stage_requested: false,
            focused_pane: FocusedPane::default(),
            written: false,
        }
    }

    /// Returns the resolution status of this file.
    #[must_use]
    pub fn status(&self) -> FileStatus {
        let (resolved, total) = self.resolution_counts();
        if total == 0 || resolved == total {
            FileStatus::FullyResolved
        } else if resolved == 0 {
            FileStatus::NotStarted
        } else {
            FileStatus::InProgress
        }
    }

    /// Returns the filename (last component) for display.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("???")
    }

    /// Returns the resolved hunk count and total hunk count.
    #[must_use]
    pub fn resolution_counts(&self) -> (usize, usize) {
        let total = self.session.hunks().len();
        let resolved = self
            .session
            .hunks()
            .iter()
            .filter(|h| matches!(h.state, HunkState::Resolved(_)))
            .count();
        (resolved, total)
    }
}

/// Manages multiple files in a single TUI session.
pub struct Workspace {
    files: Vec<FileState>,
    current_index: usize,
}

impl Workspace {
    /// Creates a new workspace from a list of file states.
    ///
    /// # Panics
    ///
    /// Panics if `files` is empty.
    #[must_use]
    pub fn new(files: Vec<FileState>) -> Self {
        assert!(!files.is_empty(), "workspace must have at least one file");
        Self {
            files,
            current_index: 0,
        }
    }

    /// Returns a reference to the current file state.
    #[must_use]
    pub fn current(&self) -> &FileState {
        &self.files[self.current_index]
    }

    /// Returns a mutable reference to the current file state.
    pub fn current_mut(&mut self) -> &mut FileState {
        &mut self.files[self.current_index]
    }

    /// Returns the current file index.
    #[must_use]
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Returns the total number of files.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Moves to the next file. Returns `true` if the index changed.
    pub fn advance(&mut self) -> bool {
        if self.current_index + 1 < self.files.len() {
            self.current_index += 1;
            true
        } else {
            false
        }
    }

    /// Moves to the previous file. Returns `true` if the index changed.
    pub fn go_back(&mut self) -> bool {
        if self.current_index > 0 {
            self.current_index -= 1;
            true
        } else {
            false
        }
    }

    /// Moves to a specific file by index. Returns `true` if the index changed.
    pub fn go_to(&mut self, index: usize) -> bool {
        if index < self.files.len() && index != self.current_index {
            self.current_index = index;
            true
        } else {
            false
        }
    }

    /// Returns the statuses of all files.
    #[must_use]
    pub fn file_statuses(&self) -> Vec<(FileStatus, bool)> {
        self.files.iter().map(|f| (f.status(), f.written)).collect()
    }

    /// Returns `true` if all files have been marked as written.
    #[must_use]
    pub fn all_written(&self) -> bool {
        self.files.iter().all(|f| f.written)
    }

    /// Returns the index of the next unwritten file, if any.
    #[must_use]
    pub fn next_unwritten_index(&self) -> Option<usize> {
        // Search forward from current
        for i in (self.current_index + 1)..self.files.len() {
            if !self.files[i].written {
                return Some(i);
            }
        }
        // Wrap around from the beginning
        (0..self.current_index).find(|&i| !self.files[i].written)
    }

    /// Returns a reference to all file states.
    #[must_use]
    pub fn files(&self) -> &[FileState] {
        &self.files
    }

    /// Returns a mutable reference to all file states.
    pub fn files_mut(&mut self) -> &mut Vec<FileState> {
        &mut self.files
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weavr_core::MergeSession;

    fn make_conflicted_session() -> MergeSession {
        let content = "line 1\n<<<<<<< HEAD\nleft\n=======\nright\n>>>>>>> branch\nline 3\n";
        MergeSession::from_conflicted(content, PathBuf::from("test.rs")).unwrap()
    }

    fn make_clean_session() -> MergeSession {
        let content = "no conflicts here\n";
        MergeSession::from_conflicted(content, PathBuf::from("clean.rs")).unwrap()
    }

    fn make_file_state(name: &str) -> FileState {
        FileState::new(PathBuf::from(name), make_conflicted_session())
    }

    #[test]
    fn workspace_creation() {
        let ws = Workspace::new(vec![make_file_state("a.rs"), make_file_state("b.rs")]);
        assert_eq!(ws.file_count(), 2);
        assert_eq!(ws.current_index(), 0);
    }

    #[test]
    #[should_panic(expected = "workspace must have at least one file")]
    fn workspace_empty_panics() {
        let _ = Workspace::new(vec![]);
    }

    #[test]
    fn workspace_navigation_next() {
        let mut ws = Workspace::new(vec![
            make_file_state("a.rs"),
            make_file_state("b.rs"),
            make_file_state("c.rs"),
        ]);
        assert_eq!(ws.current_index(), 0);
        assert!(ws.advance());
        assert_eq!(ws.current_index(), 1);
        assert!(ws.advance());
        assert_eq!(ws.current_index(), 2);
        assert!(!ws.advance()); // at end
        assert_eq!(ws.current_index(), 2);
    }

    #[test]
    fn workspace_navigation_prev() {
        let mut ws = Workspace::new(vec![make_file_state("a.rs"), make_file_state("b.rs")]);
        assert!(!ws.go_back()); // at start
        assert!(ws.advance());
        assert!(ws.go_back());
        assert_eq!(ws.current_index(), 0);
    }

    #[test]
    fn workspace_go_to() {
        let mut ws = Workspace::new(vec![
            make_file_state("a.rs"),
            make_file_state("b.rs"),
            make_file_state("c.rs"),
        ]);
        assert!(ws.go_to(2));
        assert_eq!(ws.current_index(), 2);
        assert!(!ws.go_to(2)); // same index
        assert!(!ws.go_to(5)); // out of bounds
    }

    #[test]
    fn file_status_not_started() {
        let state = make_file_state("a.rs");
        assert_eq!(state.status(), FileStatus::NotStarted);
    }

    #[test]
    fn file_status_fully_resolved_for_clean() {
        let state = FileState::new(PathBuf::from("clean.rs"), make_clean_session());
        assert_eq!(state.status(), FileStatus::FullyResolved);
    }

    #[test]
    fn file_display_name() {
        let state = FileState::new(PathBuf::from("src/lib.rs"), make_conflicted_session());
        assert_eq!(state.display_name(), "lib.rs");
    }

    #[test]
    fn resolution_counts() {
        let state = make_file_state("a.rs");
        let (resolved, total) = state.resolution_counts();
        assert_eq!(resolved, 0);
        assert_eq!(total, 1);
    }

    #[test]
    fn all_written_false_initially() {
        let ws = Workspace::new(vec![make_file_state("a.rs"), make_file_state("b.rs")]);
        assert!(!ws.all_written());
    }

    #[test]
    fn all_written_true_when_all_marked() {
        let mut ws = Workspace::new(vec![make_file_state("a.rs"), make_file_state("b.rs")]);
        ws.files_mut()[0].written = true;
        ws.files_mut()[1].written = true;
        assert!(ws.all_written());
    }

    #[test]
    fn next_unwritten_index_finds_next() {
        let mut ws = Workspace::new(vec![
            make_file_state("a.rs"),
            make_file_state("b.rs"),
            make_file_state("c.rs"),
        ]);
        ws.files_mut()[0].written = true;
        // Current is 0, next unwritten is 1
        assert_eq!(ws.next_unwritten_index(), Some(1));
    }

    #[test]
    fn next_unwritten_index_wraps_around() {
        let mut ws = Workspace::new(vec![
            make_file_state("a.rs"),
            make_file_state("b.rs"),
            make_file_state("c.rs"),
        ]);
        ws.files_mut()[1].written = true;
        ws.files_mut()[2].written = true;
        // Go to index 1
        ws.go_to(1);
        // Next unwritten wraps to 0
        assert_eq!(ws.next_unwritten_index(), Some(0));
    }

    #[test]
    fn next_unwritten_index_none_when_all_written() {
        let mut ws = Workspace::new(vec![make_file_state("a.rs"), make_file_state("b.rs")]);
        ws.files_mut()[0].written = true;
        ws.files_mut()[1].written = true;
        assert_eq!(ws.next_unwritten_index(), None);
    }
}
