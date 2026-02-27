use super::*;

/// Test swap-pane -D (swap with next pane) (coverage: cmd_swap_pane.rs lines 55-60)
#[test]
fn swap_pane_down() {
    let tmux = TmuxServer::new("swap_pane_down");

    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    // Split to create a second pane
    tmux.run(&["splitw", "-d"]);
    sleep_ms(500);

    // Mark the first pane so we can track it
    tmux.run(&["selectp", "-t", "0"]);
    tmux.run(&["send-keys", "export PANE_MARK=first", "Enter"]);
    tmux.run(&["selectp", "-t", "1"]);
    tmux.run(&["send-keys", "export PANE_MARK=second", "Enter"]);
    sleep_ms(500);

    // Get pane IDs before swap
    let before = tmux.run(&["lsp", "-F", "#{pane_index} #{pane_id}"]);

    // Swap pane 0 down (with -D, target becomes source, next pane is new source)
    tmux.run(&["swapp", "-D", "-t", "0"]);

    // Get pane IDs after swap
    let after = tmux.run(&["lsp", "-F", "#{pane_index} #{pane_id}"]);

    assert_ne!(before, after, "pane order should change after swap-pane -D");
}

/// Test swap-pane -U (swap with previous pane) (coverage: cmd_swap_pane.rs lines 62-66)
#[test]
fn swap_pane_up() {
    let tmux = TmuxServer::new("swap_pane_up");

    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["splitw", "-d"]);
    sleep_ms(500);

    let before = tmux.run(&["lsp", "-F", "#{pane_index} #{pane_id}"]);

    // Swap pane 1 up
    tmux.run(&["swapp", "-U", "-t", "1"]);

    let after = tmux.run(&["lsp", "-F", "#{pane_index} #{pane_id}"]);

    assert_ne!(before, after, "pane order should change after swap-pane -U");
}

/// Test swap-pane -d (don't change active pane) (coverage: cmd_swap_pane.rs lines 128-133)
#[test]
fn swap_pane_no_focus() {
    let tmux = TmuxServer::new("swap_pane_nofocus");

    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-d"]);
    sleep_ms(500);

    // Get pane IDs before swap
    let before = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    let ids_before: Vec<&str> = before.lines().collect();

    // Swap panes 0 and 2 with -d (exercises the -d code path)
    tmux.run(&["swapp", "-d", "-s", "0", "-t", "2"]);

    let after = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    let ids_after: Vec<&str> = after.lines().collect();

    // Verify the swap happened
    assert_eq!(
        ids_before[0], ids_after[2],
        "pane 0 should now be at position 2"
    );
    assert_eq!(
        ids_before[2], ids_after[0],
        "pane 2 should now be at position 0"
    );
    assert_eq!(ids_before[1], ids_after[1], "pane 1 should be unchanged");
}

/// Test swap-pane within same window (coverage: cmd_swap_pane.rs lines 124-125)
#[test]
fn swap_pane_same_window() {
    let tmux = TmuxServer::new("swap_pane_same");

    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["splitw", "-d"]);
    tmux.run(&["splitw", "-d"]);
    sleep_ms(500);

    // Get pane ids before
    let before = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    let ids_before: Vec<&str> = before.lines().collect();

    // Swap first and last pane within the same window
    tmux.run(&["swapp", "-s", "0", "-t", "2"]);

    let after = tmux.run(&["lsp", "-F", "#{pane_id}"]);
    let ids_after: Vec<&str> = after.lines().collect();

    assert_eq!(ids_before.len(), ids_after.len());
    // First and last should have swapped
    assert_eq!(ids_before[0], ids_after[2]);
    assert_eq!(ids_before[2], ids_after[0]);
    // Middle should be unchanged
    assert_eq!(ids_before[1], ids_after[1]);
}
