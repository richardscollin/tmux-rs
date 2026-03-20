use super::*;

/// Test save-buffer and show-buffer.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_buffer_default() {
    let tmux = TmuxServer::new("showb_default");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "hello world"]);
    let out = tmux.run(&["show-buffer"]);
    assert_eq!(out.trim(), "hello world");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_buffer_named() {
    let tmux = TmuxServer::new("showb_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "-b", "mybuf", "named data"]);
    let out = tmux.run(&["show-buffer", "-b", "mybuf"]);
    assert_eq!(out.trim(), "named data");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_buffer_nonexistent() {
    let tmux = TmuxServer::new("showb_noexist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.try_run(&["show-buffer", "-b", "nosuch"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no buffer"),
        "expected 'no buffer' error, got: {stderr}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_buffer_no_buffers() {
    let tmux = TmuxServer::new("showb_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.try_run(&["show-buffer"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no buffer"),
        "expected 'no buffer' error, got: {stderr}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn save_buffer_to_file() {
    let tmux = TmuxServer::new("saveb_file");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("");
    tmux.run(&["set-buffer", "file content"]);
    tmux.run(&["save-buffer", tmp.path_str()]);

    sleep_ms(200);
    let content = tmp.read_to_string();
    assert_eq!(content, "file content");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn save_buffer_append() {
    let tmux = TmuxServer::new("saveb_append");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("existing");
    tmux.run(&["set-buffer", " appended"]);
    tmux.run(&["save-buffer", "-a", tmp.path_str()]);

    sleep_ms(200);
    let content = tmp.read_to_string();
    assert_eq!(content, "existing appended");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn save_buffer_named() {
    let tmux = TmuxServer::new("saveb_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("");
    tmux.run(&["set-buffer", "-b", "saveme", "named save"]);
    tmux.run(&["save-buffer", "-b", "saveme", tmp.path_str()]);

    sleep_ms(200);
    let content = tmp.read_to_string();
    assert_eq!(content, "named save");
}
