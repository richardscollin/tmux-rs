use super::*;

/// Bell alert: trigger bell in non-current window, verify flag is set.
/// Covers: alerts_check_bell main path, alerts_enabled(BELL), alerts_queue,
/// alerts_reset, alerts_callback, bell-action=any (default).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_bell() {
    let tmux = TmuxServer::new("alerts_bell");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);

    // No bell flag initially
    let flag = tmux.run(&["display-message", "-t:0", "-p", "#{window_bell_flag}"]);
    assert_eq!(flag.trim(), "0");

    // Trigger bell in window 0 (non-current) via printf '\a'
    tmux.run(&["send-keys", "-t:0", "printf '\\a'", "Enter"]);
    sleep_ms(500);

    // Bell flag should be set on window 0's winlink
    let flag = tmux.run(&["display-message", "-t:0", "-p", "#{window_bell_flag}"]);
    assert_eq!(flag.trim(), "1");
}

/// Bell with monitor-bell off: early return in alerts_check_bell.
/// Covers: alerts_check_bell monitor-bell==0 branch, alerts_enabled(BELL, off).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_bell_monitor_off() {
    let tmux = TmuxServer::new("alerts_bell_off");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "-g", "monitor-bell", "off"]);
    tmux.run(&["new-window"]);

    tmux.run(&["send-keys", "-t:0", "printf '\\a'", "Enter"]);
    sleep_ms(500);

    // Bell flag should NOT be set because monitor-bell is off
    let flag = tmux.run(&["display-message", "-t:0", "-p", "#{window_bell_flag}"]);
    assert_eq!(flag.trim(), "0");
}

/// Bell-action variants: exercise all branches of alerts_action_applies.
/// Tests none/current/other with bell in both current and non-current windows.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_bell_action_variants() {
    // bell-action=none: notification skipped for all windows
    {
        let tmux = TmuxServer::new("alerts_ba_none");
        tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
        tmux.run(&["set", "-g", "window-size", "manual"]);
        tmux.run(&["set", "-g", "bell-action", "none"]);
        tmux.run(&["new-window"]);

        tmux.run(&["send-keys", "-t:0", "printf '\\a'", "Enter"]);
        sleep_ms(500);

        // WINLINK_BELL is still set (independent of action), but no notification
        let flag = tmux.run(&["display-message", "-t:0", "-p", "#{window_bell_flag}"]);
        assert_eq!(flag.trim(), "1");
    }

    // bell-action=current: applies only when wl == curw
    {
        let tmux = TmuxServer::new("alerts_ba_cur");
        tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
        tmux.run(&["set", "-g", "window-size", "manual"]);
        tmux.run(&["set", "-g", "bell-action", "current"]);
        tmux.run(&["new-window"]);

        // Bell in non-current window (wl != curw) -> action does NOT apply
        tmux.run(&["send-keys", "-t:0", "printf '\\a'", "Enter"]);
        sleep_ms(500);

        // Bell in current window (wl == curw) -> action applies
        tmux.run(&["send-keys", "-t:1", "printf '\\a'", "Enter"]);
        sleep_ms(500);
    }

    // bell-action=other: applies only when wl != curw
    {
        let tmux = TmuxServer::new("alerts_ba_other");
        tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
        tmux.run(&["set", "-g", "window-size", "manual"]);
        tmux.run(&["set", "-g", "bell-action", "other"]);
        tmux.run(&["new-window"]);

        // Bell in non-current window (wl != curw) -> action applies
        tmux.run(&["send-keys", "-t:0", "printf '\\a'", "Enter"]);
        sleep_ms(500);

        // Bell in current window (wl == curw) -> action does NOT apply
        tmux.run(&["send-keys", "-t:1", "printf '\\a'", "Enter"]);
        sleep_ms(500);
    }
}

