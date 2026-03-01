use super::*;

/// Test list-windows -a without -F (coverage: cmd_list_windows.rs lines 80-82)
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_windows_all_sessions() {
    let tmux = TmuxServer::new("list_windows_all");

    let conf = tmux.write_temp("new -sfirst\nneww\nnew -ssecond\n");
    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);

    // -a lists windows from all sessions (uses LIST_WINDOWS_WITH_SESSION_TEMPLATE)
    let output = tmux.run(&["lsw", "-a"]);
    let lines: Vec<&str> = output.lines().collect();
    // first has 2 windows, second has 1 window = 3 total
    assert_eq!(lines.len(), 3, "should have 3 windows across all sessions");
    // Each line should contain a session name prefix (the WITH_SESSION template)
    assert!(
        lines.iter().any(|l| l.starts_with("first:")),
        "should have windows from 'first' session"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("second:")),
        "should have windows from 'second' session"
    );
}

/// Test list-windows with -f filter (coverage: cmd_list_windows.rs lines 100-110)
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_windows_filter() {
    let tmux = TmuxServer::new("list_windows_filter");

    let conf = tmux.write_temp("new -stest\nneww -nfoo\nneww -nbar\n");
    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);

    // Filter to only windows named "foo"
    let output = tmux.run(&[
        "lsw",
        "-t",
        "test",
        "-f",
        "#{==:#{window_name},foo}",
        "-F",
        "#{window_name}",
    ]);
    assert_eq!(output.trim(), "foo");
}
