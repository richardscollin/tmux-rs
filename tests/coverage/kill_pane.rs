use super::*;

/// Test kill-pane: default (kill target pane), and -a (kill all except target).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn kill_pane() {
    let tmux = TmuxServer::new("kill_pane");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create 3 panes total (split twice)
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-d"]);

    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "3", "should start with 3 panes");

    // kill-pane (no -a): kill pane 2, leaving 2 panes
    tmux.run(&["killp", "-t", "2"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "2", "should have 2 panes after killing one");

    // kill-pane -a: kill all panes except target (pane 0), leaving 1
    tmux.run(&["killp", "-a", "-t", "0"]);
    let count = tmux.display("#{window_panes}");
    assert_eq!(count, "1", "should have 1 pane after kill-pane -a");
}