/// Activity alert: trigger output in non-current window with monitor-activity on.
/// Covers: alerts_check_activity main path, alerts_enabled(ACTIVITY),
/// activity-action=other (default).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_activity() {
    let tmux = TmuxServer::new("alerts_activity");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "-g", "monitor-activity", "on"]);
    tmux.run(&["new-window"]);

    // Trigger output (activity) in window 0
    tmux.run(&["send-keys", "-t:0", "echo hello", "Enter"]);
    sleep_ms(500);

    let flag = tmux.run(&["display-message", "-t:0", "-p", "#{window_activity_flag}"]);
    assert_eq!(flag.trim(), "1");
}

/// Activity with monitor-activity off (default): early return.
/// Covers: alerts_check_activity monitor-activity==0 branch.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_activity_monitor_off() {
    let tmux = TmuxServer::new("alerts_activity_off");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    // monitor-activity is off by default
    tmux.run(&["new-window"]);

    tmux.run(&["send-keys", "-t:0", "echo hello", "Enter"]);
    sleep_ms(500);

    let flag = tmux.run(&["display-message", "-t:0", "-p", "#{window_activity_flag}"]);
    assert_eq!(flag.trim(), "0");
}

/// Activity-action variants: current and other.
/// Covers: alerts_action_applies ALERT_CURRENT and ALERT_OTHER for activity.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_activity_action_variants() {
    // activity-action=current: applies only for current window
    {
        let tmux = TmuxServer::new("alerts_aa_cur");
        tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
        tmux.run(&["set", "-g", "window-size", "manual"]);
        tmux.run(&["set", "-g", "monitor-activity", "on"]);
        tmux.run(&["set", "-g", "activity-action", "current"]);
        tmux.run(&["new-window"]);

        // Activity in non-current window (wl != curw) -> action does NOT apply
        tmux.run(&["send-keys", "-t:0", "echo activity_noncur", "Enter"]);
        sleep_ms(500);

        // Activity in current window (wl == curw) -> action applies
        tmux.run(&["send-keys", "-t:1", "echo activity_cur", "Enter"]);
        sleep_ms(500);
    }

    // activity-action=none: no notification for any window
    {
        let tmux = TmuxServer::new("alerts_aa_none");
        tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
        tmux.run(&["set", "-g", "window-size", "manual"]);
        tmux.run(&["set", "-g", "monitor-activity", "on"]);
        tmux.run(&["set", "-g", "activity-action", "none"]);
        tmux.run(&["new-window"]);

        tmux.run(&["send-keys", "-t:0", "echo activity_none", "Enter"]);
        sleep_ms(500);
    }
}

/// Silence alert: set short monitor-silence, wait for timer.
/// Covers: alerts_check_silence main path, alerts_enabled(SILENCE),
/// alerts_reset timer setup, alerts_timer callback, silence-action=other (default).
/// Also covers WINLINK_SILENCE "already set" continue branch (timer fires again).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_silence() {
    let tmux = TmuxServer::new("alerts_silence");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "-g", "monitor-silence", "1"]);
    tmux.run(&["new-window"]);

    // Wait for silence timer on window 0 (1 sec silence + buffer)
    // Wait extra to also cover the "WINLINK_SILENCE already set" branch
    // when the timer fires a second time.
    sleep_secs(3);

    let flag = tmux.run(&["display-message", "-t:0", "-p", "#{window_silence_flag}"]);
    assert_eq!(flag.trim(), "1");
}

/// Silence with monitor-silence off (default): no flag set.
/// Covers: alerts_check_silence early return (monitor-silence==0).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_silence_monitor_off() {
    let tmux = TmuxServer::new("alerts_silence_off");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    // monitor-silence is 0 by default
    tmux.run(&["new-window"]);

    sleep_secs(2);

    let flag = tmux.run(&["display-message", "-t:0", "-p", "#{window_silence_flag}"]);
    assert_eq!(flag.trim(), "0");
}

