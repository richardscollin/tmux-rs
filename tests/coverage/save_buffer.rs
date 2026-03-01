use super::*;

/// Save buffer to a file (truncate mode).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn save_buffer_basic() {
    let tmux = TmuxServer::new("saveb_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "hello world"]);
    let tmp = tmux.write_temp("");
    tmux.run(&["saveb", tmp.path_str()]);
    assert_eq!(tmp.read_to_string(), "hello world");
}

/// Save buffer with -a (append mode).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn save_buffer_append() {
    let tmux = TmuxServer::new("saveb_append");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "first"]);
    let tmp = tmux.write_temp("");
    tmux.run(&["saveb", tmp.path_str()]);
    tmux.run(&["set-buffer", "second"]);
    tmux.run(&["saveb", "-a", tmp.path_str()]);
    assert_eq!(tmp.read_to_string(), "firstsecond");
}

/// Save a named buffer with -b.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn save_buffer_named() {
    let tmux = TmuxServer::new("saveb_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "-b", "mybuf", "named content"]);
    let tmp = tmux.write_temp("");
    tmux.run(&["saveb", "-b", "mybuf", tmp.path_str()]);
    assert_eq!(tmp.read_to_string(), "named content");
}

/// Save buffer with nonexistent named buffer.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn save_buffer_nonexistent_name() {
    let tmux = TmuxServer::new("saveb_noname");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["saveb", "-b", "nosuchbuf", "/dev/null"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no buffer"),
        "expected 'no buffer' error, got: {stderr}"
    );
}

/// Save buffer with no buffers at all.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn save_buffer_no_buffers() {
    let tmux = TmuxServer::new("saveb_nobufs");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Delete all buffers by looping until none remain
    for _ in 0..20 {
        let result = tmux.try_run(&["delete-buffer"]);
        if !result.status.success() {
            break;
        }
    }
    let result = tmux.try_run(&["saveb", "/dev/null"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no buffers"),
        "expected 'no buffers' error, got: {stderr}"
    );
}

/// Show buffer via control-mode client (exercises CMD_SHOW_BUFFER_ENTRY + CONTROL path).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_buffer_control_mode() {
    let tmux = TmuxServer::new("showb_ctrl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "control test data"]);
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"show-buffer\ndetach-client\n",
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("control test data"),
        "show-buffer output should contain buffer data, got: {stdout}"
    );
}

/// Show buffer with named buffer via control mode.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_buffer_named_control() {
    let tmux = TmuxServer::new("showb_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "-b", "test", "named buf data"]);
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"show-buffer -b test\ndetach-client\n",
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("named buf data"),
        "expected buffer data, got: {stdout}"
    );
}

/// Show buffer to stdout (- path, non-control non-session client).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_buffer_stdout() {
    let tmux = TmuxServer::new("showb_stdout");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "stdout data"]);
    let out = tmux.run(&["showb"]);
    assert_eq!(out.trim(), "stdout data");
}

/// Save buffer to an invalid path (exercises error callback).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn save_buffer_invalid_path() {
    let tmux = TmuxServer::new("saveb_badpath");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "test"]);
    let result = tmux.try_run(&["saveb", "/no/such/directory/file"]);
    assert!(!result.status.success());
}
