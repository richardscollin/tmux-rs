use super::*;

/// Test select-layout, next-layout, previous-layout.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_named() {
    let tmux = TmuxServer::new("selectl_named");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Select a named layout
    tmux.run(&["select-layout", "even-vertical"]);
    let layout = tmux.display("#{window_layout}");
    assert!(!layout.is_empty());

    tmux.run(&["select-layout", "even-horizontal"]);
    let layout2 = tmux.display("#{window_layout}");
    assert!(!layout2.is_empty());

    tmux.run(&["select-layout", "main-vertical"]);
    tmux.run(&["select-layout", "main-horizontal"]);
    tmux.run(&["select-layout", "tiled"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_next() {
    let tmux = TmuxServer::new("selectl_next");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    let layout_before = tmux.display("#{window_layout}");

    // next-layout
    tmux.run(&["next-layout"]);
    let layout_after = tmux.display("#{window_layout}");
    // Layout should change
    assert_ne!(
        layout_before, layout_after,
        "next-layout should change the layout"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_previous() {
    let tmux = TmuxServer::new("selectl_prev");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // previous-layout
    tmux.run(&["previous-layout"]);
    let layout = tmux.display("#{window_layout}");
    assert!(!layout.is_empty());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_next_flag() {
    let tmux = TmuxServer::new("selectl_nflag");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // select-layout -n: next
    tmux.run(&["select-layout", "-n"]);

    // select-layout -p: previous
    tmux.run(&["select-layout", "-p"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_spread() {
    let tmux = TmuxServer::new("selectl_spread");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);
    tmux.run(&["split-window", "-d"]);

    // select-layout -E: spread panes evenly
    tmux.run(&["select-layout", "-E"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_old() {
    let tmux = TmuxServer::new("selectl_old");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    let layout1 = tmux.display("#{window_layout}");
    tmux.run(&["select-layout", "even-vertical"]);
    let layout2 = tmux.display("#{window_layout}");

    // select-layout -o: revert to old layout
    tmux.run(&["select-layout", "-o"]);
    let layout3 = tmux.display("#{window_layout}");
    assert_eq!(layout3, layout1, "should revert to old layout with -o");
    let _ = layout2;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_custom_string() {
    let tmux = TmuxServer::new("selectl_custom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Get current layout string, then re-apply it
    let layout = tmux.display("#{window_layout}");
    tmux.run(&["select-layout", &layout]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_invalid() {
    let tmux = TmuxServer::new("selectl_invalid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Invalid layout string should error
    let out = tmux.try_run(&["select-layout", "not-a-valid-layout-string"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn select_layout_no_arg_no_flag() {
    let tmux = TmuxServer::new("selectl_noarg");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // select-layout with no args and no -n/-p/-E/-o: uses lastlayout or no-op
    tmux.run(&["select-layout"]);
}
