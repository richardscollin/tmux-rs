use super::*;

/// last-pane switches to the previously active pane.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn last_pane_basic() {
    let tmux = TmuxServer::new("lastp_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Split to create pane 1, which becomes active
    tmux.run(&["split-window"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "1");

    // last-pane should go back to pane 0
    tmux.run(&["last-pane"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "0");

    // last-pane again should go back to pane 1
    tmux.run(&["last-pane"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "1");
}

/// last-pane with only one pane: "no last pane" error.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn last_pane_no_last() {
    let tmux = TmuxServer::new("lastp_nolast");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["last-pane"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no last pane"),
        "expected 'no last pane' error, got: {stderr}"
    );
}

/// last-pane with 2 panes but no last_panes list (split-window -d).
/// Exercises the fallback to prev/next of active pane.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn last_pane_split_detached() {
    let tmux = TmuxServer::new("lastp_splitd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -d means pane 1 is created but never visited, so last_panes is empty
    tmux.run(&["split-window", "-d"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "0");

    // last-pane should find the other pane via prev/next fallback
    tmux.run(&["last-pane"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "1");
}

/// last-pane -e (enable input on last pane) and -d (disable input).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn last_pane_enable_disable() {
    let tmux = TmuxServer::new("lastp_ed");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window"]);

    // -d disables input on the last pane (pane 0)
    tmux.run(&["last-pane", "-d"]);
    // Should still be on pane 1 (not switching)
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "1");

    // -e re-enables input on the last pane
    tmux.run(&["last-pane", "-e"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "1");
}

/// last-pane -Z (zoom handling).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn last_pane_zoom() {
    let tmux = TmuxServer::new("lastp_zoom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window"]);

    // Zoom pane 1
    tmux.run(&["resize-pane", "-Z"]);
    let zoomed = tmux.display("#{window_zoomed_flag}");
    assert_eq!(zoomed.trim(), "1");

    // last-pane -Z should unzoom and switch
    tmux.run(&["last-pane", "-Z"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "0");
}

/// select-pane -l (alias for last-pane behavior).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_last() {
    let tmux = TmuxServer::new("selp_last");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window"]);

    tmux.run(&["select-pane", "-l"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "0");
}

/// select-pane -m (mark) and -M (unmark).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_mark() {
    let tmux = TmuxServer::new("selp_mark");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Mark pane 0
    tmux.run(&["select-pane", "-m", "-t", ":.0"]);
    let marked = tmux.display("#{pane_marked}");
    assert_eq!(marked.trim(), "1");

    // Mark the same pane again toggles it off
    tmux.run(&["select-pane", "-m", "-t", ":.0"]);
    let marked = tmux.display("#{pane_marked}");
    assert_eq!(marked.trim(), "0");

    // Mark, then unmark with -M
    tmux.run(&["select-pane", "-m", "-t", ":.0"]);
    tmux.run(&["select-pane", "-M"]);
    let marked = tmux.display("#{pane_marked}");
    assert_eq!(marked.trim(), "0");
}

/// Mark a pane, then mark a different pane (clears old mark).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_mark_switch() {
    let tmux = TmuxServer::new("selp_mark_sw");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Mark pane 0
    tmux.run(&["select-pane", "-m", "-t", ":.0"]);
    // Mark pane 1 (should clear mark on pane 0, set on pane 1)
    tmux.run(&["select-pane", "-m", "-t", ":.1"]);
    let m0 = tmux.run(&["display", "-t", ":.0", "-p", "#{pane_marked}"]);
    let m1 = tmux.run(&["display", "-t", ":.1", "-p", "#{pane_marked}"]);
    assert_eq!(m0.trim(), "0");
    assert_eq!(m1.trim(), "1");
}

/// select-pane -P (set style) and -g (get style).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_style() {
    let tmux = TmuxServer::new("selp_style");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Set style with -P
    tmux.run(&["select-pane", "-P", "bg=red"]);

    // Get style with -g
    let out = tmux.run(&["select-pane", "-g"]);
    assert!(
        out.contains("bg=") || out.contains("red"),
        "expected style output, got: {out}"
    );
}

/// select-pane directional: -L (left), -R (right).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_directional_lr() {
    let tmux = TmuxServer::new("selp_dir_lr");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Horizontal split: pane 0 left, pane 1 right (active)
    tmux.run(&["split-window", "-h"]);
    assert_eq!(tmux.display("#{pane_index}").trim(), "1");

    // -L from right pane should go to left pane
    tmux.run(&["select-pane", "-L"]);
    assert_eq!(tmux.display("#{pane_index}").trim(), "0");

    // -R from left pane should go to right pane
    tmux.run(&["select-pane", "-R"]);
    assert_eq!(tmux.display("#{pane_index}").trim(), "1");
}

