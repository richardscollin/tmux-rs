use super::*;

/// Switch to next session with -n.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_next() {
    let tmux = TmuxServer::new("switchc_next");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sess1", "-x80", "-y24"]);
    tmux.run(&["new", "-d", "-s", "sess2"]);

    let cmd = "switch-client -n\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach", "-t", "sess1"], cmd.as_bytes());
    assert!(output.status.success());
}

/// Switch to previous session with -p.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_prev() {
    let tmux = TmuxServer::new("switchc_prev");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sess1", "-x80", "-y24"]);
    tmux.run(&["new", "-d", "-s", "sess2"]);

    let cmd = "switch-client -p\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach", "-t", "sess2"], cmd.as_bytes());
    assert!(output.status.success());
}

/// Switch to last session with -l.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_last() {
    let tmux = TmuxServer::new("switchc_last");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sess1", "-x80", "-y24"]);
    tmux.run(&["new", "-d", "-s", "sess2"]);

    // First switch to sess2, then switch back with -l
    let cmd = "switch-client -t sess2\nswitch-client -l\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach", "-t", "sess1"], cmd.as_bytes());
    assert!(output.status.success());
}

/// Switch with -r (toggle read-only).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_readonly() {
    let tmux = TmuxServer::new("switchc_ro");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sess1", "-x80", "-y24"]);

    // Toggle read-only on and off
    let cmd = "switch-client -r\nswitch-client -r\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());
}

/// Switch with -T (key table).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_key_table() {
    let tmux = TmuxServer::new("switchc_T");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Switch to the prefix key table
    let cmd = "switch-client -T prefix\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());
}

/// Switch with -T to nonexistent key table (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_bad_key_table() {
    let tmux = TmuxServer::new("switchc_badT");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let cmd = "switch-client -T nonexistent_table_xyz\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("doesn't exist"),
        "expected 'doesn't exist' error in control output, got: {stdout}"
    );
}

/// Switch -n with only one session (error: can't find next).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_no_next() {
    let tmux = TmuxServer::new("switchc_nonext");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "only", "-x80", "-y24"]);

    let cmd = "switch-client -n\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("can't find next session"),
        "expected 'can't find next session' error, got: {stdout}"
    );
}

/// Switch -p with only one session (error: can't find previous).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_no_prev() {
    let tmux = TmuxServer::new("switchc_noprev");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "only", "-x80", "-y24"]);

    let cmd = "switch-client -p\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("can't find previous session"),
        "expected 'can't find previous session' error, got: {stdout}"
    );
}

/// Switch -l without having switched before (error: can't find last).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_no_last() {
    let tmux = TmuxServer::new("switchc_nolast");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "only", "-x80", "-y24"]);

    let cmd = "switch-client -l\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("can't find last session"),
        "expected 'can't find last session' error, got: {stdout}"
    );
}

/// Switch to a specific target session with -t.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_target() {
    let tmux = TmuxServer::new("switchc_target");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sess1", "-x80", "-y24"]);
    tmux.run(&["new", "-d", "-s", "sess2"]);

    let cmd = "switch-client -t sess2\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach", "-t", "sess1"], cmd.as_bytes());
    assert!(output.status.success());
}

/// Switch to a target pane with -t (pane specifier triggers pane switching).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_target_pane() {
    let tmux = TmuxServer::new("switchc_pane");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["split-window", "-d"]);

    let cmd = "switch-client -t :.1\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());
}

/// Switch with -E (no environment update).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_no_env() {
    let tmux = TmuxServer::new("switchc_E");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sess1", "-x80", "-y24"]);
    tmux.run(&["new", "-d", "-s", "sess2"]);

    let cmd = "switch-client -E -t sess2\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach", "-t", "sess1"], cmd.as_bytes());
    assert!(output.status.success());
}

/// switchc alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn switch_client_alias() {
    let tmux = TmuxServer::new("switchc_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-s", "sess1", "-x80", "-y24"]);
    tmux.run(&["new", "-d", "-s", "sess2"]);

    let cmd = "switchc -n\ndetach-client\n";
    let output = tmux.run_with_stdin(&["-C", "attach", "-t", "sess1"], cmd.as_bytes());
    assert!(output.status.success());
}
