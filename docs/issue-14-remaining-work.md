# Issue #14: Remaining Implementation Work

## Context

Issue #14 "Resolution keybindings" is partially implemented on branch `feat/issue-14-resolution-keybindings`.

**Completed phases:**
- Phase 1: Keybindings (`o`/`t`/`b`/`x`)
- Phase 2: Status messages
- Phase 3: Undo system (`u`)
- Phase 4: Command mode (`:w`, `:q`, `:wq`, `:q!`)
- Phase 5: Help overlay (`?`)
- Phase 6: AcceptBoth options dialog (`B`)
- Phase 7: `$EDITOR` integration (`e`)

**Remaining phases:** None (Issue #14 is fully implemented)

---

## Phase 6: AcceptBoth Options Dialog (`B`)

### Goal
When user presses `B` (shift-b), show a dialog to configure `AcceptBothOptions` before applying.

### AcceptBothOptions (from weavr-core)
```rust
pub struct AcceptBothOptions {
    pub order: BothOrder,      // LeftThenRight or RightThenLeft
    pub deduplicate: bool,     // Remove duplicate lines
    pub trim_whitespace: bool, // Trim whitespace when comparing
}
```

### UI Design
```
┌─ Accept Both Options ─────────────────┐
│                                       │
│  Order: [L]eft first  (R)ight first   │
│  Deduplicate: [ ] enabled             │
│                                       │
│  [Enter] confirm  [Esc] cancel        │
└───────────────────────────────────────┘
```

### Implementation Steps

1. **Add Dialog variant** in `src/input.rs`:
   ```rust
   pub enum Dialog {
       Help,
       AcceptBothOptions(AcceptBothOptionsState),
   }

   #[derive(Debug, Clone, Default)]
   pub struct AcceptBothOptionsState {
       pub order: BothOrder,
       pub deduplicate: bool,
       pub focused_field: usize, // 0 = order, 1 = deduplicate
   }
   ```

2. **Add App methods** in `src/lib.rs`:
   ```rust
   pub fn show_accept_both_dialog(&mut self)
   pub fn toggle_accept_both_order(&mut self)
   pub fn toggle_accept_both_dedupe(&mut self)
   pub fn confirm_accept_both(&mut self)
   ```

3. **Add rendering** in `src/ui/overlay.rs`:
   ```rust
   pub fn render_accept_both_dialog(frame, area, theme, state)
   ```

4. **Update ui/mod.rs** to render the new dialog variant

5. **Update event.rs**:
   - Add `B` keybinding in normal mode to show dialog
   - Handle dialog-specific keys (L/R to toggle order, Space for dedupe, Enter to confirm)

6. **Add keybinding** in `src/event.rs` normal mode:
   ```rust
   KeyCode::Char('B') => app.show_accept_both_dialog(),
   ```

---

## Phase 7: $EDITOR Integration (`e`)

### Goal
When user presses `e`, open `$EDITOR` with the current hunk content for manual editing. On save, apply as `Resolution::manual()`.

### Flow
1. User presses `e`
2. Get current hunk's content (or existing resolution if resolved)
3. Write to temp file
4. Suspend TUI (`ratatui::restore()`)
5. Spawn `$EDITOR` (fallback to `vi`)
6. Wait for editor to exit
7. Read temp file content
8. Resume TUI (`ratatui::init()`)
9. Apply as `Resolution::manual(content)`
10. Show status message

### Implementation Steps

1. **Add dependency** in `crates/weavr-tui/Cargo.toml`:
   ```toml
   tempfile = "3"
   ```

2. **Add EditorPending state** - Since editor runs synchronously and suspends TUI, we need a way to signal main loop:
   ```rust
   // In App struct
   editor_pending: Option<String>, // Content to edit
   ```

3. **Add App methods** in `src/lib.rs`:
   ```rust
   pub fn prepare_editor(&mut self) -> Option<String>  // Returns content to edit
   pub fn apply_editor_result(&mut self, content: String)
   ```

4. **Modify main.rs** event loop:
   ```rust
   fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
       while !app.should_quit() {
           // Check for pending editor
           if let Some(content) = app.take_editor_pending() {
               ratatui::restore();
               let result = run_editor(&content)?;
               *terminal = ratatui::init();
               if let Some(new_content) = result {
                   app.apply_editor_result(new_content);
               }
               continue;
           }

           terminal.draw(|frame| ui::draw(frame, app))?;
           // ... event handling
       }
       Ok(())
   }

   fn run_editor(content: &str) -> io::Result<Option<String>> {
       let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
       let mut tmp = tempfile::NamedTempFile::new()?;
       std::io::Write::write_all(&mut tmp, content.as_bytes())?;

       let status = std::process::Command::new(&editor)
           .arg(tmp.path())
           .status()?;

       if status.success() {
           Ok(Some(std::fs::read_to_string(tmp.path())?))
       } else {
           Ok(None) // Editor exited with error, cancel
       }
   }
   ```

5. **Add keybinding** in `src/event.rs` normal mode:
   ```rust
   KeyCode::Char('e') => app.prepare_editor(),
   ```

6. **Get hunk content** - Need helper to extract editable content:
   ```rust
   fn get_current_hunk_content(&self) -> Option<String> {
       self.session.as_ref().and_then(|s| {
           s.hunks().get(self.current_hunk_index).map(|hunk| {
               // If already resolved, use that; otherwise combine left/right
               match &hunk.state {
                   HunkState::Resolved(r) => r.content.clone(),
                   HunkState::Unresolved => {
                       format!("<<<<<<< OURS\n{}\n=======\n{}\n>>>>>>> THEIRS",
                           hunk.left.text, hunk.right.text)
                   }
               }
           })
       })
   }
   ```

---

## Files to Modify

| File | Phase 6 | Phase 7 |
|------|---------|---------|
| `src/input.rs` | Add AcceptBothOptionsState | - |
| `src/lib.rs` | Dialog methods | Editor methods |
| `src/event.rs` | `B` key, dialog handling | `e` key |
| `src/ui/overlay.rs` | AcceptBoth dialog render | - |
| `src/ui/mod.rs` | Render new dialog | - |
| `src/main.rs` | - | Editor subprocess |
| `Cargo.toml` | - | Add tempfile |

---

## Testing

After implementation, verify:

**Phase 6:**
- [ ] `B` opens options dialog
- [ ] L/R toggles order
- [ ] Space toggles dedupe
- [ ] Enter applies with selected options
- [ ] Esc cancels

**Phase 7:**
- [ ] `e` opens $EDITOR
- [ ] Content shows current hunk (or resolution if exists)
- [ ] Saving applies as manual resolution
- [ ] Exiting without save cancels
- [ ] TUI resumes correctly after editor closes

Run tests: `cargo test -p weavr-tui`
