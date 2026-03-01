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
    tmux.run(&["if", "-b", "false", "set -g @result changed", "set -g @result else"]);
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
    tmux.run(&["if", "-F", "#{session_windows}", "set -g @result yes", "set -g @result no"]);
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
