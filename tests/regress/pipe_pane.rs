use super::*;

/// Pipe pane to a command.
#[test]
fn pipe_pane_basic() {
    let tmux = TmuxServer::new("pipep_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("");

    // Pipe pane output to a file
    tmux.run(&["pipep", &format!("cat > {}", tmp.path_str())]);

    // Generate some output
    tmux.run(&["send-keys", "echo hello", "Enter"]);
    sleep_ms(500);

    // Close the pipe
    tmux.run(&["pipep"]);
    sleep_ms(200);

    let content = tmp.read_to_string();
    assert!(!content.is_empty(), "piped output should not be empty");
}

/// Pipe pane with -o (toggle/open only if not already piping).
#[test]
fn pipe_pane_toggle() {
    let tmux = TmuxServer::new("pipep_toggle");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("");

    // -o: open pipe only if not already piping
    tmux.run(&["pipep", "-o", &format!("cat > {}", tmp.path_str())]);

    // -o again: should close the pipe (toggle)
    tmux.run(&["pipep", "-o", &format!("cat > {}", tmp.path_str())]);
}

/// Pipe pane with -I (input mode).
#[test]
fn pipe_pane_input() {
    let tmux = TmuxServer::new("pipep_input");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Pipe input from a command
    tmux.run(&["pipep", "-I", "echo injected"]);
    sleep_ms(500);

    // Close pipe
    tmux.run(&["pipep"]);
}

/// Pipe pane with explicit -O (output mode).
#[test]
fn pipe_pane_output_flag() {
    let tmux = TmuxServer::new("pipep_output");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("");

    // Explicitly use -O for output mode
    tmux.run(&["pipep", "-O", &format!("cat > {}", tmp.path_str())]);

    // Generate some output
    tmux.run(&["send-keys", "echo testoutput", "Enter"]);
    sleep_ms(500);

    // Close the pipe
    tmux.run(&["pipep"]);
    sleep_ms(200);

    let content = tmp.read_to_string();
    assert!(
        !content.is_empty(),
        "piped output with -O should not be empty"
    );
}

/// Pipe pane with -I and -O combined (bidirectional).
#[test]
fn pipe_pane_input_output() {
    let tmux = TmuxServer::new("pipep_io");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Use both -I and -O for bidirectional pipe
    tmux.run(&["pipep", "-IO", "cat"]);
    sleep_ms(500);

    // Close the pipe
    tmux.run(&["pipep"]);
}

/// Pipe to a dead/exited pane returns error.
#[test]
fn pipe_pane_dead_pane() {
    let tmux = TmuxServer::new("pipep_dead");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Split and kill the new pane so it is dead but still exists
    tmux.run(&["splitw", "-d"]);
    sleep_ms(200);

    // Kill the process in pane %1 but keep the pane as dead (remain-on-exit)
    tmux.run(&["set", "-p", "-t", "%1", "remain-on-exit", "on"]);
    tmux.run(&["send-keys", "-t", "%1", "exit", "Enter"]);
    sleep_ms(500);

    // Try to pipe a dead pane -- should fail
    let result = tmux.try_run(&["pipep", "-t", "%1", "cat"]);
    assert!(
        !result.status.success(),
        "pipe-pane on dead pane should fail"
    );
}

/// Close pipe by running pipep with no arguments.
#[test]
fn pipe_pane_close_no_args() {
    let tmux = TmuxServer::new("pipep_close");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("");

    // Open a pipe
    tmux.run(&["pipep", &format!("cat > {}", tmp.path_str())]);
    sleep_ms(200);

    // Close the pipe with no arguments
    tmux.run(&["pipep"]);
    sleep_ms(200);

    // Verify pipe is closed: send output, it should not appear in file
    let before = tmp.read_to_string();
    tmux.run(&["send-keys", "echo afterclose", "Enter"]);
    sleep_ms(500);
    let after = tmp.read_to_string();
    // After closing pipe, no new output should be captured
    assert_eq!(before, after, "no new output after pipe is closed");
}

/// Close pipe by running pipep with empty command string.
#[test]
fn pipe_pane_close_empty_cmd() {
    let tmux = TmuxServer::new("pipep_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("");

    // Open a pipe
    tmux.run(&["pipep", &format!("cat > {}", tmp.path_str())]);
    sleep_ms(200);

    // Close with empty string argument
    tmux.run(&["pipep", ""]);
    sleep_ms(200);
}

/// Toggle -o: first call opens, second call with -o closes (no new pipe).
#[test]
fn pipe_pane_toggle_no_reopen() {
    let tmux = TmuxServer::new("pipep_toggle2");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp1 = tmux.write_temp("");
    let tmp2 = tmux.write_temp("");

    // Open pipe
    tmux.run(&["pipep", &format!("cat > {}", tmp1.path_str())]);
    tmux.run(&["send-keys", "echo first", "Enter"]);
    sleep_ms(500);

    // -o with an existing pipe: closes old pipe but does NOT open new one
    tmux.run(&["pipep", "-o", &format!("cat > {}", tmp2.path_str())]);
    sleep_ms(200);

    // Verify first pipe got data
    let content1 = tmp1.read_to_string();
    assert!(!content1.is_empty(), "first pipe should have data");

    // Second file should be empty since -o prevented reopening
    let content2 = tmp2.read_to_string();
    assert!(
        content2.is_empty(),
        "-o should not reopen pipe when one was active"
    );
}

/// Use pipe-pane alias 'pipep'.
#[test]
fn pipe_pane_alias() {
    let tmux = TmuxServer::new("pipep_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("");

    // Use alias
    tmux.run(&["pipep", &format!("cat > {}", tmp.path_str())]);
    tmux.run(&["send-keys", "echo aliaswork", "Enter"]);
    sleep_ms(500);
    tmux.run(&["pipep"]);
    sleep_ms(200);

    let content = tmp.read_to_string();
    assert!(!content.is_empty(), "alias pipep should work");
}
