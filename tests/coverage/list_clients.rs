use super::*;

/// List clients on a detached session (no clients expected).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_clients_empty() {
    let tmux = TmuxServer::new("lsc_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["lsc"]);
    // No attached clients in detached mode
    assert!(
        out.trim().is_empty(),
        "no clients expected on detached session"
    );
}

/// List clients with custom format.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_clients_format() {
    let tmux = TmuxServer::new("lsc_fmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["lsc", "-F", "#{client_name}"]);
    let _ = out; // May be empty, just verify no crash
}

/// List clients for a specific session.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_clients_session() {
    let tmux = TmuxServer::new("lsc_session");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "test"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["lsc", "-t", "test"]);
    let _ = out;
}

/// List clients with an attached control-mode client (exercises the loop body).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_clients_with_client() {
    let tmux = TmuxServer::new("lsc_client");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Attach a control-mode client, list clients, then detach
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"list-clients\nlist-clients -F '#{client_name}'\ndetach-client\n",
    );
    assert!(output.status.success());
}

/// List clients with -t session filter (with a control-mode client attached).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_clients_session_with_client() {
    let tmux = TmuxServer::new("lsc_sess_client");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "mysess"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Attach control-mode client, list clients filtered by session
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"list-clients -t mysess\ndetach-client\n",
    );
    assert!(output.status.success());
}

/// List clients with filter -f.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_clients_filter() {
    let tmux = TmuxServer::new("lsc_filter");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Attach control-mode client, list with matching and non-matching filters
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"list-clients -f '#{==:#{line},0}'\nlist-clients -f '#{==:#{line},999}'\ndetach-client\n",
    );
    assert!(output.status.success());
}

/// List clients with -t pointing to a different session (session mismatch).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_clients_session_mismatch() {
    let tmux = TmuxServer::new("lsc_mismatch");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "sess1"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new", "-d", "-s", "sess2"]);

    // Attach to sess1, but list clients of sess2 (should find none)
    let output = tmux.run_with_stdin(
        &["-C", "attach", "-t", "sess1"],
        b"list-clients -t sess2\ndetach-client\n",
    );
    assert!(output.status.success());
}
