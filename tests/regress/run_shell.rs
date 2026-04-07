use super::*;

/// Ported from run-shell-output.sh
///
/// run-shell should go to stdout if present without -t.
/// With -t, output goes to the pane instead and pane enters view-mode.
#[test]
fn run_shell_output() {
    let tmux = TmuxServer::new("run_shell_output");
    let binary = TmuxServer::binary_path();
    let tmp = tmux.write_temp("");

    // Test 1: `run 'echo foo'` without -t goes to stdout.
    // The pane command runs: tmux run 'echo foo' > $TMP; sleep 10
    let pane_cmd = format!(
        "{} -L{} run 'echo foo' >{}; sleep 10",
        binary,
        tmux.socket(),
        tmp.path_str()
    );
    tmux.run(&["-f/dev/null", "new", "-d", &pane_cmd]);
    sleep_secs(1);
    let content = tmp.read_to_string();
    assert_eq!(
        content.trim(),
        "foo",
        "run without -t should write to stdout"
    );

    // Test 2: `run -t: 'echo foo'` goes to the pane, not stdout.
    let tmp2 = tmux.write_temp("");
    let pane_cmd2 = format!(
        "{} -L{} run -t: 'echo foo' >{}; sleep 10",
        binary,
        tmux.socket(),
        tmp2.path_str()
    );
    tmux.run(&["-f/dev/null", "new", "-d", &pane_cmd2]);
    sleep_secs(1);
    let content2 = tmp2.read_to_string();
    assert_eq!(content2, "", "run -t: should not write to stdout");
    let pane_mode = tmux.display("#{pane_mode}");
    assert_eq!(pane_mode, "view-mode", "pane should be in view-mode");

    tmux.kill_server();
}
