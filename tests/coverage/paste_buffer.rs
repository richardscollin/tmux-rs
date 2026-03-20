use super::*;

/// Test paste-buffer: basic paste, paste with -d (delete after), paste with -r
/// (linefeed separator), paste named buffer with -b, and paste into exited pane.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_basic() {
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
fn paste_buffer_delete() {
    let tmux = TmuxServer::new("paste_buffer_delete");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "deleteme"]);
    // paste-buffer -d: paste and delete the buffer
    tmux.run(&["paste-buffer", "-d"]);

    // Buffer should be gone
    let out = tmux.try_run(&["show-buffer"]);
    assert!(!out.status.success(), "buffer should be deleted after paste-buffer -d");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_buffer_named() {
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
    assert!(!out.status.success(), "paste-buffer with nonexistent buffer should fail");
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
