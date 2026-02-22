use super::*;

/// Test capture-pane -e for OSC 8 hyperlink sequences (translates capture-pane-hyperlink.sh)
///
/// The shell script runs capturep INSIDE the pane command (after printf), so
/// we replicate that by having the pane write capture output to a temp file.
#[test]
fn capture_pane_hyperlink() {
    let tmux = TmuxServer::new("capture_pane_hyperlink");
    let binary = TmuxServer::binary_path();
    let socket = tmux.socket();

    // Test 1: hyperlink with id parameter
    {
        let out = tmux.write_temp("");
        let pane_cmd = format!(
            "printf '\\033]8;id=1;https://github.com\\033\\\\test1\\033]8;;\\033\\\\\\n'; \
             {} -L{} capturep -peS0 -E1 > {}",
            binary,
            socket,
            out.path_str()
        );
        tmux.run(&["-f/dev/null", "new", "-d", &pane_cmd]);
        sleep_secs(1);

        let captured = out.read_to_bytes();
        let expected = b"\x1b]8;id=1;https://github.com\x1b\\test1\x1b]8;;\x1b\\\n\n";
        assert_eq!(captured, expected, "hyperlink test 1 failed");
    }

    // Verify server exited (pane command finished, no remain-on-exit)
    let output = tmux.try_run(&["has"]);
    assert!(!output.status.success(), "server should have exited");

    // Test 2: hyperlink without id parameter
    {
        let out = tmux.write_temp("");
        let pane_cmd = format!(
            "printf '\\033]8;;https://github.com/tmux/tmux\\033\\\\test1\\033]8;;\\033\\\\\\n'; \
             {} -L{} capturep -peS0 -E1 > {}",
            binary,
            socket,
            out.path_str()
        );
        tmux.run(&["-f/dev/null", "new", "-d", &pane_cmd]);
        sleep_secs(1);

        let captured = out.read_to_bytes();
        let expected = b"\x1b]8;;https://github.com/tmux/tmux\x1b\\test1\x1b]8;;\x1b\\\n\n";
        assert_eq!(captured, expected, "hyperlink test 2 failed");
    }

    // Verify server exited
    let output = tmux.try_run(&["has"]);
    assert!(!output.status.success(), "server should have exited");
}

/// Test capture-pane sends colours after SGR 0 (translates capture-pane-sgr0.sh)
#[test]
fn capture_pane_sgr0() {
    let tmux = TmuxServer::new("capture_pane_sgr0");
    let binary = TmuxServer::binary_path();
    let socket = tmux.socket();

    let out = tmux.write_temp("");
    let pane_cmd = format!(
        "printf '\\033[31;42;1mabc\\033[0;31mdef\\n'; \
         printf '\\033[m\\033[100m bright bg \\033[m'; \
         {} -L{} capturep -peS0 -E1 >> {}",
        binary,
        socket,
        out.path_str()
    );
    tmux.run(&["-f/dev/null", "new", "-d", &pane_cmd]);
    sleep_secs(1);

    let captured = out.read_to_bytes();

    let mut expected = Vec::new();
    expected.extend_from_slice(b"\x1b[1m\x1b[31m\x1b[42mabc\x1b[0m\x1b[31mdef\x1b[39m\n");
    expected.extend_from_slice(b"\x1b[100m bright bg \x1b[49m\n");
    assert_eq!(captured, expected, "SGR 0 colour capture mismatch");
}
