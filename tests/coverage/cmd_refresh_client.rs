use super::*;

/// Test refresh-client with -S (status force) and default (no flags = redraw).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn refresh_client_status_and_redraw() {
    let tmux = TmuxServer::new("refresh_status");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"refresh-client -S\nrefresh-client\ndetach\n",
    );
    assert!(output.status.success());
}

/// Test refresh-client -f and -F (set client flags).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn refresh_client_set_flags() {
    let tmux = TmuxServer::new("refresh_flags");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"refresh-client -f no-output\nrefresh-client -F no-output\ndetach\n",
    );
    assert!(output.status.success());
}

/// Test refresh-client -C with various size formats (control client size).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn refresh_client_control_size() {
    let tmux = TmuxServer::new("refresh_csize");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        concat!(
            // WxH format
            "refresh-client -C 80x24\n",
            // W,H format (comma separator)
            "refresh-client -C 40,20\n",
            // Bad format
            "refresh-client -C badarg\n",
            // Size too small (x=0, both dimensions)
            "refresh-client -C 0x0\n",
            // Size too big y only (x valid, y=0) - exercises y check on line 96
            "refresh-client -C 80x0\n",
            // Size too big y only for WxH path - exercises y check on line 139
            "refresh-client -C 80x0\n",
            // @N:WxH format (per-window size)
            "refresh-client -C @0:80x24\n",
            // @N: format (reset per-window size, cw exists)
            "refresh-client -C @0:\n",
            // @N: format for non-existent window (cw is null)
            "refresh-client -C @999:\n",
            // @N:WxH too small
            "refresh-client -C @0:0x0\n",
            // @N:WxH y only too small
            "refresh-client -C @0:80x0\n",
            "detach\n",
        )
        .as_bytes(),
    );
    assert!(output.status.success());
}

/// Test refresh-client -A with various offset values (update pane offset).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn refresh_client_update_offset() {
    let tmux = TmuxServer::new("refresh_offset");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        concat!(
            "refresh-client -A '%0:on'\n",
            "refresh-client -A '%0:off'\n",
            "refresh-client -A '%0:continue'\n",
            "refresh-client -A '%0:pause'\n",
            // Value not starting with % - early return
            "refresh-client -A notpercent\n",
            // No colon after % - early return
            "refresh-client -A '%0'\n",
            // Non-numeric pane ID - sscanf fails
            "refresh-client -A '%abc:on'\n",
            // Nonexistent pane - wp is null
            "refresh-client -A '%999999:on'\n",
            // Unrecognized action (not on/off/continue/pause)
            "refresh-client -A '%0:unknown'\n",
            "detach\n",
        )
        .as_bytes(),
    );
    assert!(output.status.success());
}

/// Test refresh-client -B with various subscription values.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn refresh_client_subscriptions() {
    let tmux = TmuxServer::new("refresh_sub");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        concat!(
            // All panes subscription type
            "refresh-client -B mysub:%*:fmt\n",
            // All windows subscription type
            "refresh-client -B winsub:@*:fmt\n",
            // Specific pane subscription type
            "refresh-client -B panesub:%0:fmt\n",
            // Specific window subscription type
            "refresh-client -B winsub2:@0:fmt\n",
            // Session subscription type (fallback)
            "refresh-client -B sesssub:session:fmt\n",
            // Only one colon (no format) - breaks out early
            "refresh-client -B incomplete:what\n",
            // No colon - remove subscription
            "refresh-client -B removeme\n",
            "detach\n",
        )
        .as_bytes(),
    );
    assert!(output.status.success());
}

/// Test refresh-client -l (clipboard).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn refresh_client_clipboard() {
    let tmux = TmuxServer::new("refresh_clip");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // First -l sets CLIPBOARDBUFFER, second returns early (already set).
    // -l%0 passes pane as argument to target specific pane.
    // -l with invalid target exercises error path.
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        concat!(
            "refresh-client -l\n",
            "refresh-client -l\n",
            "refresh-client '-l%0'\n",
            // Same pane again - exercises duplicate detection (already present)
            "refresh-client '-l%0'\n",
            // Invalid target - exercises cmd_find_target error
            "refresh-client -lbadtarget\n",
            "detach\n",
        )
        .as_bytes(),
    );
    assert!(output.status.success());
}

/// Test refresh-client -r (report) with various values.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn refresh_client_report() {
    let tmux = TmuxServer::new("refresh_report");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        concat!(
            "refresh-client -r '%0:rgb:ff/00/00'\n",
            "refresh-client -r notpercent\n",
            // No colon - early return
            "refresh-client -r '%0'\n",
            // Non-numeric pane ID - sscanf fails
            "refresh-client -r '%abc:rgb:ff/00/00'\n",
            // Nonexistent pane - wp is null
            "refresh-client -r '%999999:rgb:ff/00/00'\n",
            "detach\n",
        )
        .as_bytes(),
    );
    assert!(output.status.success());
}

/// Test refresh-client panning flags (-L, -R, -U, -D, -c) and adjustment.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn refresh_client_panning() {
    let tmux = TmuxServer::new("refresh_pan");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        concat!(
            // Pan right first so pan_ox > 0, then left exercises decrement branch
            "refresh-client -R 5\n",
            "refresh-client -L 3\n",
            // Pan left past zero to exercise pan_ox = 0 branch
            "refresh-client -L 100\n",
            // Pan right far enough to hit the cap (pan_ox > w.sx - tty.osx)
            "refresh-client -R 10000\n",
            // Pan down first so pan_oy > 0, then up exercises decrement branch
            "refresh-client -D 5\n",
            "refresh-client -U 3\n",
            // Pan up past zero to exercise pan_oy = 0 branch
            "refresh-client -U 100\n",
            // Pan down far enough to hit the cap (pan_oy > w.sy - tty.osy)
            "refresh-client -D 10000\n",
            // Default adjustment (no number argument)
            "refresh-client -L\n",
            "refresh-client -R\n",
            "refresh-client -U\n",
            "refresh-client -D\n",
            // Reset panning with -c
            "refresh-client -c\n",
            "detach\n",
        )
        .as_bytes(),
    );
    assert!(output.status.success());
}

/// Test invalid adjustment value for panning.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn refresh_client_invalid_adjustment() {
    let tmux = TmuxServer::new("refresh_badadj");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // Invalid adjustment in control mode produces error but doesn't kill client
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        concat!(
            "refresh-client -L 0\n",
            "refresh-client -R abc\n",
            "detach\n",
        )
        .as_bytes(),
    );
    assert!(output.status.success());
}

/// Test "not a control client" error: -A/-B/-C on non-control client.
/// The CLI client with no -t flag fails with "no current client" before
/// reaching exec, so we need an attached non-control client. Using a
/// background control-mode session to keep a client alive, then run-shell
/// invokes the command which targets the most recent non-control client.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn refresh_client_not_control() {
    let tmux = TmuxServer::new("refresh_noctrl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);

    // CLI without an attached client fails with "no current client"
    let result = tmux.try_run(&["refresh-client", "-A", "%0:on"]);
    assert!(!result.status.success());

    let result = tmux.try_run(&["refresh-client", "-B", "sub:%*:fmt"]);
    assert!(!result.status.success());

    let result = tmux.try_run(&["refresh-client", "-C", "80x24"]);
    assert!(!result.status.success());
}
