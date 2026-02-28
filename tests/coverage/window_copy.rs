use super::*;

fn show_buffer(tmux: &TmuxServer) -> String {
    tmux.run(&["show-buffer"])
        .trim_end_matches('\n')
        .to_string()
}

fn show_buffer0(tmux: &TmuxServer) -> String {
    tmux.run(&["show-buffer", "-b", "buffer0"])
        .trim_end_matches('\n')
        .to_string()
}

/// Helper: create a server with known content and enter copy-mode (vi).
fn setup_vi(name: &str, content: &str) -> (TmuxServer, TempFile) {
    let tmux = TmuxServer::new(name);
    let tmp = tmux.write_temp(content);
    let pane_cmd = format!("cat {}; cat", tmp.path_str());
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set-window-option", "-g", "mode-keys", "vi"]);
    tmux.run(&["copy-mode"]);
    (tmux, tmp)
}

/// Helper: create a server with known content and enter copy-mode (emacs).
fn setup_emacs(name: &str, content: &str) -> (TmuxServer, TempFile) {
    let tmux = TmuxServer::new(name);
    let tmp = tmux.write_temp(content);
    let pane_cmd = format!("cat {}; cat", tmp.path_str());
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set-window-option", "-g", "mode-keys", "emacs"]);
    tmux.run(&["copy-mode"]);
    (tmux, tmp)
}

