//! Integration tests for weavr-git using real Git repositories.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;
use weavr_git::{GitOperation, GitRepo};

/// Helper to create a Git repository in a temp directory.
fn setup_git_repo() -> TempDir {
    let dir = TempDir::new().expect("create temp dir");

    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .output()
        .expect("git init");

    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .expect("git config email");

    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .expect("git config name");

    dir
}

/// Helper to commit a file.
fn commit_file(dir: &TempDir, name: &str, content: &str, message: &str) {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&path, content).expect("write file");

    Command::new("git")
        .args(["add", name])
        .current_dir(dir.path())
        .output()
        .expect("git add");

    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(dir.path())
        .output()
        .expect("git commit");
}

/// Helper to canonicalize paths for comparison (handles macOS /var -> /private/var).
fn canonicalize_for_comparison(path: &std::path::Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[test]
fn discover_from_root() {
    let dir = setup_git_repo();
    commit_file(&dir, "file.txt", "content", "Initial commit");

    let repo = GitRepo::discover_from(dir.path()).expect("should discover repo");
    assert_eq!(
        canonicalize_for_comparison(repo.root()),
        canonicalize_for_comparison(dir.path())
    );
}

#[test]
fn discover_from_subdirectory() {
    let dir = setup_git_repo();
    commit_file(&dir, "file.txt", "content", "Initial commit");

    let subdir = dir.path().join("deep/nested/directory");
    fs::create_dir_all(&subdir).expect("create subdirs");

    let repo = GitRepo::discover_from(&subdir).expect("should discover repo");
    assert_eq!(
        canonicalize_for_comparison(repo.root()),
        canonicalize_for_comparison(dir.path())
    );
}

#[test]
fn discover_not_git_repo() {
    let dir = TempDir::new().expect("create temp dir");
    let result = GitRepo::discover_from(dir.path());
    assert!(result.is_err());
}

#[test]
fn no_conflicts_when_clean() {
    let dir = setup_git_repo();
    commit_file(&dir, "file.txt", "content", "Initial commit");

    let repo = GitRepo::discover_from(dir.path()).expect("discover repo");

    assert!(!repo.is_in_merge());
    assert!(!repo.is_in_rebase());
    assert!(!repo.is_in_cherry_pick());
    assert!(!repo.is_in_revert());
    assert_eq!(repo.current_operation(), GitOperation::None);

    let conflicts = repo.conflicted_files().expect("get conflicts");
    assert!(conflicts.is_empty());
}

#[test]
fn detect_merge_conflict() {
    let dir = setup_git_repo();

    // Create initial commit on main
    commit_file(&dir, "file.txt", "initial", "Initial commit");

    // Create branch and modify
    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(dir.path())
        .output()
        .expect("create branch");
    commit_file(&dir, "file.txt", "feature change", "Feature commit");

    // Go back to main and create conflicting change
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(dir.path())
        .output()
        .expect("checkout main");
    commit_file(&dir, "file.txt", "main change", "Main commit");

    // Attempt merge (will conflict)
    let merge_result = Command::new("git")
        .args(["merge", "feature"])
        .current_dir(dir.path())
        .output()
        .expect("merge command");

    // The merge should fail due to conflict
    assert!(
        !merge_result.status.success(),
        "merge should have conflicted"
    );

    let repo = GitRepo::discover_from(dir.path()).expect("discover repo");

    assert!(repo.is_in_merge());
    assert_eq!(repo.current_operation(), GitOperation::Merge);

    let conflicts = repo.conflicted_files().expect("get conflicts");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0], PathBuf::from("file.txt"));
}

