use super::*;

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_find_window_default_flags() {
    // No -C/-N/-T flags: defaults to all three (c && n && t branch)
    let tmux = TmuxServer::new("findw_default");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["find-window", "bash"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_find_window_flag_combinations() {
    let tmux = TmuxServer::new("findw_flags");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -C && -N (command + name)
    tmux.run_with_stdin(&["-C", "attach"], b"send-keys q\n");
    tmux.run(&["find-window", "-CN", "bash"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");

    // Exit tree-mode before next test
    tmux.run_with_stdin(&["-C", "attach"], b"send-keys q\n");

    // -C && -T (command + title)
    tmux.run(&["find-window", "-CT", "bash"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");

    tmux.run_with_stdin(&["-C", "attach"], b"send-keys q\n");

    // -N && -T (name + title)
    tmux.run(&["find-window", "-NT", "bash"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");

    tmux.run_with_stdin(&["-C", "attach"], b"send-keys q\n");

    // -C only (command only)
    tmux.run(&["find-window", "-C", "bash"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");

    tmux.run_with_stdin(&["-C", "attach"], b"send-keys q\n");

    // -N only (name only)
    tmux.run(&["find-window", "-N", "bash"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");

    tmux.run_with_stdin(&["-C", "attach"], b"send-keys q\n");

    // -T only (title only)
    tmux.run(&["find-window", "-T", "bash"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_find_window_regex_and_case() {
    let tmux = TmuxServer::new("findw_regex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -r (regex mode, no -i)
    tmux.run(&["find-window", "-r", "ba.*sh"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");

    tmux.run_with_stdin(&["-C", "attach"], b"send-keys q\n");

    // -r -i (regex + case-insensitive)
    tmux.run(&["find-window", "-ri", "BA.*SH"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");

    tmux.run_with_stdin(&["-C", "attach"], b"send-keys q\n");

    // -i only (case-insensitive, no regex)
    tmux.run(&["find-window", "-i", "BASH"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_find_window_zoom_flag() {
    let tmux = TmuxServer::new("findw_zoom");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -Z flag (zoom)
    tmux.run(&["find-window", "-Z", "bash"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_find_window_all_flags_and_regex() {
    // Exercise -C -N -T with -r and -Z together
    let tmux = TmuxServer::new("findw_all");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["find-window", "-CNTrZ", "ba.*sh"]);
    assert_eq!(tmux.display("#{pane_mode}"), "tree-mode");
}
