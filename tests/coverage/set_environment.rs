use super::*;

/// Test set-environment: set, unset (-u), clear (-r), hidden (-h),
/// global (-g), format (-F), and error paths.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn setenv_global() {
    let tmux = TmuxServer::new("setenv_global");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "TESTVAR", "testval"]);
    let out = tmux.run(&["showenv", "-g", "TESTVAR"]);
    assert!(out.contains("TESTVAR=testval"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn setenv_session() {
    let tmux = TmuxServer::new("setenv_session");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "SESSVAR", "sessval"]);
    let out = tmux.run(&["showenv", "SESSVAR"]);
    assert!(out.contains("SESSVAR=sessval"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn setenv_unset() {
    let tmux = TmuxServer::new("setenv_unset");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["setenv", "-g", "DELVAR", "delval"]);
    tmux.run(&["setenv", "-gu", "DELVAR"]);
    let out = tmux.try_run(&["showenv", "-g", "DELVAR"]);
    // Should be gone or show as unset
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("DELVAR=delval"), "should be unset, got: {stdout}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn setenv_clear() {
    let tmux = TmuxServer::new("setenv_clear");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -r: mark for removal (clear)
    tmux.run(&["setenv", "-g", "CLEARVAR", "val"]);
    tmux.run(&["setenv", "-gr", "CLEARVAR"]);
    let out = tmux.run(&["showenv", "-g", "CLEARVAR"]);
    // Should show as -CLEARVAR (marked for removal)
    assert!(out.contains("-CLEARVAR"), "should be marked for removal, got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn setenv_hidden() {
    let tmux = TmuxServer::new("setenv_hidden");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -h: hidden variable
    tmux.run(&["setenv", "-gh", "HIDDENVAR", "secret"]);
    // Hidden vars don't show in normal showenv
    let out = tmux.run(&["showenv", "-g"]);
    assert!(!out.contains("HIDDENVAR"), "hidden var should not show in normal output");

    // But should show with -h flag
    let out = tmux.run(&["showenv", "-gh", "HIDDENVAR"]);
    assert!(out.contains("secret"), "hidden var should show with -h, got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn setenv_format() {
    let tmux = TmuxServer::new("setenv_fmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -F: expand format string in value
    tmux.run(&["setenv", "-gF", "FMTVAR", "#{session_name}"]);
    let out = tmux.run(&["showenv", "-g", "FMTVAR"]);
    assert!(!out.contains("#{"), "format should be expanded, got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn setenv_empty_name() {
    let tmux = TmuxServer::new("setenv_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.try_run(&["setenv", "-g", "", "val"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("empty variable"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn setenv_name_with_equals() {
    let tmux = TmuxServer::new("setenv_equals");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.try_run(&["setenv", "-g", "BAD=NAME", "val"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("contains ="), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn setenv_no_value() {
    let tmux = TmuxServer::new("setenv_noval");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // set without value and without -u/-r: should error
    let out = tmux.try_run(&["setenv", "-g", "NOVALUE"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no value"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn setenv_unset_with_value() {
    let tmux = TmuxServer::new("setenv_uval");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -u with a value: should error
    let out = tmux.try_run(&["setenv", "-gu", "VAR", "val"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("can't specify a value with -u"), "got: {stderr}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn setenv_clear_with_value() {
    let tmux = TmuxServer::new("setenv_rval");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -r with a value: should error
    let out = tmux.try_run(&["setenv", "-gr", "VAR", "val"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("can't specify a value with -r"), "got: {stderr}");
}
