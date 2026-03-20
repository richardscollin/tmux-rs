use super::*;

/// Test if-shell: -F (format/immediate), true/false branches, else clause.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn if_shell_format_true() {
    let tmux = TmuxServer::new("ifsh_ftrue");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // if-shell -F: immediate evaluation
    // "1" is truthy — should run the first command
    tmux.run(&["if-shell", "-F", "1", "set -g @ifresult yes"]);
    let val = tmux.run(&["show-options", "-gv", "@ifresult"]);
    assert_eq!(val.trim(), "yes");
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
fn if_shell_format_false_no_else() {
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
fn if_shell_background() {
    let tmux = TmuxServer::new("ifsh_bg");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -b: run in background (don't block command queue)
    tmux.run(&["if-shell", "-b", "true", "set -g @ifr7 yes"]);
    sleep_ms(500);
    let val = tmux.run(&["show-options", "-gv", "@ifr7"]);
    assert_eq!(val.trim(), "yes");
}
