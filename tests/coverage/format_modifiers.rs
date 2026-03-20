use super::*;

/// Test format modifiers via display-message -p to exercise format.rs paths.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_basename() {
    let tmux = TmuxServer::new("fmt_basename");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // b: basename modifier
    let out = tmux.display("#{b:pane_current_path}");
    // Just exercise the path — value depends on shell cwd
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_dirname() {
    let tmux = TmuxServer::new("fmt_dirname");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // d: dirname modifier
    let out = tmux.display("#{d:pane_current_path}");
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_length() {
    let tmux = TmuxServer::new("fmt_length");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // n: string length
    let out = tmux.display("#{n:session_name}");
    let name = tmux.display("#{session_name}");
    assert_eq!(out, name.len().to_string(), "length should match session name length");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_equality() {
    let tmux = TmuxServer::new("fmt_eq");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // ==: equality comparison
    let out = tmux.display("#{==:abc,abc}");
    assert_eq!(out, "1");
    let out = tmux.display("#{==:abc,def}");
    assert_eq!(out, "0");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_not_equal() {
    let tmux = TmuxServer::new("fmt_ne");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.display("#{!=:abc,abc}");
    assert_eq!(out, "0");
    let out = tmux.display("#{!=:abc,def}");
    assert_eq!(out, "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_conditional() {
    let tmux = TmuxServer::new("fmt_cond");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // ?: conditional
    let out = tmux.display("#{?pane_active,yes,no}");
    assert_eq!(out, "yes");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_arithmetic() {
    let tmux = TmuxServer::new("fmt_arith");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // e|+: addition
    let out = tmux.display("#{e|+:3,4}");
    assert_eq!(out, "7");

    // e|-: subtraction
    let out = tmux.display("#{e|-:10,3}");
    assert_eq!(out, "7");

    // e|*: multiplication
    let out = tmux.display("#{e|*:3,4}");
    assert_eq!(out, "12");

    // e|/: division
    let out = tmux.display("#{e|/:12,4}");
    assert_eq!(out, "3");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_comparison_operators() {
    let tmux = TmuxServer::new("fmt_cmp");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // <, >, <=, >=
    let out = tmux.display("#{e|<:3,5}");
    assert_eq!(out, "1");
    let out = tmux.display("#{e|>:3,5}");
    assert_eq!(out, "0");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_string_operations() {
    let tmux = TmuxServer::new("fmt_strop");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // l: lowercase — exercises the modifier path
    let out = tmux.display("#{l:session_name}");
    let _ = out;

    // t: timestamp format
    let out = tmux.display("#{t:start_time}");
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_match() {
    let tmux = TmuxServer::new("fmt_match");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // m: glob match
    let out = tmux.display("#{m:fmt_*,fmt_match}");
    assert_eq!(out, "1");
    let out = tmux.display("#{m:other_*,fmt_match}");
    assert_eq!(out, "0");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_regex_match() {
    let tmux = TmuxServer::new("fmt_regex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // m/r: regex match
    let out = tmux.display("#{m/r:^fmt.*,fmt_regex}");
    assert_eq!(out, "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_substitute() {
    let tmux = TmuxServer::new("fmt_sub");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // s/from/to: substitution — exercises the modifier parser
    tmux.run(&["set", "-g", "@subval", "old_value"]);
    let out = tmux.display("#{s/old/new:#{@subval}}");
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_loop_windows() {
    let tmux = TmuxServer::new("fmt_loop");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["new-window"]);

    // W: loop over windows
    let out = tmux.display("#{W:#{window_index} }");
    assert!(out.contains("0") && out.contains("1"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_loop_panes() {
    let tmux = TmuxServer::new("fmt_panes");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);
    tmux.run(&["split-window", "-d"]);

    // P: loop over panes
    let out = tmux.display("#{P:#{pane_index} }");
    assert!(out.contains("0") && out.contains("1"), "got: {out}");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_colour_modifier() {
    let tmux = TmuxServer::new("fmt_colour");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // c: colour modifier (converts colour name to hex or similar)
    let out = tmux.display("#{c:red}");
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn format_width() {
    let tmux = TmuxServer::new("fmt_width");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // w: display width — exercises the modifier path
    let out = tmux.display("#{w:session_name}");
    let _ = out;
}
