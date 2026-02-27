use super::*;

/// Break pane from a multi-pane window (the main code path: else branch at line 87).
/// Also tests -d (don't select new window) and default window naming.
#[test]
fn break_pane_multi_pane() {
    let tmux = TmuxServer::new("break_pane_multi");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create a second pane
    tmux.run(&["splitw", "-d"]);

    // Get pane IDs
    let panes_before = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    let ids: Vec<&str> = panes_before.lines().collect();
    assert_eq!(ids.len(), 2);

    // Break the second pane into a new window (with -d to stay on current)
    tmux.run(&["breakp", "-d", "-t:", "-s", &format!("{}", ids[1])]);

    // Should now have 2 windows, each with 1 pane
    let win_count = tmux.display("#{session_windows}");
    assert_eq!(win_count, "2");

    let panes_w0 = tmux.run(&["lsp", "-t", ":0", "-F", "#{pane_id}"]);
    assert_eq!(panes_w0.trim(), ids[0]);

    let panes_w1 = tmux.run(&["lsp", "-t", ":1", "-F", "#{pane_id}"]);
    assert_eq!(panes_w1.trim(), ids[1]);

    // -d: current window should still be 0
    let cur = tmux.display("#{window_index}");
    assert_eq!(cur, "0");
}

/// Break pane from a single-pane window into a different session.
/// Exercises the window_count_panes == 1 path (line 66) and cross-session branches.
#[test]
fn break_pane_single_pane_cross_session() {
    let tmux = TmuxServer::new("break_pane_single_cross");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "src"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new", "-d", "-x80", "-y24", "-s", "dst"]);

    // src has 1 window with 1 pane; break it into dst
    #[expect(
        unused_variables,
        reason = "#{pane_id} is a tmux format string, not Rust interpolation"
    )]
    let pane_id = tmux.display("-t src #{pane_id}");
    tmux.run(&["breakp", "-s", "src:0.0", "-t", "dst:"]);

    // dst should now have 2 windows
    let dst_wins = tmux.run(&["lsw", "-t", "dst", "-F", "#{window_index}"]);
    assert!(dst_wins.lines().count() >= 2);

    // src should have lost that window (it will have been destroyed or have 0 windows)
    let src_result = tmux.try_run(&["lsw", "-t", "src"]);
    // The session may be destroyed entirely since it had only one window
    // Either no windows or session gone is acceptable
    let src_stdout = String::from_utf8_lossy(&src_result.stdout);
    let src_stderr = String::from_utf8_lossy(&src_result.stderr);
    assert!(
        src_stdout.is_empty() || src_stderr.contains("can't find") || !src_result.status.success(),
        "src session should have no windows or be gone"
    );
}

/// Break pane with -n to set a custom window name.
/// Exercises line 78 (single pane) and line 115-117 (multi pane).
#[test]
fn break_pane_with_name() {
    let tmux = TmuxServer::new("break_pane_name");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Multi-pane case: split, then break with -n
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["breakp", "-d", "-n", "my_custom_name", "-t:"]);

    let name = tmux.run(&["display", "-t", ":1", "-p", "#{window_name}"]);
    assert_eq!(name.trim(), "my_custom_name");

    // Verify automatic-rename is off
    let auto_rename = tmux.run(&["showw", "-t", ":1", "-v", "automatic-rename"]);
    assert_eq!(auto_rename.trim(), "off");
}

/// Break pane with -P to print the new pane location (default format).
/// Exercises lines 149-157.
#[test]
fn break_pane_print() {
    let tmux = TmuxServer::new("break_pane_print");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    // Break with -P (print), should output session:window.pane
    let output = tmux.run(&["breakp", "-d", "-P", "-t:"]);
    let trimmed = output.trim();
    // Default format is #{session_name}:#{window_index}.#{pane_index}
    // Should look like "0:1.0" or similar
    assert!(
        trimmed.contains(':') && trimmed.contains('.'),
        "expected session:window.pane format, got: {trimmed}"
    );
}

