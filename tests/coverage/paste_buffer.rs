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
