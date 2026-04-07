use super::*;

/// Basic paste-buffer: paste default (top) buffer into pane.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_basic() {
    let tmux = TmuxServer::new("pasteb_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "hello"]);
    tmux.run(&["pasteb"]);

    // Buffer content should have been written to the pane
    let out = tmux.run(&["capture-pane", "-p"]);
    assert!(
        out.contains("hello"),
        "expected pane to contain 'hello', got: {out}"
    );
}

/// Paste named buffer with -b.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_named() {
    let tmux = TmuxServer::new("pasteb_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "-b", "mybuf", "named_data"]);
    tmux.run(&["pasteb", "-b", "mybuf"]);

    let out = tmux.run(&["capture-pane", "-p"]);
    assert!(
        out.contains("named_data"),
        "expected pane to contain 'named_data', got: {out}"
    );
}

/// Paste nonexistent named buffer gives error.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_nonexistent() {
    let tmux = TmuxServer::new("pasteb_noexist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["pasteb", "-b", "nosuchbuf"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no buffer"),
        "expected 'no buffer' error, got: {stderr}"
    );
}

/// Paste with -d deletes the buffer after pasting.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_delete() {
    let tmux = TmuxServer::new("pasteb_delete");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "-b", "delbuf", "deldata"]);
    tmux.run(&["pasteb", "-d", "-b", "delbuf"]);

    // Buffer should be deleted after paste
    let result = tmux.try_run(&["showb", "-b", "delbuf"]);
    assert!(!result.status.success());
}

/// Paste with -r uses \n as separator instead of \r.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_raw_separator() {
    let tmux = TmuxServer::new("pasteb_raw");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Buffer with newlines - with -r the separator is \n
    tmux.run(&["setb", "line1\nline2"]);
    tmux.run(&["pasteb", "-r"]);

    let out = tmux.run(&["capture-pane", "-p"]);
    assert!(
        out.contains("line1"),
        "expected pane to contain 'line1', got: {out}"
    );
}

/// Paste with custom separator -s.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_custom_separator() {
    let tmux = TmuxServer::new("pasteb_sep");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "a\nb"]);
    tmux.run(&["pasteb", "-s", ","]);

    let out = tmux.run(&["capture-pane", "-p"]);
    // With separator "," instead of default \r, newlines become commas
    assert!(
        out.contains("a,b"),
        "expected pane to contain 'a,b', got: {out}"
    );
}

/// Paste with -p for bracket paste mode.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_bracket_paste() {
    let tmux = TmuxServer::new("pasteb_bracket");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "bracketed"]);
    // -p enables bracket paste wrapping
    tmux.run(&["pasteb", "-p"]);

    let out = tmux.run(&["capture-pane", "-p"]);
    assert!(
        out.contains("bracketed"),
        "expected pane to contain 'bracketed', got: {out}"
    );
}

/// Paste when no buffers exist (no -b flag).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_no_buffers() {
    let tmux = TmuxServer::new("pasteb_nobuf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // No buffers set, paste should be a no-op (pb is null)
    tmux.run(&["pasteb"]);
}

/// Paste buffer whose content ends with newline (bufdata == bufend after loop).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_trailing_newline() {
    let tmux = TmuxServer::new("pasteb_trailing");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Buffer content ends with \n, so after loop bufdata == bufend
    tmux.run(&["setb", "line1\n"]);
    tmux.run(&["pasteb"]);

    let out = tmux.run(&["capture-pane", "-p"]);
    assert!(
        out.contains("line1"),
        "expected pane to contain 'line1', got: {out}"
    );
}

/// Paste to exited pane gives error.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_exited_pane() {
    let tmux = TmuxServer::new("pasteb_exited");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "data"]);
    // Create a second pane with remain-on-exit so it stays findable
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["set", "-p", "-t", ":.1", "remain-on-exit", "on"]);
    tmux.run(&["send-keys", "-t", ":.1", "exit", "Enter"]);
    std::thread::sleep(std::time::Duration::from_millis(500));

    let result = tmux.try_run(&["pasteb", "-t", ":.1"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("has exited"),
        "expected 'has exited' error, got: {stderr}"
    );
}

/// Test paste-buffer: basic paste, paste with -d (delete after), paste with -r
/// (linefeed separator), paste named buffer with -b, and paste into exited pane.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_basic_02() {
    let tmux = TmuxServer::new("paste_buffer_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a buffer and verify it exists
    tmux.run(&["set-buffer", "hello"]);
    let buf = tmux.run(&["show-buffer"]);
    assert_eq!(buf.trim(), "hello");

    // paste-buffer (default): pastes top buffer
    tmux.run(&["paste-buffer"]);

    // Buffer should still exist (no -d)
    let buf = tmux.run(&["show-buffer"]);
    assert_eq!(buf.trim(), "hello");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_delete_02() {
    let tmux = TmuxServer::new("paste_buffer_delete");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "deleteme"]);
    // paste-buffer -d: paste and delete the buffer
    tmux.run(&["paste-buffer", "-d"]);

    // Buffer should be gone
    let out = tmux.try_run(&["show-buffer"]);
    assert!(
        !out.status.success(),
        "buffer should be deleted after paste-buffer -d"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_named_02() {
    let tmux = TmuxServer::new("paste_buffer_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set named buffers
    tmux.run(&["set-buffer", "-b", "mybuf", "named-content"]);
    tmux.run(&["set-buffer", "-b", "other", "other-content"]);

    // paste-buffer -b mybuf: paste specific named buffer
    tmux.run(&["paste-buffer", "-b", "mybuf"]);

    // Both buffers should still exist
    let buf = tmux.run(&["show-buffer", "-b", "mybuf"]);
    assert_eq!(buf.trim(), "named-content");
    let buf = tmux.run(&["show-buffer", "-b", "other"]);
    assert_eq!(buf.trim(), "other-content");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_named_nonexistent() {
    let tmux = TmuxServer::new("paste_buffer_noexist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // paste-buffer -b nonexistent: should fail
    let out = tmux.try_run(&["paste-buffer", "-b", "nosuchbuffer"]);
    assert!(
        !out.status.success(),
        "paste-buffer with nonexistent buffer should fail"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_separator() {
    let tmux = TmuxServer::new("paste_buffer_sep");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a multi-line buffer
    tmux.run(&["set-buffer", "line1\nline2\nline3"]);

    // paste-buffer -r: use \n separator instead of \r
    tmux.run(&["paste-buffer", "-r"]);

    // paste-buffer -s ',': use custom separator
    tmux.run(&["set-buffer", "a\nb\nc"]);
    tmux.run(&["paste-buffer", "-s", ","]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_bracket_mode() {
    let tmux = TmuxServer::new("paste_buffer_bracket");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "bracketed"]);
    // paste-buffer -p: bracket paste mode
    tmux.run(&["paste-buffer", "-p"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_no_buffer() {
    let tmux = TmuxServer::new("paste_buffer_nobuf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Delete buffers one at a time until none remain
    loop {
        let out = tmux.try_run(&["delete-buffer"]);
        if !out.status.success() {
            break;
        }
    }

    // paste-buffer with no buffers: should be a no-op (top buffer is NULL)
    // This shouldn't crash
    let out = tmux.try_run(&["paste-buffer"]);
    let _ = out;
}
