use super::*;

/// List clients on a detached session (no clients expected).
#[test]
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
fn list_clients_format() {
    let tmux = TmuxServer::new("lsc_fmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["lsc", "-F", "#{client_name}"]);
    let _ = out; // May be empty, just verify no crash
}

/// List clients for a specific session.
#[test]
fn list_clients_session() {
    let tmux = TmuxServer::new("lsc_session");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24", "-s", "test"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["lsc", "-t", "test"]);
    let _ = out;
}
