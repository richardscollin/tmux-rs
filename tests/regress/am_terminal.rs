use super::*;

/// Test am (auto margin) terminal override (translates am-terminal.sh)
#[test]
fn am_terminal() {
    let binary = TmuxServer::binary_path();

    // First server (tmux) uses the default test socket
    let tmux = TmuxServer::new("am_terminal");
    // Second server (tmux2) uses a different socket
    let tmux2 = TmuxServer::new("am_terminal2");

    // Set up tmux2 with a detached session and custom options
    tmux2.run(&["-f/dev/null", "new", "-d"]);
    tmux2.run(&["set", "-as", "terminal-overrides", ",*:am@"]);
    tmux2.run(&["set", "-g", "status-right", "RRR"]);
    tmux2.run(&["set", "-g", "status-left", "LLL"]);
    tmux2.run(&["set", "-g", "window-status-current-format", "WWW"]);

    // Create a 20x2 terminal in tmux that runs "tmux2 attach" inside it
    let attach_cmd = format!("{} -L{} attach", binary, tmux2.socket());
    tmux.run(&["-f/dev/null", "new", "-x20", "-y2", "-d", &attach_cmd]);
    sleep_secs(1);

    // Capture the pane and get the last non-empty line
    let output = tmux.run(&["capturep", "-p"]);
    let last_line = output.lines().rev().find(|l| !l.is_empty()).unwrap_or("");

    assert_eq!(last_line, "LLLWWW           RR");
}