/// Break pane with -P -F to print with custom format.
/// Exercises the custom template path at line 150.
#[test]
fn break_pane_print_custom_format() {
    let tmux = TmuxServer::new("break_pane_print_fmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["splitw", "-d"]);

    let output = tmux.run(&["breakp", "-d", "-P", "-F", "#{window_index}", "-t:"]);
    let trimmed = output.trim();
    // Should just be the window index number
    assert_eq!(trimmed, "1");
}

/// Break pane with -a (after current window).
/// Exercises the winlink_shuffle_up path at line 54-62.
#[test]
fn break_pane_after() {
    let tmux = TmuxServer::new("break_pane_after");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create windows 0 and 1
    tmux.run(&["neww", "-d"]);
    // Split window 0 so we have a pane to break
    tmux.run(&["splitw", "-d", "-t", ":0"]);

    // Break pane from window 0 with -a (insert after current window)
    tmux.run(&["breakp", "-d", "-a", "-s", ":0.1"]);

    // Should have 3 windows now
    let wins = tmux.run(&["lsw", "-F", "#{window_index}"]);
    let indices: Vec<&str> = wins.lines().collect();
    assert_eq!(indices.len(), 3);
}

/// Break pane with -b (before current window).
/// Exercises the `before` variable path at line 53.
#[test]
fn break_pane_before() {
    let tmux = TmuxServer::new("break_pane_before");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create window 1
    tmux.run(&["neww", "-d"]);
    // Split window 1 to get a pane to break
    tmux.run(&["splitw", "-d", "-t", ":1"]);

    // Select window 1 then break with -b (insert before window 1)
    tmux.run(&["selectw", "-t", ":1"]);
    tmux.run(&["breakp", "-d", "-b", "-s", ":1.1"]);

    // Should have 3 windows
    let wins = tmux.run(&["lsw", "-F", "#{window_index}"]);
    assert_eq!(wins.lines().count(), 3);
}

/// Break pane to a specific target index that is already in use.
/// Exercises the "index in use" error at line 88-90.
#[test]
fn break_pane_index_in_use() {
    let tmux = TmuxServer::new("break_pane_idx_used");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Split to get multiple panes
    tmux.run(&["splitw", "-d"]);
    // Create window at index 1
    tmux.run(&["neww", "-d"]);

    // Try to break a pane to index 1 which already exists
    let result = tmux.try_run(&["breakp", "-s", ":0.1", "-t", ":1"]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("index in use") || !result.status.success(),
        "expected error about index in use, got stderr: {stderr}"
    );
}

/// Break pane without -d (should select the new window).
/// Exercises the session_select path at line 134-136.
#[test]
fn break_pane_selects_new_window() {
    let tmux = TmuxServer::new("break_pane_select");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["splitw", "-d"]);

    // Break without -d: should switch to the new window
    tmux.run(&["breakp", "-t:"]);

    let cur = tmux.display("#{window_index}");
    assert_eq!(cur, "1", "should have switched to the new window");
}

/// Break pane from multi-pane window into a different session.
/// Exercises src_s != dst_s branches at lines 140-145.
#[test]
fn break_pane_multi_pane_cross_session() {
    let tmux = TmuxServer::new("break_pane_multi_cross");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "src"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new", "-d", "-x80", "-y24", "-s", "dst"]);

    // Split src to have 2 panes
    tmux.run(&["splitw", "-d", "-t", "src"]);

    let pane_id = tmux.run(&["lsp", "-t", "src:0", "-F", "#{pane_id}"]);
    let ids: Vec<&str> = pane_id.lines().collect();
    assert_eq!(ids.len(), 2);

    // Break pane 1 from src into dst
    tmux.run(&[
        "breakp",
        "-d",
        "-s",
        &format!("src:0.{}", ids[1].trim()),
        "-t",
        "dst:",
    ]);

    // src should still have 1 pane in window 0
    let src_panes = tmux.run(&["lsp", "-t", "src:0", "-F", "#{pane_id}"]);
    assert_eq!(src_panes.lines().count(), 1);

    // dst should have 2 windows
    let dst_wins = tmux.run(&["lsw", "-t", "dst", "-F", "#{window_index}"]);
    assert_eq!(dst_wins.lines().count(), 2);
}

/// Break pane with -a but no explicit -t target.
/// Exercises line 58: winlink_shuffle_up with curw fallback.
#[test]
fn break_pane_after_no_target() {
    let tmux = TmuxServer::new("break_pane_after_notgt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Split to get 2 panes
    tmux.run(&["splitw", "-d"]);

    // Break with -a but no -t (uses current window for shuffle)
    tmux.run(&["breakp", "-d", "-a"]);

    // Should have 2 windows
    let wins = tmux.run(&["lsw", "-F", "#{window_index}"]);
    assert_eq!(wins.lines().count(), 2);
}

/// Break pane with -n from a single-pane window (the link path).
/// Exercises -n at line 78-81 in the single-pane branch.
#[test]
fn break_pane_single_pane_with_name() {
    let tmux = TmuxServer::new("break_pane_single_name");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "src"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new", "-d", "-x80", "-y24", "-s", "dst"]);

    // Break src's single pane into dst with a name
    tmux.run(&["breakp", "-n", "renamed", "-s", "src:0.0", "-t", "dst:"]);

    // Verify the window name in dst
    let names = tmux.run(&["lsw", "-t", "dst", "-F", "#{window_name}"]);
    assert!(
        names.lines().any(|l| l.trim() == "renamed"),
        "dst should have a window named 'renamed', got: {names}"
    );
}
