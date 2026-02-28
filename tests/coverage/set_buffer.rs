use super::*;

/// Set buffer with data.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_basic() {
    let tmux = TmuxServer::new("setb_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "hello"]);
    let out = tmux.run(&["showb"]);
    assert_eq!(out.trim(), "hello");
}

/// Set named buffer with -b.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_named() {
    let tmux = TmuxServer::new("setb_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "-b", "mybuf", "data"]);
    let out = tmux.run(&["showb", "-b", "mybuf"]);
    assert_eq!(out.trim(), "data");
}

/// Append to buffer with -a.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_append() {
    let tmux = TmuxServer::new("setb_append");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "-b", "mybuf", "hello"]);
    tmux.run(&["setb", "-a", "-b", "mybuf", " world"]);
    let out = tmux.run(&["showb", "-b", "mybuf"]);
    assert_eq!(out.trim(), "hello world");
}

/// Delete buffer (deleteb).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn delete_buffer() {
    let tmux = TmuxServer::new("deleteb_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "-b", "delbuf", "data"]);
    tmux.run(&["deleteb", "-b", "delbuf"]);

    let result = tmux.try_run(&["showb", "-b", "delbuf"]);
    assert!(!result.status.success());
}

/// Rename buffer with -n.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_rename() {
    let tmux = TmuxServer::new("setb_rename");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "-b", "oldbuf", "data"]);
    tmux.run(&["setb", "-b", "oldbuf", "-n", "newbuf"]);
    let out = tmux.run(&["showb", "-b", "newbuf"]);
    assert_eq!(out.trim(), "data");
}

/// Delete nonexistent buffer (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn delete_buffer_nonexistent() {
    let tmux = TmuxServer::new("deleteb_noexist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["deleteb", "-b", "nosuchbuf"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("unknown buffer"),
        "expected 'unknown buffer' error, got: {stderr}"
    );
}

/// Delete top buffer (without -b flag).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn delete_buffer_top() {
    let tmux = TmuxServer::new("deleteb_top");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Use setb without -b to create automatic buffers (paste_get_top only
    // returns automatic buffers)
    tmux.run(&["setb", "data1"]);
    tmux.run(&["setb", "data2"]);

    // Delete without -b removes the top (most recent automatic) buffer
    tmux.run(&["deleteb"]);
    let out = tmux.run(&["lsb"]);
    let lines: Vec<&str> = out.trim().lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "expected 1 buffer remaining, got: {out}");
}

/// Delete when no buffers exist at all.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn delete_buffer_empty() {
    let tmux = TmuxServer::new("deleteb_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["deleteb"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no buffer"),
        "expected 'no buffer' error, got: {stderr}"
    );
}

/// Rename buffer without -b (renames top buffer).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_rename_top() {
    let tmux = TmuxServer::new("setb_rename_top");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "topdata"]);
    // Rename top buffer (no -b) to a new name
    tmux.run(&["setb", "-n", "renamed"]);
    let out = tmux.run(&["showb", "-b", "renamed"]);
    assert_eq!(out.trim(), "topdata");
}

/// Rename nonexistent named buffer (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_rename_nonexistent() {
    let tmux = TmuxServer::new("setb_rename_noex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["setb", "-b", "nosuch", "-n", "newname"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("unknown buffer"),
        "expected 'unknown buffer' error, got: {stderr}"
    );
}

/// Rename when no buffers exist (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_rename_empty() {
    let tmux = TmuxServer::new("setb_rename_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["setb", "-n", "newname"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no buffer"),
        "expected 'no buffer' error, got: {stderr}"
    );
}

/// Set buffer without data argument (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_no_data() {
    let tmux = TmuxServer::new("setb_nodata");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["setb"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no data"),
        "expected 'no data' error, got: {stderr}"
    );
}

/// Set buffer with empty string data (no-op).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_empty_data() {
    let tmux = TmuxServer::new("setb_emptydata");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Setting an empty string should be a no-op (returns normally)
    tmux.run(&["setb", ""]);
    // No buffer should be created
    let out = tmux.run(&["lsb"]);
    assert!(
        out.trim().is_empty(),
        "expected no buffers after empty setb, got: {out}"
    );
}

/// Set buffer without -b (auto-named buffer).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_auto_name() {
    let tmux = TmuxServer::new("setb_autoname");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setb", "auto1"]);
    tmux.run(&["setb", "auto2"]);
    let out = tmux.run(&["lsb"]);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert!(lines.len() >= 2, "expected at least 2 buffers, got: {out}");
}

/// Append with -a without -b creates a new buffer (no existing pb to append to).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_append_no_b_flag() {
    let tmux = TmuxServer::new("setb_append_nob");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // First set a buffer
    tmux.run(&["setb", "start"]);
    // Append with -a but without -b: pb is null so no append happens,
    // just creates a new auto buffer with the new data
    tmux.run(&["setb", "-a", "end"]);
    let out = tmux.run(&["lsb"]);
    let lines: Vec<&str> = out.trim().lines().filter(|l| !l.is_empty()).collect();
    assert!(
        lines.len() >= 2,
        "expected at least 2 buffers (original + new), got: {out}"
    );
}

/// Append with -a when no buffer exists (creates new buffer).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn set_buffer_append_no_existing() {
    let tmux = TmuxServer::new("setb_append_noex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Append when no buffer exists - should just set the data
    tmux.run(&["setb", "-a", "-b", "newbuf", "appended"]);
    let out = tmux.run(&["showb", "-b", "newbuf"]);
    assert_eq!(out.trim(), "appended");
}
