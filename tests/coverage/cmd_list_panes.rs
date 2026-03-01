use super::*;

/// Test list-panes default (window-level, type=0 template),
/// list-panes -s (session-level, type=1), list-panes -a (server-level, type=2),
/// custom -F format, -f filter, and multiple panes.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_panes_all_modes() {
    let tmux = TmuxServer::new("list_panes_all");

    // Create two sessions: "first" with 2 panes, "second" with 1 pane and 2 windows
    let conf =
        tmux.write_temp("new -sfirst -x80 -y24\nsplit-window\nnew -ssecond -x80 -y24\nneww\n");
    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);

    // -- Default (window-level, type=0): list panes of "first" which has 2 panes
    let output = tmux.run(&["lsp", "-tfirst"]);
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2, "first session window should have 2 panes");
    // type=0 template starts with "N: [WxH]"
    assert!(
        lines[0].starts_with("0:"),
        "type=0 should start with pane_index"
    );
    assert!(lines[1].starts_with("1:"), "type=0 second pane");

    // -- Session-level (-s, type=1): list panes of "second" which has 2 windows, 1 pane each
    let output = tmux.run(&["lsp", "-s", "-tsecond"]);
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "second session should have 2 windows with 1 pane each"
    );
    // type=1 template starts with "window_index.pane_index:"
    assert!(
        lines[0].contains('.'),
        "type=1 should contain window.pane format"
    );

    // -- Server-level (-a, type=2): list all panes across all sessions
    let output = tmux.run(&["lsp", "-a"]);
    let lines: Vec<&str> = output.lines().collect();
    // first: 2 panes, second: 2 panes (2 windows * 1 pane) = 4 total
    assert_eq!(lines.len(), 4, "should have 4 panes across all sessions");
    // type=2 template starts with "session_name:window_index.pane_index:"
    assert!(
        lines.iter().any(|l| l.starts_with("first:")),
        "should have panes from 'first'"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("second:")),
        "should have panes from 'second'"
    );

    // -- Custom format (-F)
    let output = tmux.run(&["lsp", "-tfirst", "-F", "#{pane_index}-#{pane_id}"]);
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("0-"), "custom format should work");

    // -- Filter (-f) matching
    let output = tmux.run(&[
        "lsp",
        "-tfirst",
        "-f",
        "#{==:#{pane_index},0}",
        "-F",
        "#{pane_index}",
    ]);
    assert_eq!(output.trim(), "0", "filter should only show pane 0");

    // -- Filter (-f) non-matching: filter that matches nothing
    let output = tmux.run(&[
        "lsp",
        "-tfirst",
        "-f",
        "#{==:#{pane_index},99}",
        "-F",
        "#{pane_index}",
    ]);
    assert_eq!(
        output.trim(),
        "",
        "filter matching nothing should produce empty output"
    );
}