#[test]
fn detect_rebase_conflict() {
    let dir = setup_git_repo();

    // Create initial commit
    commit_file(&dir, "file.txt", "initial", "Initial commit");

    // Create branch and modify
    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(dir.path())
        .output()
        .expect("create branch");
    commit_file(&dir, "file.txt", "feature change", "Feature commit");

    // Go back to main and create conflicting change
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(dir.path())
        .output()
        .expect("checkout main");
    commit_file(&dir, "file.txt", "main change", "Main commit");

    // Go to feature branch and rebase onto main
    Command::new("git")
        .args(["checkout", "feature"])
        .current_dir(dir.path())
        .output()
        .expect("checkout feature");

    let rebase_result = Command::new("git")
        .args(["rebase", "main"])
        .current_dir(dir.path())
        .output()
        .expect("rebase command");

    // The rebase should fail due to conflict
    assert!(
        !rebase_result.status.success(),
        "rebase should have conflicted"
    );

    let repo = GitRepo::discover_from(dir.path()).expect("discover repo");

    assert!(repo.is_in_rebase());
    assert_eq!(repo.current_operation(), GitOperation::Rebase);

    let conflicts = repo.conflicted_files().expect("get conflicts");
    assert_eq!(conflicts.len(), 1);
}

#[test]
fn detect_cherry_pick_conflict() {
    let dir = setup_git_repo();

    // Create initial commit
    commit_file(&dir, "file.txt", "initial", "Initial commit");

    // Create branch and modify
    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(dir.path())
        .output()
        .expect("create branch");
    commit_file(&dir, "file.txt", "feature change", "Feature commit");

    // Get the feature commit hash
    let log_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir.path())
        .output()
        .expect("git rev-parse");
    let feature_commit = String::from_utf8_lossy(&log_output.stdout)
        .trim()
        .to_string();

    // Go back to main and create conflicting change
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(dir.path())
        .output()
        .expect("checkout main");
    commit_file(&dir, "file.txt", "main change", "Main commit");

    // Cherry-pick the feature commit
    let cherry_pick_result = Command::new("git")
        .args(["cherry-pick", &feature_commit])
        .current_dir(dir.path())
        .output()
        .expect("cherry-pick command");

    // The cherry-pick should fail due to conflict
    assert!(
        !cherry_pick_result.status.success(),
        "cherry-pick should have conflicted"
    );

    let repo = GitRepo::discover_from(dir.path()).expect("discover repo");

    assert!(repo.is_in_cherry_pick());
    assert_eq!(repo.current_operation(), GitOperation::CherryPick);

    let conflicts = repo.conflicted_files().expect("get conflicts");
    assert_eq!(conflicts.len(), 1);
}

#[test]
fn stage_resolved_file() {
    let dir = setup_git_repo();

    // Set up merge conflict
    commit_file(&dir, "file.txt", "initial", "Initial commit");

    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(dir.path())
        .output()
        .expect("create branch");
    commit_file(&dir, "file.txt", "feature", "Feature commit");

    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(dir.path())
        .output()
        .expect("checkout main");
    commit_file(&dir, "file.txt", "main", "Main commit");

    Command::new("git")
        .args(["merge", "feature"])
        .current_dir(dir.path())
        .output()
        .ok();

    let repo = GitRepo::discover_from(dir.path()).expect("discover repo");

    // Verify we have a conflict
    let conflicts_before = repo.conflicted_files().expect("get conflicts");
    assert_eq!(conflicts_before.len(), 1);

    // Resolve the conflict manually by writing resolved content
    let file_path = dir.path().join("file.txt");
    fs::write(&file_path, "resolved content").expect("write resolved");

    // Stage the resolved file
    repo.stage_file(&PathBuf::from("file.txt"))
        .expect("stage file");

    // Should no longer be in conflicts list
    let conflicts_after = repo.conflicted_files().expect("get conflicts");
    assert!(conflicts_after.is_empty());
}

#[test]
fn conflicted_entries_returns_conflict_types() {
    let dir = setup_git_repo();

    // Set up merge conflict
    commit_file(&dir, "file.txt", "initial", "Initial commit");

    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(dir.path())
        .output()
        .expect("create branch");
    commit_file(&dir, "file.txt", "feature", "Feature commit");

    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(dir.path())
        .output()
        .expect("checkout main");
    commit_file(&dir, "file.txt", "main", "Main commit");

    Command::new("git")
        .args(["merge", "feature"])
        .current_dir(dir.path())
        .output()
        .ok();

    let repo = GitRepo::discover_from(dir.path()).expect("discover repo");

    let entries = repo.conflicted_entries().expect("get entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, PathBuf::from("file.txt"));
    assert_eq!(
        entries[0].conflict_type,
        weavr_git::ConflictType::BothModified
    );
}