/// SESSION_ALERTED dedup: link a window to two indexes so it has two winlinks
/// in the same session. Bell in that window processes the first winlink fully,
/// then SESSION_ALERTED causes the second winlink to skip notify.
/// Covers: alerts_check_bell SESSION_ALERTED branch (continue).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_session_alerted_dedup() {
    let tmux = TmuxServer::new("alerts_dedup");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Link window 0 to index 2 (same window, two winlinks in session)
    tmux.run(&["link-window", "-s", ":0", "-t", ":2"]);

    // Create a separate window to be current
    tmux.run(&["new-window", "-t", ":3"]);
    tmux.run(&["select-window", "-t", ":3"]);

    // Trigger bell in the linked window
    tmux.run(&["send-keys", "-t:0", "printf '\\a'", "Enter"]);
    sleep_ms(500);

    // Both winlinks should have WINLINK_BELL set
    let flag0 = tmux.run(&["display-message", "-t:0", "-p", "#{window_bell_flag}"]);
    let flag2 = tmux.run(&["display-message", "-t:2", "-p", "#{window_bell_flag}"]);
    assert_eq!(flag0.trim(), "1");
    assert_eq!(flag2.trim(), "1");
}

/// Cross-alert-type queueing: monitor-bell off but monitor-activity on.
/// Bell sets BELL flag but doesn't queue. Activity queues the window.
/// In alerts_callback, alerts_check_bell sees BELL flag with monitor-bell==0
/// and takes the early return at line 174.
/// Covers: alerts_check_bell monitor-bell==0 early return when BELL flag IS set.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_cross_alert_bell_disabled() {
    let tmux = TmuxServer::new("alerts_cross_bell");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "-g", "monitor-bell", "off"]);
    tmux.run(&["set", "-g", "monitor-activity", "on"]);
    tmux.run(&["new-window"]);

    // Trigger both bell and activity in window 0 (non-current).
    // Bell sets BELL flag (via alerts_queue), but isn't queued (monitor-bell off).
    // Activity sets ACTIVITY flag and queues the window (monitor-activity on).
    // When alerts_callback runs, alerts_check_bell sees BELL+monitor-bell==0 -> early return.
    tmux.run(&["send-keys", "-t:0", "printf '\\a' && echo done", "Enter"]);
    sleep_ms(500);

    // Activity flag should be set (queued via ACTIVITY)
    let aflag = tmux.run(&["display-message", "-t:0", "-p", "#{window_activity_flag}"]);
    assert_eq!(aflag.trim(), "1");

    // Bell flag should NOT be set (monitor-bell off, alerts_check_bell returned early)
    let bflag = tmux.run(&["display-message", "-t:0", "-p", "#{window_bell_flag}"]);
    assert_eq!(bflag.trim(), "0");
}

/// SESSION_ALERTED dedup for activity: linked window with two winlinks.
/// Covers: alerts_check_activity SESSION_ALERTED branch (line 235).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_activity_session_alerted() {
    let tmux = TmuxServer::new("alerts_act_dedup");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "-g", "monitor-activity", "on"]);

    // Link window 0 to index 2 (two winlinks for same window)
    tmux.run(&["link-window", "-s", ":0", "-t", ":2"]);

    // Make a different window current
    tmux.run(&["new-window", "-t", ":3"]);
    tmux.run(&["select-window", "-t", ":3"]);

    // Trigger activity in the linked window
    tmux.run(&["send-keys", "-t:0", "echo activity_dedup", "Enter"]);
    sleep_ms(500);

    // Both winlinks should have WINLINK_ACTIVITY
    let flag0 = tmux.run(&["display-message", "-t:0", "-p", "#{window_activity_flag}"]);
    let flag2 = tmux.run(&["display-message", "-t:2", "-p", "#{window_activity_flag}"]);
    assert_eq!(flag0.trim(), "1");
    assert_eq!(flag2.trim(), "1");
}

/// SESSION_ALERTED dedup for silence: linked window with two winlinks.
/// Covers: alerts_check_silence SESSION_ALERTED branch (line 277).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_silence_session_alerted() {
    let tmux = TmuxServer::new("alerts_sil_dedup");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "-g", "monitor-silence", "1"]);

    // Link window 0 to index 2 (two winlinks for same window)
    tmux.run(&["link-window", "-s", ":0", "-t", ":2"]);

    // Make a different window current
    tmux.run(&["new-window", "-t", ":3"]);
    tmux.run(&["select-window", "-t", ":3"]);

    // Wait for silence timer
    sleep_secs(3);

    // Both winlinks should have WINLINK_SILENCE
    let flag0 = tmux.run(&["display-message", "-t:0", "-p", "#{window_silence_flag}"]);
    let flag2 = tmux.run(&["display-message", "-t:2", "-p", "#{window_silence_flag}"]);
    assert_eq!(flag0.trim(), "1");
    assert_eq!(flag2.trim(), "1");
}

