use super::*;

/// Test paste buffer operations to exercise paste.rs edge cases.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_set_overwrite() {
    let tmux = TmuxServer::new("paste_ow");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set a named buffer, then overwrite with new data
    tmux.run(&["set-buffer", "-b", "ow", "first"]);
    tmux.run(&["set-buffer", "-b", "ow", "second"]);
    let out = tmux.run(&["show-buffer", "-b", "ow"]);
    assert_eq!(out.trim(), "second");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_many_auto_buffers() {
    let tmux = TmuxServer::new("paste_many");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create many auto-named buffers to exercise buffer limit/cleanup
    for i in 0..20 {
        tmux.run(&["set-buffer", &format!("data{i}")]);
    }
    let out = tmux.run(&["list-buffers"]);
    let count = out.trim().lines().count();
    assert!(count >= 10, "should have multiple buffers, got {count}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_rename_named() {
    let tmux = TmuxServer::new("paste_rn");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Rename a named buffer (doesn't go through paste_get_top, so no BUG-001)
    tmux.run(&["set-buffer", "-b", "oldbuf", "data"]);
    tmux.run(&["set-buffer", "-b", "oldbuf", "-n", "newbuf"]);
    let out = tmux.run(&["show-buffer", "-b", "newbuf"]);
    assert_eq!(out.trim(), "data");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_rename_nonexistent() {
    let tmux = TmuxServer::new("paste_rnne");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Rename nonexistent buffer: should error
    let out = tmux.try_run(&["set-buffer", "-b", "nosuch", "-n", "new"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown buffer"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_rename_to_empty() {
    let tmux = TmuxServer::new("paste_rnempty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Rename to empty name
    tmux.run(&["set-buffer", "-b", "src", "data"]);
    let out = tmux.try_run(&["set-buffer", "-b", "src", "-n", ""]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_rename_to_existing() {
    let tmux = TmuxServer::new("paste_rnexist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Rename to existing buffer name — should replace target
    tmux.run(&["set-buffer", "-b", "a", "data_a"]);
    tmux.run(&["set-buffer", "-b", "b", "data_b"]);
    tmux.run(&["set-buffer", "-b", "a", "-n", "b"]);

    // "a" should be gone, "b" should have "data_a"
    let out = tmux.try_run(&["show-buffer", "-b", "a"]);
    assert!(!out.status.success(), "old buffer should not exist");

    let out = tmux.run(&["show-buffer", "-b", "b"]);
    assert_eq!(out.trim(), "data_a");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_rename_same_name() {
    let tmux = TmuxServer::new("paste_rnsame");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Rename buffer to itself — should be a no-op
    tmux.run(&["set-buffer", "-b", "same", "data"]);
    tmux.run(&["set-buffer", "-b", "same", "-n", "same"]);
    let out = tmux.run(&["show-buffer", "-b", "same"]);
    assert_eq!(out.trim(), "data");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_list_format() {
    let tmux = TmuxServer::new("paste_lstfmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "-b", "fmtbuf", "hello world"]);
    let out = tmux.run(&["list-buffers", "-F", "#{buffer_name}: #{buffer_size}"]);
    assert!(out.contains("fmtbuf: 11"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_set_empty_name() {
    let tmux = TmuxServer::new("paste_emptyname");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set buffer with empty name
    let out = tmux.try_run(&["set-buffer", "-b", "", "data"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn paste_delete_named() {
    let tmux = TmuxServer::new("paste_delnamed");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set-buffer", "-b", "delbuf", "data"]);
    tmux.run(&["delete-buffer", "-b", "delbuf"]);

    let out = tmux.try_run(&["show-buffer", "-b", "delbuf"]);
    assert!(!out.status.success());
}
