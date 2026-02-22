use super::*;

/// Test control client sanity (translates control-client-sanity.sh)
#[test]
fn control_client_sanity() {
    let tmux = TmuxServer::new("control_client_sanity");

    // Create a detached session with 200x200
    tmux.run(&["-f/dev/null", "new", "-d", "-x200", "-y200"]);
    tmux.run(&["-f/dev/null", "splitw"]);
    sleep_secs(1);

    // Send control client commands via stdin
    let commands = b"\
refresh-client -C 200x200\n\
selectp -t%0\n\
splitw\n\
neww\n\
splitw\n\
selectp -t%0\n\
killp -t%1\n\
swapp -t%2 -s%3\n\
neww\n\
splitw\n\
splitw\n\
selectl tiled\n\
killw\n";

    tmux.run_with_stdin(&["-C", "a"], commands);
    sleep_secs(1);

    // Verify server is still running
    let output = tmux.try_run(&["has"]);
    assert!(output.status.success(), "server should still be running");

    // Verify pane layout
    let layout_output = tmux.run(&["lsp", "-aF", "#{pane_id} #{window_layout}"]);
    let lines: Vec<&str> = layout_output.lines().collect();

    assert_eq!(lines.len(), 4, "expected 4 pane lines, got: {:?}", lines);

    // Check pane IDs are as expected
    let pane_ids: Vec<&str> = lines.iter().map(|l| l.split(' ').next().unwrap()).collect();
    assert_eq!(pane_ids, vec!["%0", "%3", "%2", "%4"]);

    // Check that paired panes share the same layout checksum
    let layouts: Vec<&str> = lines
        .iter()
        .map(|l| l.splitn(2, ' ').nth(1).unwrap())
        .collect();
    assert_eq!(
        layouts[0], layouts[1],
        "panes %0 and %3 should share layout"
    );
    assert_eq!(
        layouts[2], layouts[3],
        "panes %2 and %4 should share layout"
    );
}

/// Test control client size handling (translates control-client-size.sh)
#[test]
fn control_client_size() {
    // Test 1: default size -> refresh to 100x50
    {
        let tmux = TmuxServer::new("control_client_size_1");

        tmux.run(&["-f/dev/null", "new", "-d"]);
        sleep_secs(1);

        let commands = b"\
ls -F':#{window_width} #{window_height}'\n\
refresh -C 100,50\n";

        let output = tmux.run_with_stdin(&["-C", "a"], commands);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Filter lines starting with ':'
        let mut result = String::new();
        for line in stdout.lines() {
            if line.starts_with(':') {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Append current ls output
        let ls_output = tmux.run(&["ls", "-F", ":#{window_width} #{window_height}"]);
        result.push_str(&ls_output);

        let result_lines: Vec<&str> = result.lines().collect();
        assert_eq!(
            result_lines,
            vec![":80 24", ":100 50"],
            "test 1: default -> refresh 100x50"
        );
    }

    // Test 2: no refresh, stays 80x24
    {
        let tmux = TmuxServer::new("control_client_size_2");

        tmux.run(&["-f/dev/null", "new", "-d"]);
        sleep_secs(1);

        let commands = b"\
ls -F':#{window_width} #{window_height}'\n\
refresh -C 80,24\n";

        let output = tmux.run_with_stdin(&["-f/dev/null", "-C", "a"], commands);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Filter lines starting with ':' from control output
        let mut result = String::new();
        for line in stdout.lines() {
            if line.starts_with(':') {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Append current ls output
        let ls_output = tmux.run(&["ls", "-F", ":#{window_width} #{window_height}"]);
        result.push_str(&ls_output);

        let result_lines: Vec<&str> = result.lines().collect();
        assert_eq!(
            result_lines,
            vec![":80 24", ":80 24"],
            "test 2: should stay 80x24"
        );
    }

    // Test 3: new with -x100 -y50, then refresh to 80x24
    {
        let tmux = TmuxServer::new("control_client_size_3");

        let commands = b"\
ls -F':#{window_width} #{window_height}'\n\
refresh -C 80,24\n";

        let output = tmux.run_with_stdin(
            &["-f/dev/null", "-C", "new", "-x", "100", "-y", "50"],
            commands,
        );
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Filter lines starting with ':' from control output
        let mut result = String::new();
        for line in stdout.lines() {
            if line.starts_with(':') {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Append current ls output
        let ls_output = tmux.run(&["ls", "-F", ":#{window_width} #{window_height}"]);
        result.push_str(&ls_output);

        let result_lines: Vec<&str> = result.lines().collect();
        assert_eq!(
            result_lines,
            vec![":100 50", ":80 24"],
            "test 3: 100x50 -> refresh 80x24"
        );
    }
}
