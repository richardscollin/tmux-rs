use super::*;

/// Test argument parsing edge cases to exercise arguments.rs.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn args_escape_in_list_keys() {
    let tmux = TmuxServer::new("args_esc");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind with special characters to exercise args_escape in list-keys output
    tmux.run(&["bind-key", "Q", "display-message", "hello world"]);
    tmux.run(&["bind-key", "W", "display-message", "with 'quotes'"]);
    tmux.run(&["bind-key", "E", "display-message", "with \"double\""]);

    let out = tmux.run(&["list-keys", "-T", "prefix"]);
    assert!(out.contains("Q") && out.contains("W") && out.contains("E"));
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn args_percentage_option() {
    let tmux = TmuxServer::new("args_pct");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // resize-pane uses args_percentage for -x/-y with % suffix
    tmux.run(&["resize-pane", "-x", "50%"]);
    tmux.run(&["resize-pane", "-y", "50%"]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn args_strtonum_errors() {
    let tmux = TmuxServer::new("args_strerr");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Invalid number in -N flag for send-keys
    let out = tmux.try_run(&["send-keys", "-N", "0", "x"]);
    assert!(!out.status.success());

    // Invalid resize value
    let out = tmux.try_run(&["resize-window", "-x", "999999999"]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn args_print_complex_binding() {
    let tmux = TmuxServer::new("args_print");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Bind with multiple args to test args_print
    let tmp = tmux.write_temp("bind-key R run-shell -b 'echo test'\n");
    tmux.run(&["source-file", tmp.path_str()]);

    let out = tmux.run(&["list-keys", "-T", "prefix"]);
    assert!(out.contains("R"));
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn args_with_empty_string() {
    let tmux = TmuxServer::new("args_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Command with empty string argument
    tmux.run(&["set", "-g", "@emptyarg", ""]);
    let out = tmux.run(&["show", "-gv", "@emptyarg"]);
    assert!(out.trim().is_empty());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn args_flag_value_missing() {
    let tmux = TmuxServer::new("args_flagmiss");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Flag requiring value but none given
    let out = tmux.try_run(&["resize-window", "-x"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn args_multiple_flags() {
    let tmux = TmuxServer::new("args_multi");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // Multiple flags combined: resize-pane -x 40 -y 12
    tmux.run(&["resize-pane", "-x", "40", "-y", "12"]);
}