/// Control mode client: exercises alerts processing with an attached client.
/// Control mode clients have the CONTROL flag, so they hit the "continue" branch
/// at line 295 in alerts_set_message.
/// Uses respawn-pane to produce BEL in the current window while attached,
/// covering the curw==wl && attached!=0 branch (lines 187, 226).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_with_control_client() {
    let tmux = TmuxServer::new("alerts_ctrl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "-g", "monitor-activity", "on"]);
    tmux.run(&["set", "-g", "visual-bell", "on"]);
    tmux.run(&["set", "-g", "visual-activity", "on"]);
    tmux.run(&["new-window"]);

    // Control client attaches (session.attached > 0), then:
    // 1. Triggers bell+activity in window 0 (non-current) via send-keys
    // 2. Respawns pane in window 1 (current/curw) with a command that outputs BEL,
    //    covering curw==wl && attached!=0 branch where WINLINK flag is NOT set
    // 3. Waits for processing, then detaches
    let ctrl_cmds = b"send-keys -t:0 \"printf '\\\\a' && echo noncur_activity\" Enter\n\
                       respawn-pane -t:1 -k \"printf '\\\\a' && echo cur_activity && exec sleep 10\"\n\
                       run-shell \"sleep 1\"\n\
                       detach\n";
    let _output = tmux.run_with_stdin(&["-C", "attach"], ctrl_cmds);

    sleep_ms(500);

    // Non-current window (0): bell flag set (curw != wl, always set when detached or not)
    let bflag0 = tmux.run(&["display-message", "-t:0", "-p", "#{window_bell_flag}"]);
    assert_eq!(bflag0.trim(), "1");

    // Current window (1) while attached: curw==wl && attached!=0 -> flag NOT set
    // After detach the session is detached again, but alerts were processed while attached.
    // The bell in window 1 was processed while the control client was attached.
}

/// Silence alert on current window while attached via control mode.
/// Uses wait-for to keep control client attached while silence timer fires.
/// Covers: alerts_check_silence curw==wl && attached!=0 branch (line 268).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn alerts_silence_current_attached() {
    let tmux = TmuxServer::new("alerts_sil_att");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["set", "-g", "monitor-silence", "1"]);
    tmux.run(&["new-window"]);

    // Wait for shell prompts to settle so silence timer starts fresh
    sleep_ms(500);

    // Spawn control mode client in background thread.
    // It blocks on wait-for until we signal it from the main thread.
    let binary = TmuxServer::binary_path().to_string();
    let socket = tmux.socket().to_string();
    let handle = std::thread::spawn(move || {
        let mut cmd = std::process::Command::new(&binary);
        cmd.arg("-L").arg(&socket).args(&["-C", "attach"]);
        cmd.env("PATH", "/bin:/usr/bin:/usr/local/bin");
        cmd.env("TERM", "screen");
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn().expect("spawn control client");
        let stdin = child.stdin.as_mut().unwrap();
        // Block on wait-for (keeps client attached)
        stdin
            .write_all(b"wait-for detach_signal\ndetach\n")
            .unwrap();
        child.wait_with_output().expect("wait for control client")
    });

    // Wait for silence timer to fire (1s) on both windows while attached
    sleep_secs(2);

    // Signal the control client to detach
    tmux.run(&["wait-for", "-S", "detach_signal"]);
    let _output = handle.join().unwrap();

    sleep_ms(200);

    // Window 0 (non-current during attach) should have silence flag
    let flag0 = tmux.run(&["display-message", "-t:0", "-p", "#{window_silence_flag}"]);
    assert_eq!(flag0.trim(), "1");
}
