use super::*;

/// has-session should return 1 on error (translates has-session-return.sh)
#[test]
fn has_session_return() {
    let tmux = TmuxServer::new("has_session_return");

    // has -tfoo with no server should fail
    let output = tmux.try_run(&["-f/dev/null", "has", "-tfoo"]);
    assert!(
        !output.status.success(),
        "has -tfoo should fail with no server"
    );

    // start; has -tfoo should fail (server started but no session named foo)
    let output = tmux.try_run(&["-f/dev/null", "start", ";", "has", "-tfoo"]);
    assert!(!output.status.success(), "start; has -tfoo should fail");

    // new -d; has -tfoo should fail (session created with default name, not foo)
    let output = tmux.try_run(&["-f/dev/null", "new", "-d", ";", "has", "-tfoo"]);
    assert!(!output.status.success(), "new -d; has -tfoo should fail");

    // new -dsfoo; has -tfoo should succeed (session named foo exists)
    let output = tmux.try_run(&["-f/dev/null", "new", "-dsfoo", ";", "has", "-tfoo"]);
    assert!(
        output.status.success(),
        "new -dsfoo; has -tfoo should succeed"
    );
}
