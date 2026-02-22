use super::*;

/// Strip trailing newlines from show-buffer output.
/// Shell $(...) strips ALL trailing newlines, so we do the same.
fn show_buffer(tmux: &TmuxServer) -> String {
    tmux.run(&["show-buffer"])
        .trim_end_matches('\n')
        .to_string()
}

/// Ported from copy-mode-test-emacs.sh
#[test]
fn copy_mode_emacs() {
    let tmux = TmuxServer::new("copy_mode_emacs");

    let test_file = TmuxServer::regress_dir().join("copy-mode-test.txt");
    let pane_cmd = format!("cat {}; printf '\\e[9;15H'; cat", test_file.display());

    tmux.run(&["-f/dev/null", "new", "-d", "-x40", "-y10", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy mode and go to the first column of the first row.
    tmux.run(&["set-window-option", "-g", "mode-keys", "emacs"]);
    tmux.run(&["set-window-option", "-g", "word-separators", ""]);
    tmux.run(&["copy-mode"]);
    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // 1. Test that `previous-word` and `previous-space`
    //    do not go past the start of text.
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "previous-word"]);
    tmux.run(&["send-keys", "-X", "previous-space"]);
    tmux.run(&["send-keys", "-X", "previous-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let output = tmux.try_run(&["show-buffer"]);
    let buf = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(buf, "");

    // 2. Test that `next-word-end` does not skip single-letter words.
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "previous-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "A");

    // 3. Test that `next-word-end` wraps around indented line breaks.
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "words\n\tIndented");

    // 4. Test that `next-word` wraps around un-indented line breaks.
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "line");

    // 5. Test that `next-word-end` treats periods as letters.
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "line...");

    // 6. Test that `previous-word` and `next-word` treat periods as letters.
    tmux.run(&["send-keys", "-X", "previous-word"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "line...");

    // 7. Test that `previous-space` and `next-space` treat periods as letters.
    tmux.run(&["send-keys", "-X", "previous-space"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-space"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "line...");

    // 8. Test that `next-word` and `next-word-end` treat other symbols as letters.
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "... @nd then $ym_bols[]{}");

    // 9. Test that `previous-word` treats other symbols as letters
    //    and `next-word` wraps around for indented symbols.
    tmux.run(&["send-keys", "-X", "previous-word"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "$ym_bols[]{}\n ");

    // 10. Test that `next-word-end` treats digits as letters.
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, " 500xyz");

    // 11. Test that `previous-word` treats digits as letters.
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "previous-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "500xyz");

    // 12. Test that `next-word` and `next-word-end` stop at the end of text.
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "next-space"]);
    tmux.run(&["send-keys", "-X", "next-space-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "500xyz");

    tmux.kill_server();
}

/// Ported from copy-mode-test-vi.sh
#[test]
fn copy_mode_vi() {
    let tmux = TmuxServer::new("copy_mode_vi");

    let test_file = TmuxServer::regress_dir().join("copy-mode-test.txt");
    let pane_cmd = format!("cat {}; printf '\\e[9;15H'; cat", test_file.display());

    tmux.run(&["-f/dev/null", "new", "-d", "-x40", "-y10", &pane_cmd]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Enter copy mode and go to the first column of the first row.
    tmux.run(&["set-window-option", "-g", "mode-keys", "vi"]);
    tmux.run(&["copy-mode"]);
    tmux.run(&["send-keys", "-X", "history-top"]);
    tmux.run(&["send-keys", "-X", "start-of-line"]);

    // 1. Test that `previous-word` and `previous-space`
    //    do not go past the start of text (vi includes cursor char).
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "previous-word"]);
    tmux.run(&["send-keys", "-X", "previous-space"]);
    tmux.run(&["send-keys", "-X", "previous-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "A");

    // 2. Test that `next-word-end` skips single-letter words
    //    and `previous-word` does not skip multi-letter words.
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "previous-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "line");

    // 3. Test that `next-word-end` wraps around indented line breaks.
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "words\n\tIndented");

    // 4. Test that `next-word` wraps around un-indented line breaks.
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "line\nA");

    // 5. Test that `next-word-end` does not treat periods as letters (vi mode).
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "line");

    // 6. Test that `next-space-end` treats periods as letters.
    tmux.run(&["send-keys", "-X", "previous-word"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-space-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "line...");

    // 7. Test that `previous-space` and `next-space` treat periods as letters.
    tmux.run(&["send-keys", "-X", "previous-space"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-space"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "line...\n.");

    // 8. Test that `next-word` and `next-word-end` do not treat other symbols
    //    as letters (vi mode).
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "... @nd then");

    // 9. Test that `next-space` wraps around for indented symbols.
    tmux.run(&["send-keys", "-X", "next-space"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-space"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "$ym_bols[]{}\n ?");

    // 10. Test that `next-word-end` treats digits as letters.
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "? 500xyz");

    // 11. Test that `previous-word` treats digits as letters.
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "previous-word"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "500xyz");

    // 12. Test that `next-word`, `next-word-end`,
    //     `next-space`, and `next-space-end` stop at the end of text.
    tmux.run(&["send-keys", "-X", "begin-selection"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "next-word-end"]);
    tmux.run(&["send-keys", "-X", "next-word"]);
    tmux.run(&["send-keys", "-X", "next-space"]);
    tmux.run(&["send-keys", "-X", "next-space-end"]);
    tmux.run(&["send-keys", "-X", "copy-selection"]);
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "500xyz");

    tmux.kill_server();
}