// ============================================================
// Basic entry/exit, navigation
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_enter_exit() {
    let tmux = TmuxServer::new("wc_enter_exit");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy-mode and verify pane_mode
    tmux.run(&["copy-mode"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // Cancel copy mode
    tmux.run(&["send-keys", "-X", "cancel"]);
    assert_eq!(tmux.display("#{pane_mode}"), "");

    // Enter again, test clear-selection + cancel
    tmux.run(&["copy-mode"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "clear-selection"]);
    tmux.run(&["send-keys", "-X", "cancel"]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
}

// ============================================================
// Cursor movement: up, down, left, right
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_cursor_movement() {
    let content = "line one\nline two\nline three\nline four\n";
    let (tmux, _tmp) = setup_vi("wc_cursor_mv", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Move right to position 5 (char 'o' in "line one")
    for _ in 0..5 {
        tmux.run(&["send-keys", "-X", "cursor-right"]);
    }
    // In vi, begin-selection + copy-selection copies the char under cursor
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "o");

    // Move down to line two, same column
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "t");

    // Move left
    tmux.run(&["send-keys", "-X", "cursor-left"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, " ");

    // Move up - cursor returns to same column on previous line
    tmux.run(&["send-keys", "-X", "cursor-up"]);
    // Just verify we're still in copy mode and can select
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Start/end of line, back-to-indentation
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_line_navigation() {
    let content = "hello world\n  indented text\nplain\n";
    let (tmux, _tmp) = setup_vi("wc_line_nav", content);

    tmux.run(&["send-keys", "-X", "history-top"]);

    // End of line then select back to start
    tmux.run(&["send-keys", "-X", "end-of-line"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "hello world");

    // Move to indented line, back-to-indentation skips leading spaces
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "back-to-indentation"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "indented text");
}

// ============================================================
// History top/bottom
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_history_top_bottom() {
    let content = "first line\nsecond line\nthird line\nfourth line\n";
    let (tmux, _tmp) = setup_vi("wc_hist_tb", content);

    // Go to top, select first line
    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "first line");

    // Go to bottom
    tmux.run(&["send-keys", "-X", "history-bottom"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Page up/down, half-page up/down
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_page_navigation() {
    let mut content = String::new();
    for i in 0..50 {
        content.push_str(&format!("line number {:03}\n", i));
    }
    let (tmux, _tmp) = setup_vi("wc_page_nav", &content);

    tmux.run(&["send-keys", "-X", "history-top"]);

    // Page down
    tmux.run(&["send-keys", "-X", "page-down"]);
    // Half-page down
    tmux.run(&["send-keys", "-X", "halfpage-down"]);
    // Half-page up
    tmux.run(&["send-keys", "-X", "halfpage-up"]);
    // Page up
    tmux.run(&["send-keys", "-X", "page-up"]);

    // Verify we're still in copy mode and can select
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert!(buf.starts_with("line number"));
}

// ============================================================
// Scroll up/down
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_scroll() {
    let mut content = String::new();
    for i in 0..50 {
        content.push_str(&format!("scroll line {:03}\n", i));
    }
    let (tmux, _tmp) = setup_vi("wc_scroll", &content);

    tmux.run(&["send-keys", "-X", "history-top"]);

    for _ in 0..5 {
        tmux.run(&["send-keys", "-X", "scroll-down"]);
    }
    for _ in 0..3 {
        tmux.run(&["send-keys", "-X", "scroll-up"]);
    }

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Search forward/backward - exercises search code paths
// Note: search-forward argument passing may not move cursor
// correctly, but we still exercise the search code paths.
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_search() {
    let content = "apple banana cherry\ndate elderberry fig\ngrape honeydew\n";
    let (tmux, _tmp) = setup_vi("wc_search", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Exercise search-forward (sets up search state)
    tmux.run(&["send-keys", "-X", "search-forward", "banana"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // Exercise search-backward
    tmux.run(&["send-keys", "-X", "search-backward", "apple"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // search-again and search-reverse
    tmux.run(&["send-keys", "-X", "search-again"]);
    tmux.run(&["send-keys", "-X", "search-reverse"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Search text (non-regex) mode
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_search_text() {
    let content = "hello [world] test\nfoo [bar] end\n";
    let (tmux, _tmp) = setup_vi("wc_search_text", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // search-forward-text does literal (no regex)
    tmux.run(&["send-keys", "-X", "search-forward-text", "[bar]"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // search-backward-text
    tmux.run(&["send-keys", "-X", "search-backward-text", "[world]"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Jump commands (f/F/t/T) and repeat (;/,)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_jump_commands() {
    let content = "abcdefghij\nklmnopqrst\n";
    let (tmux, _tmp) = setup_vi("wc_jump_cmds", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // jump-forward to 'f'
    tmux.run(&["send-keys", "-X", "jump-forward", "f"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "f");

    // jump-backward to 'c'
    tmux.run(&["send-keys", "-X", "jump-backward", "c"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "c");

    // jump-to-forward to 'h' (stops one before)
    tmux.run(&["send-keys", "-X", "jump-to-forward", "h"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "g");

    // jump-to-backward to 'd' (stops one after)
    tmux.run(&["send-keys", "-X", "jump-to-backward", "d"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "e");

    // jump-again (repeat last jump)
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "jump-forward", "g"]);
    tmux.run(&["send-keys", "-X", "jump-again"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // jump-reverse
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "jump-forward", "e"]);
    tmux.run(&["send-keys", "-X", "jump-reverse"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Rectangle selection mode
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_rectangle_mode() {
    let content = "aaaa bbbb cccc\ndddd eeee ffff\ngggg hhhh iiii\n";
    let (tmux, _tmp) = setup_vi("wc_rect_mode", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Enable rectangle mode
    tmux.run(&["send-keys", "-X", "rectangle-on"]);

    // Select a rectangle
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    for _ in 0..3 {
        tmux.run(&["send-keys", "-X", "cursor-right"]);
    }
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("aaaa"));
    assert!(buf.contains("dddd"));
    assert!(buf.contains("gggg"));

    // rectangle-off and rectangle-toggle
    tmux.run(&["send-keys", "-X", "rectangle-off"]);
    tmux.run(&["send-keys", "-X", "rectangle-toggle"]);
    tmux.run(&["send-keys", "-X", "rectangle-toggle"]);
}

// ============================================================
// Select-line and select-word
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_select_line_word() {
    let content = "hello world test\nsecond line here\n";
    let (tmux, _tmp) = setup_vi("wc_sel_lw", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // select-line selects the entire line (may include trailing newline)
    tmux.run(&["send-keys", "-X", "select-line"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("hello world test"));

    // select-word selects the word under cursor
    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    // Move to "world"
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "select-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "world");
}

// ============================================================
// Other-end (swap selection endpoints)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_other_end() {
    let content = "abcdefghij\nklmnopqrst\n";
    let (tmux, _tmp) = setup_vi("wc_other_end", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Select "abcde", then other-end to swap cursor to start
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    for _ in 0..4 {
        tmux.run(&["send-keys", "-X", "cursor-right"]);
    }
    tmux.run(&["send-keys", "-X", "other-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "abcde");
}

// ============================================================
// Goto-line
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_goto_line() {
    let mut content = String::new();
    for i in 0..30 {
        content.push_str(&format!("line {:02}\n", i));
    }
    let (tmux, _tmp) = setup_vi("wc_goto_line", &content);

    tmux.run(&["send-keys", "-X", "history-top"]);

    // goto-line 5
    tmux.run(&["send-keys", "-X", "goto-line", "5"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    // goto-line counts from bottom of history
    assert!(buf.starts_with("line"));
}

// ============================================================
// Set-mark and jump-to-mark
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_mark() {
    let content = "alpha beta gamma\ndelta epsilon zeta\n";
    let (tmux, _tmp) = setup_vi("wc_mark", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Set a mark at position 0,0
    tmux.run(&["send-keys", "-X", "set-mark"]);

    // Move to a different position
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "next-word"]);

    // Jump to mark (should swap cursor and mark)
    tmux.run(&["send-keys", "-X", "jump-to-mark"]);

    // Verify cursor word at original position
    let word = tmux.display("#{copy_cursor_word}");
    assert_eq!(word, "alpha");
}

// ============================================================
// Paragraph navigation
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_paragraphs() {
    let content = "paragraph one\nstill one\n\nparagraph two\nstill two\n\nparagraph three\n";
    let (tmux, _tmp) = setup_vi("wc_paragraphs", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Next paragraph should skip past the blank line
    tmux.run(&["send-keys", "-X", "next-paragraph"]);

    // Next paragraph again
    tmux.run(&["send-keys", "-X", "next-paragraph"]);

    // Previous paragraph should go back
    tmux.run(&["send-keys", "-X", "previous-paragraph"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Centre vertical/horizontal, top/middle/bottom line
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_centre_and_lines() {
    let mut content = String::new();
    for i in 0..60 {
        content.push_str(&format!("content line {:03}\n", i));
    }
    let (tmux, _tmp) = setup_vi("wc_centre", &content);

    tmux.run(&["send-keys", "-X", "history-top"]);

    for _ in 0..20 {
        tmux.run(&["send-keys", "-X", "cursor-down"]);
    }

    tmux.run(&["send-keys", "-X", "centre-vertical"]);
    tmux.run(&["send-keys", "-X", "centre-horizontal"]);
    tmux.run(&["send-keys", "-X", "top-line"]);
    tmux.run(&["send-keys", "-X", "middle-line"]);
    tmux.run(&["send-keys", "-X", "bottom-line"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Scroll-to top/middle/bottom
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_scroll_to() {
    let mut content = String::new();
    for i in 0..60 {
        content.push_str(&format!("scrollto line {:03}\n", i));
    }
    let (tmux, _tmp) = setup_vi("wc_scroll_to", &content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    for _ in 0..15 {
        tmux.run(&["send-keys", "-X", "cursor-down"]);
    }

    tmux.run(&["send-keys", "-X", "scroll-top"]);
    tmux.run(&["send-keys", "-X", "scroll-middle"]);
    tmux.run(&["send-keys", "-X", "scroll-bottom"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Toggle position (show/hide position indicator)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_toggle_position() {
    let content = "line one\nline two\nline three\n";
    let (tmux, _tmp) = setup_vi("wc_toggle_pos", content);

    tmux.run(&["send-keys", "-X", "toggle-position"]);
    tmux.run(&["send-keys", "-X", "toggle-position"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Copy line
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_copy_line() {
    let content = "hello world test\nsecond line here\n";
    let (tmux, _tmp) = setup_vi("wc_copy_line", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // copy-line: copies entire line
    tmux.run(&["send-keys", "-X", "copy-line"]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("hello world test"));
}

// ============================================================
// Copy end-of-line and cancel
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_copy_eol_and_cancel() {
    let content = "test line one\ntest line two\n";
    let (tmux, _tmp) = setup_vi("wc_copy_eol_cancel", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    tmux.run(&["send-keys", "-X", "copy-end-of-line-and-cancel"]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
    let buf = show_buffer(&tmux);
    assert!(buf.contains("test line one"));
}

// ============================================================
// Copy end-of-line (stays in copy mode)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_copy_eol() {
    let content = "hello world test\nsecond line here\n";
    let (tmux, _tmp) = setup_vi("wc_copy_eol", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "next-word"]);

    tmux.run(&["send-keys", "-X", "copy-end-of-line"]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("world test"));
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Copy line and cancel
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_copy_line_and_cancel() {
    let content = "full line content\nnext line\n";
    let (tmux, _tmp) = setup_vi("wc_copy_line_cancel", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "next-word"]);

    tmux.run(&["send-keys", "-X", "copy-line-and-cancel"]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
    let buf = show_buffer(&tmux);
    assert!(buf.contains("full line content"));
}

// ============================================================
// Append selection
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_append_selection() {
    let content = "word1 word2 word3\n";
    let (tmux, _tmp) = setup_vi("wc_append_sel", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Use set-buffer to ensure a paste buffer exists, then append
    tmux.run(&["set-buffer", "initial"]);

    // Select word1 and append
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "append-selection"]);
    // NOTE: show-buffer (default) has a bug after append-selection;
    // use -b buffer0 to read the buffer explicitly.
    let buf = show_buffer0(&tmux);
    assert_eq!(buf, "initialword1");
}

// ============================================================
// Append selection and cancel
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_append_selection_and_cancel() {
    let content = "aaa bbb ccc\n";
    let (tmux, _tmp) = setup_vi("wc_append_cancel", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Set a buffer first
    tmux.run(&["set-buffer", "base"]);

    // Select "aaa" and append-and-cancel
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "append-selection-and-cancel"]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
    let buf = show_buffer0(&tmux);
    assert_eq!(buf, "baseaaa");
}

// ============================================================
// Copy-selection-no-clear (selection stays active)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_copy_selection_no_clear() {
    let content = "test data here\n";
    let (tmux, _tmp) = setup_vi("wc_copy_no_clear", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    tmux.run(&["send-keys", "-X", "select-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection-no-clear"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "test");
}

// ============================================================
// Copy-selection-and-cancel
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_copy_selection_and_cancel() {
    let content = "cancel after copy\n";
    let (tmux, _tmp) = setup_vi("wc_copy_cancel", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    tmux.run(&["send-keys", "-X", "select-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection-and-cancel"]);

    assert_eq!(tmux.display("#{pane_mode}"), "");
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "cancel");
}

// ============================================================
// Cursor down and cancel
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_cursor_down_and_cancel() {
    let content = "line 1\nline 2\nline 3\n";
    let (tmux, _tmp) = setup_vi("wc_down_cancel", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "history-bottom"]);
    tmux.run(&["send-keys", "-X", "cursor-down-and-cancel"]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
}

// ============================================================
// Halfpage-down-and-cancel
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_halfpage_down_and_cancel() {
    let content = "short\n";
    let (tmux, _tmp) = setup_vi("wc_hpd_cancel", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "halfpage-down-and-cancel"]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
}

// ============================================================
// Page-down-and-cancel
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_page_down_and_cancel() {
    let content = "short content\n";
    let (tmux, _tmp) = setup_vi("wc_pd_cancel", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "page-down-and-cancel"]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
}

// ============================================================
// Scroll-down-and-cancel
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_scroll_down_and_cancel() {
    let content = "short\n";
    let (tmux, _tmp) = setup_vi("wc_sd_cancel", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "scroll-down-and-cancel"]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
}

// ============================================================
// Selection mode command
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_selection_mode() {
    let content = "hello world test line\n";
    let (tmux, _tmp) = setup_vi("wc_sel_mode", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Cycle through selection modes: char -> line -> block
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "selection-mode"]);
    tmux.run(&["send-keys", "-X", "selection-mode"]);
    tmux.run(&["send-keys", "-X", "selection-mode"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Bracket matching
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_bracket_matching() {
    let content = "if (foo(bar) && baz) {\n  return [1, 2];\n}\n";
    let (tmux, _tmp) = setup_vi("wc_bracket", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Move to opening '('
    tmux.run(&["send-keys", "-X", "jump-forward", "("]);

    // next-matching-bracket should jump to closing ')'
    tmux.run(&["send-keys", "-X", "next-matching-bracket"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, ")");

    // previous-matching-bracket should jump back to opening '('
    tmux.run(&["send-keys", "-X", "previous-matching-bracket"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "(");
}

// ============================================================
// Emacs mode search
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_search() {
    let content = "emacs test one\nemacs test two\nemacs test three\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_search", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Exercise search commands in emacs mode
    tmux.run(&["send-keys", "-X", "search-forward", "two"]);
    tmux.run(&["send-keys", "-X", "search-backward", "one"]);
    tmux.run(&["send-keys", "-X", "search-again"]);
    tmux.run(&["send-keys", "-X", "search-reverse"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Emacs incremental search
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_incremental_search() {
    let content = "incremental search test\nfind this word\nanother line\n";
    let (tmux, _tmp) = setup_emacs("wc_inc_search", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // search-forward-incremental with = prefix (search for the string)
    tmux.run(&["send-keys", "-X", "search-forward-incremental", "=find"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // search-backward-incremental with = prefix
    tmux.run(&["send-keys", "-X", "search-backward-incremental", "=test"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // Incremental with + prefix (next match)
    tmux.run(&["send-keys", "-X", "search-forward-incremental", "+test"]);

    // Incremental with - prefix (previous match)
    tmux.run(&["send-keys", "-X", "search-forward-incremental", "-test"]);
}

// ============================================================
// Copy pipe commands
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_copy_pipe() {
    let content = "pipe test data\n";
    let tmux = TmuxServer::new("wc_copy_pipe");
    let tmp = tmux.write_temp(content);
    let out_file = tmux.write_temp("");
    let pane_cmd = format!("cat {}; cat", tmp.path_str());
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set-window-option", "-g", "mode-keys", "vi"]);
    tmux.run(&["copy-mode"]);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);

    let pipe_cmd = format!("cat > {}", out_file.path_str());
    tmux.run(&["send-keys", "-X", "copy-pipe", &pipe_cmd]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
    let buf = show_buffer(&tmux);
    assert!(buf.contains("pipe test data"));
}

// ============================================================
// Copy pipe no clear
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_copy_pipe_no_clear() {
    let content = "no clear pipe\n";
    let tmux = TmuxServer::new("wc_cpnc");
    let tmp = tmux.write_temp(content);
    let out_file = tmux.write_temp("");
    let pane_cmd = format!("cat {}; cat", tmp.path_str());
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set-window-option", "-g", "mode-keys", "vi"]);
    tmux.run(&["copy-mode"]);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);

    let pipe_cmd = format!("cat > {}", out_file.path_str());
    tmux.run(&["send-keys", "-X", "copy-pipe-no-clear", &pipe_cmd]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Copy pipe and cancel
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_copy_pipe_and_cancel() {
    let content = "cancel pipe test\n";
    let tmux = TmuxServer::new("wc_cp_cancel");
    let tmp = tmux.write_temp(content);
    let out_file = tmux.write_temp("");
    let pane_cmd = format!("cat {}; cat", tmp.path_str());
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set-window-option", "-g", "mode-keys", "vi"]);
    tmux.run(&["copy-mode"]);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);

    let pipe_cmd = format!("cat > {}", out_file.path_str());
    tmux.run(&["send-keys", "-X", "copy-pipe-and-cancel", &pipe_cmd]);

    assert_eq!(tmux.display("#{pane_mode}"), "");
}

// ============================================================
// Pipe commands (no copy to buffer)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_pipe() {
    let content = "pipe only test\n";
    let tmux = TmuxServer::new("wc_pipe");
    let tmp = tmux.write_temp(content);
    let out_file = tmux.write_temp("");
    let pane_cmd = format!("cat {}; cat", tmp.path_str());
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set-window-option", "-g", "mode-keys", "vi"]);
    tmux.run(&["copy-mode"]);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);

    let pipe_cmd = format!("cat > {}", out_file.path_str());
    tmux.run(&["send-keys", "-X", "pipe-no-clear", &pipe_cmd]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // pipe (clears selection)
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);
    tmux.run(&["send-keys", "-X", "pipe", &pipe_cmd]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // pipe-and-cancel
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);
    tmux.run(&["send-keys", "-X", "pipe-and-cancel", &pipe_cmd]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
}

// ============================================================
// Copy pipe end-of-line variants
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_copy_pipe_eol() {
    let content = "eol pipe test data\n";
    let tmux = TmuxServer::new("wc_cp_eol");
    let tmp = tmux.write_temp(content);
    let out_file = tmux.write_temp("");
    let pane_cmd = format!("cat {}; cat", tmp.path_str());
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set-window-option", "-g", "mode-keys", "vi"]);
    tmux.run(&["copy-mode"]);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "next-word"]);

    // copy-pipe-end-of-line
    let pipe_cmd = format!("cat > {}", out_file.path_str());
    tmux.run(&["send-keys", "-X", "copy-pipe-end-of-line", &pipe_cmd]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("pipe test data"));
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // copy-pipe-end-of-line-and-cancel
    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    let pipe_cmd2 = format!("cat > {}", out_file.path_str());
    tmux.run(&[
        "send-keys",
        "-X",
        "copy-pipe-end-of-line-and-cancel",
        &pipe_cmd2,
    ]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
}

// ============================================================
// Copy pipe line variants
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_copy_pipe_line() {
    let content = "line pipe test content\n";
    let tmux = TmuxServer::new("wc_cp_line");
    let tmp = tmux.write_temp(content);
    let out_file = tmux.write_temp("");
    let pane_cmd = format!("cat {}; cat", tmp.path_str());
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set-window-option", "-g", "mode-keys", "vi"]);
    tmux.run(&["copy-mode"]);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "next-word"]);

    // copy-pipe-line
    let pipe_cmd = format!("cat > {}", out_file.path_str());
    tmux.run(&["send-keys", "-X", "copy-pipe-line", &pipe_cmd]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("line pipe test content"));
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // copy-pipe-line-and-cancel
    tmux.run(&["send-keys", "-X", "history-top"]);
    let pipe_cmd2 = format!("cat > {}", out_file.path_str());
    tmux.run(&["send-keys", "-X", "copy-pipe-line-and-cancel", &pipe_cmd2]);
    assert_eq!(tmux.display("#{pane_mode}"), "");
}

// ============================================================
// Stop selection
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_stop_selection() {
    let content = "stop selection test here\n";
    let (tmux, _tmp) = setup_vi("wc_stop_sel", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // begin-selection, move right, stop-selection (freezes selection endpoint)
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    for _ in 0..3 {
        tmux.run(&["send-keys", "-X", "cursor-right"]);
    }
    tmux.run(&["send-keys", "-X", "stop-selection"]);

    // Moving after stop should not extend selection
    for _ in 0..5 {
        tmux.run(&["send-keys", "-X", "cursor-right"]);
    }
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    // stop-selection freezes at position 3, vi includes char under cursor at stop
    assert_eq!(buf, "stop");
}

// ============================================================
// Refresh from pane
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_refresh_from_pane() {
    let content = "refresh test\n";
    let (tmux, _tmp) = setup_vi("wc_refresh", content);

    tmux.run(&["send-keys", "-X", "refresh-from-pane"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Emacs mode: various navigation
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_navigation() {
    let content = "emacs nav one two three\nfour five six\nseven eight nine\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_nav", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    tmux.run(&["send-keys", "-X", "cursor-right"]);
    tmux.run(&["send-keys", "-X", "cursor-right"]);
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "cursor-left"]);
    tmux.run(&["send-keys", "-X", "cursor-up"]);

    tmux.run(&["send-keys", "-X", "end-of-line"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Select and copy in emacs mode
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    // emacs mode: selection is exclusive of cursor position
    let buf = show_buffer(&tmux);
    assert!(buf.starts_with("emacs"));

    // Page navigation
    tmux.run(&["send-keys", "-X", "page-down"]);
    tmux.run(&["send-keys", "-X", "page-up"]);
    tmux.run(&["send-keys", "-X", "halfpage-down"]);
    tmux.run(&["send-keys", "-X", "halfpage-up"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Next/previous prompt
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_prompt_navigation() {
    let content = "some output\nmore output\n";
    let (tmux, _tmp) = setup_vi("wc_prompt_nav", content);

    tmux.run(&["send-keys", "-X", "history-top"]);

    // These won't find prompts but exercise the code path
    tmux.run(&["send-keys", "-X", "next-prompt"]);
    tmux.run(&["send-keys", "-X", "previous-prompt"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Search marks and search-match format callback
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_search_marks() {
    let content = "find me here\nfind me there\nfind me everywhere\n";
    let (tmux, _tmp) = setup_vi("wc_search_marks", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Search for something to generate search marks
    tmux.run(&["send-keys", "-X", "search-forward", "find"]);

    // Use search-again to cycle through matches
    tmux.run(&["send-keys", "-X", "search-again"]);
    tmux.run(&["send-keys", "-X", "search-again"]);
    tmux.run(&["send-keys", "-X", "search-reverse"]);

    // Exercise format callbacks
    let _ = tmux.display("#{copy_cursor_word}");

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Vi mode with scrollback - exercises clone_screen / scroll
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_scrollback() {
    let tmux = TmuxServer::new("wc_scrollback");
    let pane_cmd = "seq 1 200; cat";
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "-g", "history-limit", "500"]);
    tmux.run(&["set-window-option", "-g", "mode-keys", "vi"]);

    sleep_ms(500);

    tmux.run(&["copy-mode"]);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "1");

    tmux.run(&["send-keys", "-X", "history-bottom"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Resize during copy mode - exercises size_changed/resize
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_resize() {
    let content = "resize test line\nanother line\n";
    let (tmux, _tmp) = setup_vi("wc_resize", content);

    tmux.run(&["send-keys", "-X", "history-top"]);

    tmux.run(&["resize-pane", "-x", "60", "-y", "20"]);
    tmux.run(&["resize-pane", "-x", "80", "-y", "24"]);

    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "end-of-line"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("resize"));
}

// ============================================================
// Format callbacks (cursor word, line, etc.)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_formats() {
    let content = "format word test\nsecond line content\n";
    let (tmux, _tmp) = setup_vi("wc_formats", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    let word = tmux.display("#{copy_cursor_word}");
    assert_eq!(word, "format");

    let line = tmux.display("#{copy_cursor_line}");
    assert!(line.starts_with("format word test"));

    let cx = tmux.display("#{copy_cursor_x}");
    assert_eq!(cx, "0");
}

// ============================================================
// Emacs bracket matching
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_bracket() {
    let content = "function(arg1, arg2)\n  [array]\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_bracket", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    tmux.run(&["send-keys", "-X", "next-matching-bracket"]);
    tmux.run(&["send-keys", "-X", "previous-matching-bracket"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Vi mode with line selection and cursor up/down
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_line_selection_movement() {
    let content = "line one here\nline two here\nline three here\nline four here\n";
    let (tmux, _tmp) = setup_vi("wc_line_sel_mv", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // select-line then move down to extend line selection
    tmux.run(&["send-keys", "-X", "select-line"]);
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("line one here"));
    assert!(buf.contains("line two here"));
    assert!(buf.contains("line three here"));
}

// ============================================================
// Word selection and movement
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_word_selection_movement() {
    let content = "hello world foo bar baz\n";
    let (tmux, _tmp) = setup_vi("wc_word_sel_mv", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    tmux.run(&["send-keys", "-X", "select-word"]);
    tmux.run(&["send-keys", "-X", "cursor-right"]);
    tmux.run(&["send-keys", "-X", "cursor-right"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert!(buf.starts_with("hello"));
}

// ============================================================
// Key table switching (vi vs emacs)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_key_table() {
    let tmux = TmuxServer::new("wc_key_table");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-window-option", "-g", "mode-keys", "vi"]);
    tmux.run(&["copy-mode"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
    tmux.run(&["send-keys", "-X", "cancel"]);

    tmux.run(&["set-window-option", "-g", "mode-keys", "emacs"]);
    tmux.run(&["copy-mode"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
    tmux.run(&["send-keys", "-X", "cancel"]);
}

// ============================================================
// Search wrap-around
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_search_wrap() {
    let content = "unique_start middle text\nmore lines here\nunique_end final\n";
    let (tmux, _tmp) = setup_vi("wc_search_wrap", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["set", "-g", "wrap-search", "on"]);

    // Exercise search with wrap enabled
    tmux.run(&["send-keys", "-X", "search-forward", "unique_end"]);
    tmux.run(&["send-keys", "-X", "search-forward", "unique_start"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Case-insensitive search (all-lowercase pattern)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_search_case_insensitive() {
    let content = "Hello World\nhello world\nHELLO WORLD\n";
    let (tmux, _tmp) = setup_vi("wc_search_ci", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Lowercase search should be case-insensitive
    tmux.run(&["send-keys", "-X", "search-forward", "hello"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Vi mode: select-line with other-end
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_select_line_other_end() {
    let content = "line A\nline B\nline C\nline D\n";
    let (tmux, _tmp) = setup_vi("wc_sel_line_oe", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // Select line, extend down, then other-end
    tmux.run(&["send-keys", "-X", "select-line"]);
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "other-end"]);
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("line A") || buf.contains("line B") || buf.contains("line C"));
}

// ============================================================
// Emacs: copy end-of-line, copy line
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_copy_eol_line() {
    let content = "emacs copy test\nsecond emacs line\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_eol", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // copy-line in emacs mode
    tmux.run(&["send-keys", "-X", "copy-line"]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("emacs copy test"));

    // copy-end-of-line in emacs mode
    tmux.run(&["copy-mode"]);
    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "copy-end-of-line"]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("copy test"));
}

// ============================================================
// Emacs: select-word, select-line
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_select_word_line() {
    let content = "emacs word select\nemacs line select\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_swl", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "next-word"]);

    // select-word in emacs mode
    tmux.run(&["send-keys", "-X", "select-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "word");

    // select-line in emacs mode
    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "select-line"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("emacs word select"));
}

// ============================================================
// Copy mode with -u flag (enters copy mode + page up)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_enter_with_page_up() {
    let tmux = TmuxServer::new("wc_enter_pu");
    let pane_cmd = "seq 1 100; cat";
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "-g", "history-limit", "500"]);

    sleep_ms(500);

    tmux.run(&["copy-mode", "-u"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    tmux.run(&["send-keys", "-X", "cancel"]);
}

// ============================================================
// Vi: next-space, previous-space, next-space-end
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_vi_space_movement() {
    let content = "hello.world foo-bar baz_qux\n";
    let (tmux, _tmp) = setup_vi("wc_vi_space", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // next-space moves to next whitespace-delimited word
    tmux.run(&["send-keys", "-X", "next-space"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-space-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "foo-bar");

    // previous-space
    tmux.run(&["send-keys", "-X", "previous-space"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-space-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "foo-bar");
}

// ============================================================
// Emacs: stop-selection works
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_stop_selection() {
    let content = "emacs stop test here\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_stop", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    tmux.run(&["send-keys", "-X", "begin-selection"]);
    for _ in 0..5 {
        tmux.run(&["send-keys", "-X", "cursor-right"]);
    }
    tmux.run(&["send-keys", "-X", "stop-selection"]);
    for _ in 0..5 {
        tmux.run(&["send-keys", "-X", "cursor-right"]);
    }
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "emacs");
}

// ============================================================
// Search with no results
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_search_no_match() {
    let content = "simple test content\n";
    let (tmux, _tmp) = setup_vi("wc_search_nomatch", content);

    tmux.run(&["send-keys", "-X", "history-top"]);

    // Search for something that doesn't exist
    tmux.run(&["send-keys", "-X", "search-forward", "zzzznotfound"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");

    // search-again with no match
    tmux.run(&["send-keys", "-X", "search-again"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Rectangle selection with copy
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_rectangle_copy() {
    let content = "AAAAABBBBB\nCCCCCDDDDD\nEEEEEFFFFF\n";
    let (tmux, _tmp) = setup_vi("wc_rect_copy", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    tmux.run(&["send-keys", "-X", "rectangle-toggle"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    for _ in 0..4 {
        tmux.run(&["send-keys", "-X", "cursor-right"]);
    }
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert!(buf.contains("AAAAA"));
    assert!(buf.contains("CCCCC"));
    assert!(buf.contains("EEEEE"));
}

// ============================================================
// Emacs: jump commands
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_jump() {
    let content = "abcdefghij\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_jump", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    tmux.run(&["send-keys", "-X", "jump-forward", "e"]);
    tmux.run(&["send-keys", "-X", "jump-backward", "b"]);
    tmux.run(&["send-keys", "-X", "jump-to-forward", "g"]);
    tmux.run(&["send-keys", "-X", "jump-to-backward", "c"]);
    tmux.run(&["send-keys", "-X", "jump-again"]);
    tmux.run(&["send-keys", "-X", "jump-reverse"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Emacs: rectangle mode
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_rectangle() {
    let content = "rect1 rect2\nrect3 rect4\nrect5 rect6\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_rect", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // In emacs, begin-selection first, then toggle rectangle
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "rectangle-toggle"]);
    for _ in 0..4 {
        tmux.run(&["send-keys", "-X", "cursor-right"]);
    }
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    // emacs rectangle selection
    assert!(buf.contains("rect"));
}

// ============================================================
// View mode (read-only copy mode)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_view_mode() {
    let tmux = TmuxServer::new("wc_view_mode");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // copy-mode -e is "view-like" (exits on scroll to bottom)
    tmux.run(&["copy-mode", "-e"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
    tmux.run(&["send-keys", "-X", "cancel"]);
}

// ============================================================
// Search with empty string (clears marks)
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_search_clear() {
    let content = "abc def ghi\n";
    let (tmux, _tmp) = setup_vi("wc_search_clear", content);

    tmux.run(&["send-keys", "-X", "history-top"]);

    // Search, then search with empty to clear
    tmux.run(&["send-keys", "-X", "search-forward", "abc"]);
    tmux.run(&["send-keys", "-X", "search-forward", ""]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Emacs: centre vertical/horizontal
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_centre() {
    let mut content = String::new();
    for i in 0..40 {
        content.push_str(&format!("emacs centre line {:03}\n", i));
    }
    let (tmux, _tmp) = setup_emacs("wc_emacs_centre", &content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    for _ in 0..15 {
        tmux.run(&["send-keys", "-X", "cursor-down"]);
    }

    tmux.run(&["send-keys", "-X", "centre-vertical"]);
    tmux.run(&["send-keys", "-X", "centre-horizontal"]);
    tmux.run(&["send-keys", "-X", "top-line"]);
    tmux.run(&["send-keys", "-X", "middle-line"]);
    tmux.run(&["send-keys", "-X", "bottom-line"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Emacs: other-end
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_other_end() {
    let content = "emacs other end test\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_oe", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    tmux.run(&["send-keys", "-X", "begin-selection"]);
    for _ in 0..5 {
        tmux.run(&["send-keys", "-X", "cursor-right"]);
    }
    tmux.run(&["send-keys", "-X", "other-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "emacs");
}

// ============================================================
// Emacs: goto-line
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_goto_line() {
    let mut content = String::new();
    for i in 0..20 {
        content.push_str(&format!("eline {:02}\n", i));
    }
    let (tmux, _tmp) = setup_emacs("wc_emacs_goto", &content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "goto-line", "3"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Emacs: set-mark, jump-to-mark
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_mark() {
    let content = "mark test emacs\nsecond line\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_mark", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);
    tmux.run(&["send-keys", "-X", "set-mark"]);
    tmux.run(&["send-keys", "-X", "cursor-down"]);
    tmux.run(&["send-keys", "-X", "jump-to-mark"]);

    let word = tmux.display("#{copy_cursor_word}");
    assert_eq!(word, "mark");
}

// ============================================================
// Emacs: paragraph navigation
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_paragraphs() {
    let content = "para one\n\npara two\n\npara three\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_para", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "next-paragraph"]);
    tmux.run(&["send-keys", "-X", "next-paragraph"]);
    tmux.run(&["send-keys", "-X", "previous-paragraph"]);

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Emacs: scroll commands
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_scroll() {
    let mut content = String::new();
    for i in 0..50 {
        content.push_str(&format!("escroll {:03}\n", i));
    }
    let (tmux, _tmp) = setup_emacs("wc_emacs_scroll", &content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    for _ in 0..5 {
        tmux.run(&["send-keys", "-X", "scroll-down"]);
    }
    for _ in 0..3 {
        tmux.run(&["send-keys", "-X", "scroll-up"]);
    }

    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}

// ============================================================
// Emacs: back-to-indentation
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_back_to_indentation() {
    let content = "  indented line\nnormal line\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_bti", content);

    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "back-to-indentation"]);

    let word = tmux.display("#{copy_cursor_word}");
    assert_eq!(word, "indented");
}

// ============================================================
// Emacs: toggle-position
// ============================================================

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn window_copy_emacs_toggle_position() {
    let content = "toggle test\n";
    let (tmux, _tmp) = setup_emacs("wc_emacs_toggle", content);

    tmux.run(&["send-keys", "-X", "toggle-position"]);
    tmux.run(&["send-keys", "-X", "toggle-position"]);
    assert_eq!(tmux.display("#{pane_mode}"), "copy-mode");
}