/// select-pane directional: -U (up), -D (down).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_directional_ud() {
    let tmux = TmuxServer::new("selp_dir_ud");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Vertical split: pane 0 top, pane 1 bottom (active)
    tmux.run(&["split-window", "-v"]);
    assert_eq!(tmux.display("#{pane_index}").trim(), "1");

    // -U from bottom pane should go to top pane
    tmux.run(&["select-pane", "-U"]);
    assert_eq!(tmux.display("#{pane_index}").trim(), "0");

    // -D from top pane should go to bottom pane
    tmux.run(&["select-pane", "-D"]);
    assert_eq!(tmux.display("#{pane_index}").trim(), "1");
}

/// select-pane -e (enable input) and -d (disable input).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_enable_disable() {
    let tmux = TmuxServer::new("selp_ed");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Disable input
    tmux.run(&["select-pane", "-d"]);
    // Enable input
    tmux.run(&["select-pane", "-e"]);
}

/// select-pane -T (set title).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_title() {
    let tmux = TmuxServer::new("selp_title");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["select-pane", "-T", "my custom title"]);
    let title = tmux.display("#{pane_title}");
    assert_eq!(title.trim(), "my custom title");
}

/// select-pane -t (select specific pane by index).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_target() {
    let tmux = TmuxServer::new("selp_target");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Should be on pane 0
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "0");

    // Select pane 1
    tmux.run(&["select-pane", "-t", ":.1"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "1");
}

/// select-pane with no flags on already-active pane (noop early return).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_noop() {
    let tmux = TmuxServer::new("selp_noop");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Select the already-active pane (noop path: wp == activewp)
    tmux.run(&["select-pane"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "0");
}

/// select-pane -Z (zoom toggle when switching panes).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_zoom() {
    let tmux = TmuxServer::new("selp_zoom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Zoom pane 0
    tmux.run(&["resize-pane", "-Z"]);
    let zoomed = tmux.display("#{window_zoomed_flag}");
    assert_eq!(zoomed.trim(), "1");

    // select-pane -Z -t :.1 should unzoom and switch
    tmux.run(&["select-pane", "-Z", "-t", ":.1"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "1");
}

/// selectp alias works.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_alias() {
    let tmux = TmuxServer::new("selp_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    tmux.run(&["selectp", "-t", ":.1"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "1");
}

/// lastp alias works.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn last_pane_alias() {
    let tmux = TmuxServer::new("lastp_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window"]);

    tmux.run(&["lastp"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "0");
}

/// select-pane with directional flag when no pane exists in that direction
/// (returns early with CMD_RETURN_NORMAL when wp is null).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_directional_null() {
    let tmux = TmuxServer::new("selp_dir_null");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Only one pane, -L/-R/-U/-D should all find null and return gracefully
    tmux.run(&["select-pane", "-L"]);
    tmux.run(&["select-pane", "-R"]);
    tmux.run(&["select-pane", "-U"]);
    tmux.run(&["select-pane", "-D"]);
    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "0");
}

/// select-pane via control-mode to exercise cmd_select_pane_redraw with
/// an attached client.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_control_mode() {
    let tmux = TmuxServer::new("selp_ctrl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Use control-mode client to run select-pane, which exercises
    // cmd_select_pane_redraw with a non-null client
    let output = tmux.run_with_stdin(&["-C", "attach"], b"select-pane -t :.1\ndetach-client\n");
    assert!(output.status.success());

    let active = tmux.display("#{pane_index}");
    assert_eq!(active.trim(), "1");
}

/// select-pane -m on an invisible pane (zoomed, other pane hidden).
/// Exercises the !window_pane_visible(wp) early return on line 129.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_mark_invisible() {
    let tmux = TmuxServer::new("selp_mark_invis");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Zoom pane 0, making pane 1 invisible
    tmux.run(&["resize-pane", "-Z"]);
    // Try to mark pane 1 (invisible) -- should be a noop
    tmux.run(&["select-pane", "-m", "-t", ":.1"]);
    let marked = tmux.run(&["display", "-t", ":.1", "-p", "#{pane_marked}"]);
    assert_eq!(marked.trim(), "0", "marking invisible pane should be noop");
}

/// select-pane -T with same title (screen_set_title returns 0, no notify).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_title_same() {
    let tmux = TmuxServer::new("selp_title_same");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["select-pane", "-T", "my title"]);
    // Set the same title again -- screen_set_title returns 0 (no change)
    tmux.run(&["select-pane", "-T", "my title"]);
    let title = tmux.display("#{pane_title}");
    assert_eq!(title.trim(), "my title");
}

