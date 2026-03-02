use super::*;

/// Unbind a specific key from prefix table (default).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_basic() {
    let tmux = TmuxServer::new("unbind_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind then unbind a key in prefix table
    tmux.run(&["bind", "x", "display", "hello"]);
    tmux.run(&["unbind", "x"]);

    let keys = tmux.run(&["list-keys", "-T", "prefix"]);
    assert!(
        !keys
            .lines()
            .any(|l| l.contains(" x ") && l.contains("display")),
        "key 'x' should no longer be bound"
    );
}

/// Unbind a key from root table with -n.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_root_table() {
    let tmux = TmuxServer::new("unbind_root");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["bind", "-n", "F5", "display", "hello"]);
    tmux.run(&["unbind", "-n", "F5"]);
}

/// Unbind a key from a specific table with -T.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_specific_table() {
    let tmux = TmuxServer::new("unbind_table");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["bind", "-T", "copy-mode", "x", "send-keys", "-X", "cancel"]);
    tmux.run(&["unbind", "-T", "copy-mode", "x"]);
}

/// Unbind with -T pointing to nonexistent table.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_nonexistent_table() {
    let tmux = TmuxServer::new("unbind_notable");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["unbind", "-T", "nonexistent", "x"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("doesn't exist"),
        "expected 'doesn't exist' error, got: {stderr}"
    );
}

/// Unbind with -T nonexistent table and -q (quiet, suppresses error message).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_nonexistent_table_quiet() {
    let tmux = TmuxServer::new("unbind_notable_q");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -q suppresses error messages but still returns error
    let result = tmux.try_run(&["unbind", "-q", "-T", "nonexistent", "x"]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    // With -q the error message should not appear
    assert!(
        !stderr.contains("doesn't exist"),
        "with -q, error message should be suppressed"
    );
}

/// Unbind all keys with -a and -T for a specific table.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_all_specific_table() {
    let tmux = TmuxServer::new("unbind_all_tbl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create a custom table with a binding then unbind all from it
    tmux.run(&["bind", "-T", "mytable", "x", "display", "hi"]);
    tmux.run(&["unbind", "-a", "-T", "mytable"]);
}

/// Unbind all with -a on nonexistent table.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_all_nonexistent_table() {
    let tmux = TmuxServer::new("unbind_all_noex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["unbind", "-a", "-T", "nonexistent"]);
    assert!(!result.status.success());
}

/// Unbind all with -a on nonexistent table, quiet.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_all_nonexistent_quiet() {
    let tmux = TmuxServer::new("unbind_all_noex_q");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["unbind", "-a", "-q", "-T", "nonexistent"]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !stderr.contains("doesn't exist"),
        "with -q, error message should be suppressed"
    );
}

/// Unbind with -a and a key argument (error: key given with -a).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_all_with_key_error() {
    let tmux = TmuxServer::new("unbind_a_key_err");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["unbind", "-a", "x"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("key given with -a"),
        "expected 'key given with -a' error, got: {stderr}"
    );
}

/// Unbind with -a and key, quiet mode (suppresses error message).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_all_with_key_quiet() {
    let tmux = TmuxServer::new("unbind_a_key_q");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["unbind", "-a", "-q", "x"]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !stderr.contains("key given with -a"),
        "with -q, error message should be suppressed"
    );
}

/// Unbind with no key argument (missing key error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_missing_key() {
    let tmux = TmuxServer::new("unbind_nokey");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["unbind"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("missing key"),
        "expected 'missing key' error, got: {stderr}"
    );
}

/// Unbind with no key, quiet mode (suppresses error message).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_missing_key_quiet() {
    let tmux = TmuxServer::new("unbind_nokey_q");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["unbind", "-q"]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !stderr.contains("missing key"),
        "with -q, error message should be suppressed"
    );
}

/// Unbind an unknown key name.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_unknown_key() {
    let tmux = TmuxServer::new("unbind_unkkey");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["unbind", "INVALIDKEYNAME"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("unknown key"),
        "expected 'unknown key' error, got: {stderr}"
    );
}

/// Unbind an unknown key, quiet mode (suppresses error message).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn unbind_key_unknown_key_quiet() {
    let tmux = TmuxServer::new("unbind_unkkey_q");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["unbind", "-q", "INVALIDKEYNAME"]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !stderr.contains("unknown key"),
        "with -q, error message should be suppressed"
    );
}

/// Unbind all from prefix table crashes server - this is a real bug.
#[test]
#[ignore = "crashes server: unbind -a removes all prefix bindings causing server exit"]
fn unbind_key_all_prefix_crash() {
    let tmux = TmuxServer::new("unbind_all_prefix");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["unbind", "-a"]);
}
