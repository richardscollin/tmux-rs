use super::*;

/// if-shell with true condition runs if-command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_true() {
    let tmux = TmuxServer::new("if_true");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["if", "true", "set -g @result yes", "set -g @result no"]);
    let out = tmux.run(&["show", "-gv", "@result"]);
    assert_eq!(out.trim(), "yes");
}

/// if-shell with false condition runs else-command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_false() {
    let tmux = TmuxServer::new("if_false");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["if", "false", "set -g @result yes", "set -g @result no"]);
    let out = tmux.run(&["show", "-gv", "@result"]);
    assert_eq!(out.trim(), "no");
}

/// if-shell with false and no else command (no-op).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_false_no_else() {
    let tmux = TmuxServer::new("if_false_noelse");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-g", "@result", "unchanged"]);
    tmux.run(&["if", "false", "set -g @result changed"]);
    let out = tmux.run(&["show", "-gv", "@result"]);
    assert_eq!(out.trim(), "unchanged");
}

/// if-shell with -F (format, no shell) truthy condition.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_format_true() {
    let tmux = TmuxServer::new("if_fmt_true");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["if", "-F", "1", "set -g @result yes", "set -g @result no"]);
    let out = tmux.run(&["show", "-gv", "@result"]);
    assert_eq!(out.trim(), "yes");
}

/// if-shell with -F falsy condition ("0").
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_format_false_zero() {
    let tmux = TmuxServer::new("if_fmt_false0");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["if", "-F", "0", "set -g @result yes", "set -g @result no"]);
    let out = tmux.run(&["show", "-gv", "@result"]);
    assert_eq!(out.trim(), "no");
}

/// if-shell with -F falsy condition (empty string).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_format_false_empty() {
    let tmux = TmuxServer::new("if_fmt_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["if", "-F", "", "set -g @result yes", "set -g @result no"]);
    let out = tmux.run(&["show", "-gv", "@result"]);
    assert_eq!(out.trim(), "no");
}

/// if-shell with -F falsy and no else (no-op).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_format_false_no_else() {
    let tmux = TmuxServer::new("if_fmt_noelse");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-g", "@result", "unchanged"]);
    tmux.run(&["if", "-F", "0", "set -g @result changed"]);
    let out = tmux.run(&["show", "-gv", "@result"]);
    assert_eq!(out.trim(), "unchanged");
}

/// if-shell with -b (background, don't wait).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_background() {
    let tmux = TmuxServer::new("if_bg");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["if", "-b", "true", "set -g @result yes"]);
    // Background - need to wait a moment for the job to complete
    sleep_ms(500);
    let out = tmux.run(&["show", "-gv", "@result"]);
    assert_eq!(out.trim(), "yes");
}

/// if-shell with -b and false condition.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_background_false() {
    let tmux = TmuxServer::new("if_bg_false");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["set", "-g", "@result", "unchanged"]);
    tmux.run(&[
        "if",
        "-b",
        "false",
        "set -g @result changed",
        "set -g @result else",
    ]);
    sleep_ms(500);
    let out = tmux.run(&["show", "-gv", "@result"]);
    assert_eq!(out.trim(), "else");
}

/// if-shell with -F using format expansion.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_format_expansion() {
    let tmux = TmuxServer::new("if_fmt_expand");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // #{session_windows} should be "1" (truthy)
    tmux.run(&[
        "if",
        "-F",
        "#{session_windows}",
        "set -g @result yes",
        "set -g @result no",
    ]);
    let out = tmux.run(&["show", "-gv", "@result"]);
    assert_eq!(out.trim(), "yes");
}

/// if-shell with control-mode client (exercises client references).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_with_client() {
    let tmux = TmuxServer::new("if_client");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"if-shell true 'set -g @result yes'\nif-shell -b true 'set -g @result bg_yes'\ndetach-client\n",
    );
    assert!(output.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_format_false() {
    let tmux = TmuxServer::new("ifsh_ffalse");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // "0" is falsy — should run the else command
    tmux.run(&["if-shell", "-F", "0", "set -g @ifr1 yes", "set -g @ifr1 no"]);
    let val = tmux.run(&["show-options", "-gv", "@ifr1"]);
    assert_eq!(val.trim(), "no");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_format_false_no_else_02() {
    let tmux = TmuxServer::new("ifsh_fnoelse");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // "0" with no else clause: should be a no-op
    tmux.run(&["if-shell", "-F", "0", "set -g @ifr2 yes"]);
    let out = tmux.try_run(&["show-options", "-gv", "@ifr2"]);
    // @ifr2 should not exist
    assert!(
        !out.status.success() || String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "should not set @ifr2 when condition is false with no else"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_format_empty_string() {
    let tmux = TmuxServer::new("ifsh_fempty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Empty string is falsy
    tmux.run(&["if-shell", "-F", "", "set -g @ifr3 yes", "set -g @ifr3 no"]);
    let val = tmux.run(&["show-options", "-gv", "@ifr3"]);
    assert_eq!(val.trim(), "no");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_format_with_format_string() {
    let tmux = TmuxServer::new("ifsh_ffmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Use an actual format string as the condition
    // #{session_windows} should be "1" (truthy)
    tmux.run(&["if-shell", "-F", "#{session_windows}", "set -g @ifr4 yes"]);
    let val = tmux.run(&["show-options", "-gv", "@ifr4"]);
    assert_eq!(val.trim(), "yes");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_async_true() {
    let tmux = TmuxServer::new("ifsh_async");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Without -F: runs shell command asynchronously
    // "true" exits 0 (truthy) — should run first command
    tmux.run(&["if-shell", "true", "set -g @ifr5 yes", "set -g @ifr5 no"]);
    sleep_ms(500);
    let val = tmux.run(&["show-options", "-gv", "@ifr5"]);
    assert_eq!(val.trim(), "yes");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_async_false() {
    let tmux = TmuxServer::new("ifsh_asyncf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // "false" exits 1 (falsy) — should run else command
    tmux.run(&["if-shell", "false", "set -g @ifr6 yes", "set -g @ifr6 no"]);
    sleep_ms(500);
    let val = tmux.run(&["show-options", "-gv", "@ifr6"]);
    assert_eq!(val.trim(), "no");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_background_02() {
    let tmux = TmuxServer::new("ifsh_bg");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -b: run in background (don't block command queue)
    tmux.run(&["if-shell", "-b", "true", "set -g @ifr7 yes"]);
    sleep_ms(500);
    let val = tmux.run(&["show-options", "-gv", "@ifr7"]);
    assert_eq!(val.trim(), "yes");
}