/// Test select-pane: basic selection, directional (-U/-D/-L/-R),
/// last-pane, enable/disable input, set title, and mark.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_basic() {
    let tmux = TmuxServer::new("selectp_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d", "-h"]);

    // Select pane 1
    tmux.run(&["select-pane", "-t", ":.1"]);
    let pane = tmux.display("#{pane_index}");
    assert_eq!(pane, "1");

    // Select pane 0
    tmux.run(&["select-pane", "-t", ":.0"]);
    let pane = tmux.display("#{pane_index}");
    assert_eq!(pane, "0");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_directional() {
    let tmux = TmuxServer::new("selectp_dir");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create a 2x2 grid of panes
    tmux.run(&["split-window", "-d", "-h"]); // left|right
    tmux.run(&["split-window", "-d", "-v", "-t", ":.0"]); // top-left, bottom-left
    tmux.run(&["split-window", "-d", "-v", "-t", ":.2"]); // top-right, bottom-right

    // Start at pane 0 (top-left)
    tmux.run(&["select-pane", "-t", ":.0"]);

    // -R: move right
    tmux.run(&["select-pane", "-R"]);
    let pane = tmux.display("#{pane_index}");
    // Should be in a right pane (2 or 3)
    assert!(
        pane == "2" || pane == "3",
        "expected right pane, got: {pane}"
    );

    // -L: move left
    tmux.run(&["select-pane", "-L"]);

    // -D: move down
    tmux.run(&["select-pane", "-t", ":.0"]);
    tmux.run(&["select-pane", "-D"]);
    let pane = tmux.display("#{pane_index}");
    assert_eq!(pane, "1", "expected bottom pane after -D");

    // -U: move up
    tmux.run(&["select-pane", "-U"]);
    let pane = tmux.display("#{pane_index}");
    assert_eq!(pane, "0", "expected top pane after -U");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_last_02() {
    let tmux = TmuxServer::new("selectp_last");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Visit pane 1, then pane 0 to set "last"
    tmux.run(&["select-pane", "-t", ":.1"]);
    tmux.run(&["select-pane", "-t", ":.0"]);

    // last-pane
    tmux.run(&["last-pane"]);
    let pane = tmux.display("#{pane_index}");
    assert_eq!(pane, "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_last_flag() {
    let tmux = TmuxServer::new("selectp_lflag");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    tmux.run(&["select-pane", "-t", ":.1"]);
    tmux.run(&["select-pane", "-t", ":.0"]);

    // select-pane -l: same as last-pane
    tmux.run(&["select-pane", "-l"]);
    let pane = tmux.display("#{pane_index}");
    assert_eq!(pane, "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_no_last() {
    let tmux = TmuxServer::new("selectp_nolast");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Only one pane, no last — should error
    let out = tmux.try_run(&["last-pane"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no last pane"),
        "expected 'no last pane' error, got: {stderr}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_disable_enable() {
    let tmux = TmuxServer::new("selectp_de");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // -d: disable input on pane
    tmux.run(&["select-pane", "-d", "-t", ":.1"]);

    // -e: re-enable input on pane
    tmux.run(&["select-pane", "-e", "-t", ":.1"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_title_02() {
    let tmux = TmuxServer::new("selectp_title");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -T: set pane title
    tmux.run(&["select-pane", "-T", "my-title"]);
    let title = tmux.display("#{pane_title}");
    assert_eq!(title, "my-title");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_mark_02() {
    let tmux = TmuxServer::new("selectp_mark");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // -m: set mark on a pane
    tmux.run(&["select-pane", "-m", "-t", ":.0"]);

    // -M: clear mark
    tmux.run(&["select-pane", "-M"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_pane_same_pane() {
    let tmux = TmuxServer::new("selectp_same");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Selecting the already-active pane should be a no-op
    tmux.run(&["select-pane", "-t", ":.0"]);
    let pane = tmux.display("#{pane_index}");
    assert_eq!(pane, "0");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn last_pane_enable_disable_02() {
    let tmux = TmuxServer::new("lastp_de");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Visit both panes
    tmux.run(&["select-pane", "-t", ":.1"]);
    tmux.run(&["select-pane", "-t", ":.0"]);

    // last-pane -d: disable input on last pane
    tmux.run(&["last-pane", "-d"]);

    // last-pane -e: enable input on last pane
    tmux.run(&["last-pane", "-e"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn last_pane_two_panes_no_visit() {
    let tmux = TmuxServer::new("lastp_2pane");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    // split-window -d creates pane but doesn't visit it
    tmux.run(&["split-window", "-d"]);

    // With 2 panes and no last_panes list, should fall back to prev/next pane
    tmux.run(&["last-pane"]);
    let pane = tmux.display("#{pane_index}");
    assert_eq!(pane, "1", "should select the other pane");
}
