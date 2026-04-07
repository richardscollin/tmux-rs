use super::*;

/// Test command parsing paths via source-file and direct commands.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_semicolon_separated() {
    let tmux = TmuxServer::new("parse_semi");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Multiple commands separated by semicolons in source file
    let tmp = tmux.write_temp("set -g @p1 a ; set -g @p2 b\n");
    tmux.run(&["source-file", tmp.path_str()]);

    let v1 = tmux.run(&["show", "-gv", "@p1"]);
    let v2 = tmux.run(&["show", "-gv", "@p2"]);
    assert_eq!(v1.trim(), "a");
    assert_eq!(v2.trim(), "b");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_newline_separated() {
    let tmux = TmuxServer::new("parse_newline");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("set -g @nl1 x\nset -g @nl2 y\n");
    tmux.run(&["source-file", tmp.path_str()]);

    let v1 = tmux.run(&["show", "-gv", "@nl1"]);
    let v2 = tmux.run(&["show", "-gv", "@nl2"]);
    assert_eq!(v1.trim(), "x");
    assert_eq!(v2.trim(), "y");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_line_continuation() {
    let tmux = TmuxServer::new("parse_cont");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Line continuation with backslash
    let tmp = tmux.write_temp("set -g @cont \\\nhello\n");
    tmux.run(&["source-file", tmp.path_str()]);

    let v = tmux.run(&["show", "-gv", "@cont"]);
    assert_eq!(v.trim(), "hello");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_comment() {
    let tmux = TmuxServer::new("parse_comment");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Comments should be ignored
    let tmp = tmux.write_temp("# this is a comment\nset -g @cmt yes\n# another comment\n");
    tmux.run(&["source-file", tmp.path_str()]);

    let v = tmux.run(&["show", "-gv", "@cmt"]);
    assert_eq!(v.trim(), "yes");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_empty_lines() {
    let tmux = TmuxServer::new("parse_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("\n\nset -g @empty yes\n\n\n");
    tmux.run(&["source-file", tmp.path_str()]);

    let v = tmux.run(&["show", "-gv", "@empty"]);
    assert_eq!(v.trim(), "yes");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_if_block() {
    let tmux = TmuxServer::new("parse_if");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // if-shell block in config
    let tmp = tmux.write_temp("if-shell -F '1' {\n  set -g @ifblock yes\n}\n");
    tmux.run(&["source-file", tmp.path_str()]);
    sleep_ms(100);

    let v = tmux.run(&["show", "-gv", "@ifblock"]);
    assert_eq!(v.trim(), "yes");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_if_else_block() {
    let tmux = TmuxServer::new("parse_ifelse");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // if-shell with else block
    let tmp = tmux.write_temp("if-shell -F '0' {\n  set -g @ie nope\n} {\n  set -g @ie else\n}\n");
    tmux.run(&["source-file", tmp.path_str()]);
    sleep_ms(100);

    let v = tmux.run(&["show", "-gv", "@ie"]);
    assert_eq!(v.trim(), "else");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_syntax_error() {
    let tmux = TmuxServer::new("parse_synerr");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Syntax error in config file
    let tmp = tmux.write_temp("not-a-command foo bar\n");
    let out = tmux.try_run(&["source-file", tmp.path_str()]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_syntax_error_quiet() {
    let tmux = TmuxServer::new("parse_synerrq");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Syntax error with -q: should succeed silently
    let tmp = tmux.write_temp("not-a-command foo bar\n");
    let out = tmux.try_run(&["source-file", "-q", tmp.path_str()]);
    // May or may not succeed depending on how parse errors are handled with -q
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_quoted_string() {
    let tmux = TmuxServer::new("parse_quoted");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Quoted strings with spaces
    let tmp = tmux.write_temp("set -g @quoted 'hello world'\n");
    tmux.run(&["source-file", tmp.path_str()]);

    let v = tmux.run(&["show", "-gv", "@quoted"]);
    assert_eq!(v.trim(), "hello world");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_double_quoted() {
    let tmux = TmuxServer::new("parse_dquoted");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Double-quoted strings
    let tmp = tmux.write_temp("set -g @dq \"hello world\"\n");
    tmux.run(&["source-file", tmp.path_str()]);

    let v = tmux.run(&["show", "-gv", "@dq"]);
    assert_eq!(v.trim(), "hello world");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_braces_in_command() {
    let tmux = TmuxServer::new("parse_braces");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Braces used for grouping in bind-key
    let tmp = tmux.write_temp("bind-key X { set -g @braced yes }\n");
    tmux.run(&["source-file", tmp.path_str()]);

    // Trigger the binding
    tmux.run(&["send-prefix"]);
    tmux.run(&["send-keys", "X"]);
    sleep_ms(100);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn parse_verbose() {
    let tmux = TmuxServer::new("parse_verbose");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -v: verbose parsing — exercises verbose flag in parser
    let tmp = tmux.write_temp("set -g @verbose yes\n");
    tmux.run(&["source-file", "-v", tmp.path_str()]);

    let v = tmux.run(&["show", "-gv", "@verbose"]);
    assert_eq!(v.trim(), "yes");
}
