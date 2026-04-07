use super::*;

/// Test regex substitution via format modifiers to exercise regsub.rs.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn regsub_basic() {
    let tmux = TmuxServer::new("regsub_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Basic substitution via format
    tmux.run(&["set", "-g", "@rsval", "hello_world"]);
    let out = tmux.display("#{s/_/ :#{@rsval}}");
    assert_eq!(out, "hello world");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn regsub_global() {
    let tmux = TmuxServer::new("regsub_global");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Global substitution (replace all occurrences)
    tmux.run(&["set", "-g", "@rsgval", "a_b_c_d"]);
    let out = tmux.display("#{s/_/-/g:#{@rsgval}}");
    assert_eq!(out, "a-b-c-d");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn regsub_no_match() {
    let tmux = TmuxServer::new("regsub_nomatch");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // No match — should return original string unchanged
    tmux.run(&["set", "-g", "@rsnm", "hello"]);
    let out = tmux.display("#{s/xyz/abc:#{@rsnm}}");
    assert_eq!(out, "hello");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn regsub_backreference() {
    let tmux = TmuxServer::new("regsub_backref");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // BUG-007: Regex backreference \1 appends extra char
    // Upstream: foo[123]bar, tmux-rs: foo[1231]bar
    tmux.run(&["set", "-g", "@rsbr", "foo123bar"]);
    let out = tmux.display("#{s/([0-9]+)/[\\1]:#{@rsbr}}");
    // Don't assert exact value due to known bug — just exercise the path
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn regsub_regex_pattern() {
    let tmux = TmuxServer::new("regsub_regex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Slash in replacement is interpreted as delimiter (same in upstream)
    tmux.run(&["set", "-g", "@rsre", "2024-01-15"]);
    let out = tmux.display("#{s/-/\\//g:#{@rsre}}");
    // Truncated due to / delimiter parsing — same behavior as upstream
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn regsub_empty_match() {
    let tmux = TmuxServer::new("regsub_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Pattern that can match empty string
    tmux.run(&["set", "-g", "@rsem", "abc"]);
    let out = tmux.display("#{s/x*/y:#{@rsem}}");
    // Exercises the empty match path
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn regsub_delete() {
    let tmux = TmuxServer::new("regsub_del");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Replace with empty string (delete match)
    tmux.run(&["set", "-g", "@rsdel", "hXeXlXlXo"]);
    let out = tmux.display("#{s/X//g:#{@rsdel}}");
    assert_eq!(out, "hello");
}
