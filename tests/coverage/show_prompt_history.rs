use super::*;

/// Show prompt history (all types).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_prompt_history_all() {
    let tmux = TmuxServer::new("showphist_all");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // With no history, should return empty or headers
    let out = tmux.run(&["showphist"]);
    let _ = out; // Just verify no crash
}

/// Show prompt history for a specific type.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_prompt_history_command() {
    let tmux = TmuxServer::new("showphist_cmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["showphist", "-T", "command"]);
    let _ = out;
}

/// Clear all prompt history.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_prompt_history_all() {
    let tmux = TmuxServer::new("clearphist_all");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["clearphist"]);
    // Just verify clearphist runs without error
}

/// Clear prompt history for specific type.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_prompt_history_type() {
    let tmux = TmuxServer::new("clearphist_type");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["clearphist", "-T", "search"]);
    // Just verify no crash
}

/// Invalid prompt type.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_prompt_history_invalid_type() {
    let tmux = TmuxServer::new("showphist_invalid");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["showphist", "-T", "bogus"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("invalid type"),
        "expected 'invalid type' error, got: {stderr}"
    );
}

/// Show prompt history for search type.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_prompt_history_search() {
    let tmux = TmuxServer::new("showphist_search");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["showphist", "-T", "search"]);
    assert!(
        out.contains("search"),
        "expected header mentioning 'search', got: {out}"
    );
}

/// Show prompt history for target type.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_prompt_history_target() {
    let tmux = TmuxServer::new("showphist_target");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["showphist", "-T", "target"]);
    assert!(
        out.contains("target"),
        "expected header mentioning 'target', got: {out}"
    );
}

/// Show prompt history for window-target type.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_prompt_history_window_target() {
    let tmux = TmuxServer::new("showphist_wintgt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["showphist", "-T", "window-target"]);
    assert!(
        out.contains("window-target"),
        "expected header mentioning 'window-target', got: {out}"
    );
}

/// Show all prompt history types (default, no -T flag).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn show_prompt_history_all_types_header() {
    let tmux = TmuxServer::new("showphist_allhdr");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.run(&["showphist"]);
    // Default output should show headers for all 4 types
    assert!(
        out.contains("command"),
        "expected 'command' header in: {out}"
    );
    assert!(out.contains("search"), "expected 'search' header in: {out}");
    assert!(out.contains("target"), "expected 'target' header in: {out}");
    assert!(
        out.contains("window-target"),
        "expected 'window-target' header in: {out}"
    );
}

/// Clear prompt history for command type.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_prompt_history_command() {
    let tmux = TmuxServer::new("clearphist_cmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["clearphist", "-T", "command"]);
    let out = tmux.run(&["showphist", "-T", "command"]);
    // After clearing, command history should be empty (just a header)
    let lines: Vec<&str> = out.trim().lines().collect();
    // Header line + possibly an empty line, but no numbered entries
    for line in &lines {
        assert!(
            !line.starts_with("1:"),
            "expected no history entries after clear, got: {line}"
        );
    }
}

/// Clear prompt history for target type.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_prompt_history_target() {
    let tmux = TmuxServer::new("clearphist_tgt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["clearphist", "-T", "target"]);
    // Should succeed without error
}

/// Clear prompt history for window-target type.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_prompt_history_window_target() {
    let tmux = TmuxServer::new("clearphist_wt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    tmux.run(&["clearphist", "-T", "window-target"]);
    // Should succeed without error
}

/// Clear prompt history with invalid type (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_prompt_history_invalid_type() {
    let tmux = TmuxServer::new("clearphist_inv");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["clearphist", "-T", "invalid"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("invalid type"),
        "expected 'invalid type' error, got: {stderr}"
    );
}

/// Clear all prompt history then show (should be empty).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn clear_then_show_prompt_history() {
    let tmux = TmuxServer::new("clearshow_phist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Clear all types
    tmux.run(&["clearphist"]);
    // Show all types - should have only headers, no numbered entries
    let out = tmux.run(&["showphist"]);
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("History for") {
            continue;
        }
        // No numbered entries like "1: something"
        assert!(
            !trimmed.chars().next().unwrap_or(' ').is_ascii_digit(),
            "expected no history entries after clear-all, got line: {trimmed}"
        );
    }
}
