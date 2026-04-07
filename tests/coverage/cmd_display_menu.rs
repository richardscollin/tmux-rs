use super::*;

/// Helper: run a command through a control-mode client and return stdout.
fn control_cmd(tmux: &TmuxServer, cmd: &str) -> String {
    let input = format!("{cmd}\ndetach\n");
    let output = tmux.run_with_stdin(&["-C", "attach"], input.as_bytes());
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Helper: run multiple commands through a single control-mode session.
fn control_cmds(tmux: &TmuxServer, cmds: &[&str]) -> String {
    let mut input = String::new();
    for cmd in cmds {
        input.push_str(cmd);
        input.push('\n');
    }
    input.push_str("detach\n");
    let output = tmux.run_with_stdin(&["-C", "attach"], input.as_bytes());
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Helper: run a command through control mode, assert server stays alive.
fn control_cmd_ok(tmux: &TmuxServer, cmd: &str) {
    let stdout = control_cmd(tmux, cmd);
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed running '{cmd}': {stdout}"
    );
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success(), "server died after '{cmd}'");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_menu_exec() {
    let tmux = TmuxServer::new("display_menu_exec");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Basic menu with title (exercises -T flag and item triplet parsing)
    control_cmd_ok(
        &tmux,
        "display-menu -T 'Test Menu' Item1 a 'set -g status off' Item2 b 'set -g status on'",
    );

    // No -T flag: title defaults to empty string
    control_cmd_ok(&tmux, "display-menu Item1 a 'set -g status off'");

    // Separator (empty name) between items
    control_cmd_ok(
        &tmux,
        "display-menu Top a 'set -g status off' '' Bottom b 'set -g status on'",
    );

    // -O (stay-open) and -M (mouse mode) flags
    control_cmd_ok(
        &tmux,
        "display-menu -O -M Top a 'set -g status off' '' Bottom b 'set -g status on'",
    );

    // -C numeric starting choice
    control_cmd_ok(
        &tmux,
        "display-menu -C 1 Item1 a 'set -g status off' Item2 b 'set -g status on'",
    );

    // -C '-' means starting_choice = -1
    control_cmd_ok(
        &tmux,
        "display-menu -C - Item1 a 'set -g status off' Item2 b 'set -g status on'",
    );

    // Styles: -s, -S, -H
    control_cmd_ok(
        &tmux,
        "display-menu -s 'fg=red' -S 'fg=blue' -H 'fg=green' Item1 a 'set -g status off'",
    );

    // Valid -b border-lines
    control_cmd_ok(&tmux, "display-menu -b single Item1 a 'set -g status off'");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_menu_errors() {
    let tmux = TmuxServer::new("display_menu_errors");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -C with invalid value
    let stdout = control_cmd(
        &tmux,
        "display-menu -C notanumber Item1 a 'set -g status off'",
    );
    assert!(
        stdout.contains("starting choice"),
        "expected 'starting choice' error, got: {stdout}"
    );

    // Invalid -b border-lines value
    let stdout = control_cmd(
        &tmux,
        "display-menu -b invalid_border Item1 a 'set -g status off'",
    );
    assert!(
        stdout.contains("menu-border-lines"),
        "expected border-lines error, got: {stdout}"
    );

    // Not enough arguments (name without key+command)
    let stdout = control_cmd(&tmux, "display-menu Item1 a");
    assert!(
        stdout.contains("not enough arguments"),
        "expected 'not enough arguments' error, got: {stdout}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_menu_positions() {
    let tmux = TmuxServer::new("display_menu_positions");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Default (no -x/-y): uses centre
    control_cmd_ok(&tmux, "display-menu Item1 a 'set -g status off'");

    // -x C -y C: explicit centre
    control_cmd_ok(&tmux, "display-menu -x C -y C Item1 a 'set -g status off'");

    // -x R: pane right, -y P: pane bottom
    control_cmd_ok(&tmux, "display-menu -x R -y P Item1 a 'set -g status off'");

    // -x P: pane left, -y S: status line
    control_cmd_ok(&tmux, "display-menu -x P -y S Item1 a 'set -g status off'");

    // Numeric positions
    control_cmd_ok(&tmux, "display-menu -x 5 -y 10 Item1 a 'set -g status off'");

    // -x W: window status line position, -y W: window status line y
    control_cmd_ok(&tmux, "display-menu -x W -y W Item1 a 'set -g status off'");

    // Large numeric position that would exceed terminal size (clamp path)
    control_cmd_ok(
        &tmux,
        "display-menu -x 200 -y 200 Item1 a 'set -g status off'",
    );

    // Zero position
    control_cmd_ok(&tmux, "display-menu -x 0 -y 0 Item1 a 'set -g status off'");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_popup_exec() {
    let tmux = TmuxServer::new("display_popup_exec");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Basic popup with -E (close on exit) and a quick command
    control_cmd_ok(&tmux, "display-popup -E true");

    // Custom dimensions -w and -h
    control_cmd_ok(&tmux, "display-popup -w 40 -h 10 -E true");

    // Percentage dimensions
    control_cmd_ok(&tmux, "display-popup -w 50% -h 50% -E true");

    // -T title
    control_cmd_ok(&tmux, "display-popup -T 'My Popup' -E true");

    // No -T: title defaults to empty
    control_cmd_ok(&tmux, "display-popup -E true");

    // -B: no border
    control_cmd_ok(&tmux, "display-popup -B -E true");

    // -b: border style
    control_cmd_ok(&tmux, "display-popup -b single -E true");

    // -EE: close on exit zero
    control_cmd_ok(&tmux, "display-popup -E -E true");

    // -k: close on any keypress
    control_cmd_ok(&tmux, "display-popup -k -E true");

    // -d: start directory
    control_cmd_ok(&tmux, "display-popup -d /tmp -E true");

    // -e: environment variable
    control_cmd_ok(&tmux, "display-popup -e MY_VAR=hello -E true");

    // -s style, -S border-style
    control_cmd_ok(&tmux, "display-popup -s 'fg=red' -S 'fg=blue' -E true");

    // No arguments: uses default-command/default-shell
    control_cmd_ok(&tmux, "display-popup");

    // -N flag
    control_cmd_ok(&tmux, "display-popup -N -E true");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_popup_close() {
    let tmux = TmuxServer::new("display_popup_close");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -C closes existing popup (no-op when none exists)
    control_cmd_ok(&tmux, "display-popup -C");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_popup_errors() {
    let tmux = TmuxServer::new("display_popup_errors");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Invalid height
    let stdout = control_cmd(&tmux, "display-popup -h notanumber true");
    assert!(
        stdout.contains("height"),
        "expected height error, got: {stdout}"
    );

    // Invalid width
    let stdout = control_cmd(&tmux, "display-popup -w notanumber true");
    assert!(
        stdout.contains("width"),
        "expected width error, got: {stdout}"
    );

    // Invalid border style
    let stdout = control_cmd(&tmux, "display-popup -b invalid_border true");
    assert!(
        stdout.contains("popup-border-lines"),
        "expected border-lines error, got: {stdout}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_popup_positions() {
    let tmux = TmuxServer::new("display_popup_positions");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Various position options for popup
    control_cmd_ok(&tmux, "display-popup -x C -y C -E true");
    control_cmd_ok(&tmux, "display-popup -x R -y P -E true");
    control_cmd_ok(&tmux, "display-popup -x P -y S -E true");
    control_cmd_ok(&tmux, "display-popup -x 5 -y 5 -E true");
    control_cmd_ok(&tmux, "display-popup -x W -y W -E true");

    // Clamping: large values
    control_cmd_ok(&tmux, "display-popup -x 200 -y 200 -E true");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_menu_empty_menu() {
    let tmux = TmuxServer::new("display_menu_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Menu with only separators: items list is empty, triggers early return
    control_cmd_ok(&tmux, "display-menu ''");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_menu_overlay_already_present() {
    let tmux = TmuxServer::new("display_menu_overlay");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Open a popup (which sets overlay_draw), then try display-menu.
    // display-menu should return early because overlay is already present.
    // Also exercises the popup modify path when display-popup is called
    // with an existing popup.
    let stdout = control_cmds(
        &tmux,
        &[
            "display-popup -E cat",
            "display-menu Item1 a 'set -g status off'",
        ],
    );
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed: {stdout}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_popup_modify() {
    let tmux = TmuxServer::new("display_popup_modify");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Open a popup, then modify it (exercises the popup_modify path).
    // The second display-popup on an existing popup calls popup_modify.
    let stdout = control_cmds(
        &tmux,
        &[
            "display-popup cat",
            "display-popup -T 'New Title' -b single -s 'fg=red' -S 'fg=blue'",
        ],
    );
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed: {stdout}"
    );

    // Modify with -E and -k flags (exercises flags != -1 paths in modify context)
    let stdout = control_cmds(&tmux, &["display-popup cat", "display-popup -E -k"]);
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed: {stdout}"
    );

    // Modify with -EE flag
    let stdout = control_cmds(&tmux, &["display-popup cat", "display-popup -E -E"]);
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed: {stdout}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_popup_close_existing_popup() {
    let tmux = TmuxServer::new("display_popup_closeex2");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Open a popup, then close it with -C (exercises server_client_clear_overlay)
    let stdout = control_cmds(&tmux, &["display-popup cat", "display-popup -C"]);
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed: {stdout}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_popup_default_command() {
    let tmux = TmuxServer::new("display_popup_defcmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // count==0 path: uses default-command (empty by default -> falls through to
    // default-shell). Exercises the shellcmd.is_null()/empty path.
    control_cmd_ok(&tmux, "display-popup -E");

    // Set default-command to empty explicitly
    tmux.run(&["set", "-g", "default-command", ""]);
    control_cmd_ok(&tmux, "display-popup -E");

    // count==1 with a single command
    control_cmd_ok(&tmux, "display-popup -E true");

    // count>1 with multiple args (exercises args_to_vector path)
    control_cmd_ok(&tmux, "display-popup -E echo hello world");
}

/// BUG: Server crashes when display-menu is run on a control client with
/// a very small terminal size (4x4 or smaller via refresh-client -C).
/// Upstream tmux handles this gracefully. The crash likely occurs during
/// window/pane resize when the terminal is too small.
/// Threshold: 5x5 works, 4x4 crashes.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn display_menu_small_terminal_crash() {
    let tmux = TmuxServer::new("display_menu_tiny");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let stdout = control_cmds(
        &tmux,
        &[
            "refresh-client -C 4,4",
            "display-menu A a 'set -g status off'",
        ],
    );
    eprintln!("STDOUT: {stdout}");
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed with 4x4 terminal + display-menu"
    );
    let has = tmux.try_run(&["has-session"]);
    assert!(has.status.success(), "server died");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_menu_small_terminal() {
    let tmux = TmuxServer::new("display_menu_small");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // 10x5 terminal with a long menu item name: menu too wide, get_pos returns 0
    let stdout = control_cmds(
        &tmux,
        &[
            "refresh-client -C 10,5",
            "display-menu 'VeryLongMenuItemNameThatExceedsWidth' a 'set -g status off'",
        ],
    );
    assert!(
        !stdout.contains("server exited unexpectedly"),
        "server crashed: {stdout}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn test_display_menu_position_m() {
    let tmux = TmuxServer::new("display_menu_pos_m");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -x M and -y M: mouse position (without actual mouse event, format
    // variables will be empty/0, but the code path is still exercised)
    control_cmd_ok(&tmux, "display-menu -x M -y M Item1 a 'set -g status off'");
}
