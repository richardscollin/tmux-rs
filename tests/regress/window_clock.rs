use super::*;

/// Test clock-mode rendering across all styles, screen sizes, resize, and colour.
/// Covers: window_clock_init, window_clock_draw_screen (all 4 styles, bitmap path,
///         small-screen text fallback, wide-short fallback, too-narrow early return,
///         AM/PM, digit/colon/character indexing, colour option),
///         window_clock_resize, window_clock_free.
#[test]
fn clock_mode_draw() {
    // --- Style 0 (12h) with resize through all screen-size branches ---
    {
        let tmux = TmuxServer::new("clock_mode_draw_12h");
        tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
        tmux.run(&["set", "-g", "window-size", "manual"]);
        tmux.run(&["set", "clock-mode-colour", "red"]);
        tmux.run(&["set", "clock-mode-style", "12"]);
        tmux.run(&["clock-mode"]);
        assert_eq!(tmux.display("#{pane_mode}"), "clock-mode");

        // Wide but short: screen_size_y < 6 branch (text fallback via height)
        tmux.run(&["resize-window", "-x", "80", "-y", "4"]);
        assert_eq!(tmux.display("#{pane_mode}"), "clock-mode");

        // Small: screen_size_x < 6*tim_len branch (text fallback via width)
        tmux.run(&["resize-window", "-x", "20", "-y", "5"]);
        assert_eq!(tmux.display("#{pane_mode}"), "clock-mode");

        // Tiny: screen_size_x < tim_len branch (no rendering, early return)
        tmux.run(&["resize-window", "-x", "3", "-y", "3"]);
        assert_eq!(tmux.display("#{pane_mode}"), "clock-mode");

        // Grow back to large -> bitmap path again
        tmux.run(&["resize-window", "-x", "80", "-y", "24"]);
        assert_eq!(tmux.display("#{pane_mode}"), "clock-mode");
    }

    // --- Style 1 (24h) ---
    {
        let tmux = TmuxServer::new("clock_mode_draw_24h");
        tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
        tmux.run(&["set", "-g", "window-size", "manual"]);
        tmux.run(&["set", "clock-mode-style", "24"]);
        tmux.run(&["clock-mode"]);
        assert_eq!(tmux.display("#{pane_mode}"), "clock-mode");
    }

    // --- Style 2 (12h with seconds) ---
    {
        let tmux = TmuxServer::new("clock_mode_draw_12s");
        tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
        tmux.run(&["set", "-g", "window-size", "manual"]);
        tmux.run(&["set", "clock-mode-style", "12-with-seconds"]);
        tmux.run(&["clock-mode"]);
        assert_eq!(tmux.display("#{pane_mode}"), "clock-mode");
    }

    // --- Style 3 (24h with seconds) ---
    {
        let tmux = TmuxServer::new("clock_mode_draw_24s");
        tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
        tmux.run(&["set", "-g", "window-size", "manual"]);
        tmux.run(&["set", "clock-mode-style", "24-with-seconds"]);
        tmux.run(&["clock-mode"]);
        assert_eq!(tmux.display("#{pane_mode}"), "clock-mode");
    }
}

/// Test that any key exits clock mode.
/// Covers: window_clock_key (calls window_pane_reset_mode).
/// Uses a control-mode client since window_pane_key requires a non-null client.
#[test]
fn clock_mode_exit_on_key() {
    let tmux = TmuxServer::new("clock_mode_exit_key");

    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["clock-mode"]);
    assert_eq!(tmux.display("#{pane_mode}"), "clock-mode");

    // send-keys requires a non-null client; use a control-mode client
    tmux.run_with_stdin(&["-C", "attach"], b"send-keys q\n");

    assert_eq!(
        tmux.display("#{pane_mode}"),
        "",
        "should have exited clock-mode"
    );
}

/// Test that the timer callback fires and re-renders the clock.
/// Covers: window_clock_timer_callback (evtimer_del, evtimer_add, time comparison,
///         window_clock_draw_screen re-invocation).
/// Uses run-shell to keep the server alive for 2s while the 1s timer fires.
#[test]
fn clock_mode_timer() {
    let tmux = TmuxServer::new("clock_mode_timer");

    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["clock-mode"]);

    // Keep server alive for 2s so the 1s timer callback fires
    tmux.run(&["run-shell", "sleep 2"]);

    assert_eq!(tmux.display("#{pane_mode}"), "clock-mode");
}

/// neww while in clock-mode crashes with null pointer dereference in
/// clients_calculate_size (resize.rs:145). Upstream tmux also crashes.
#[test]
fn clock_mode_neww_crash() {
    let tmux = TmuxServer::new("clock_mode_neww_crash");

    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["clock-mode"]);
    tmux.run(&["neww"]); // crashes the server
}
