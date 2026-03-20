use super::*;

/// Test list-panes: default (window), -s (session), -a (all),
/// -F (format), -f (filter).

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_panes_default() {
    let tmux = TmuxServer::new("lsp_default");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    let out = tmux.run(&["list-panes"]);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines.len(), 2, "should list 2 panes, got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_panes_session() {
    let tmux = TmuxServer::new("lsp_session");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);
    tmux.run(&["split-window", "-d"]);

    // -s: list all panes in the session (across windows)
    let out = tmux.run(&["list-panes", "-s"]);
    let lines: Vec<&str> = out.trim().lines().collect();
    // Window 0 has 1 pane, window 1 has 2 panes = 3 total
    assert_eq!(lines.len(), 3, "should list 3 panes across session, got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_panes_all() {
    let tmux = TmuxServer::new("lsp_all");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "s1", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-session", "-d", "-s", "s2"]);

    // -a: list panes across all sessions
    let out = tmux.run(&["list-panes", "-a"]);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert!(lines.len() >= 2, "should list panes from all sessions, got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_panes_format() {
    let tmux = TmuxServer::new("lsp_fmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // -F: custom format
    let out = tmux.run(&["list-panes", "-F", "#{pane_index}"]);
    assert!(out.contains("0") && out.contains("1"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_panes_filter() {
    let tmux = TmuxServer::new("lsp_filter");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // -f: filter panes
    let out = tmux.run(&["list-panes", "-f", "#{==:#{pane_index},0}", "-F", "#{pane_index}"]);
    assert_eq!(out.trim(), "0", "filter should only show pane 0, got: {out}");
}
