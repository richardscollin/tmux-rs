use super::*;

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
    tmux.run(&["split-window", "-d", "-h"]);  // left|right
    tmux.run(&["split-window", "-d", "-v", "-t", ":.0"]);  // top-left, bottom-left
    tmux.run(&["split-window", "-d", "-v", "-t", ":.2"]);  // top-right, bottom-right

    // Start at pane 0 (top-left)
    tmux.run(&["select-pane", "-t", ":.0"]);

    // -R: move right
    tmux.run(&["select-pane", "-R"]);
    let pane = tmux.display("#{pane_index}");
    // Should be in a right pane (2 or 3)
    assert!(pane == "2" || pane == "3", "expected right pane, got: {pane}");

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
fn select_pane_last() {
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
fn select_pane_title() {
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
fn select_pane_mark() {
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
fn last_pane_enable_disable() {
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
