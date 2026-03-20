use super::*;

/// Test command dispatch error paths in cmd_/mod.rs.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn cmd_ambiguous() {
    let tmux = TmuxServer::new("cmd_ambig");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Ambiguous command prefix — "se" matches set, set-buffer, etc.
    let out = tmux.try_run(&["se"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("ambiguous") || stderr.contains("unknown command"),
        "expected ambiguous error, got: {stderr}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn cmd_unknown() {
    let tmux = TmuxServer::new("cmd_unknown");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Completely unknown command
    let out = tmux.try_run(&["zzz-not-a-command"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn cmd_no_command() {
    let tmux = TmuxServer::new("cmd_nocmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Empty command via source-file
    let tmp = tmux.write_temp(";\n");
    let out = tmux.try_run(&["source-file", tmp.path_str()]);
    let _ = out;
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn cmd_wrong_usage() {
    let tmux = TmuxServer::new("cmd_usage");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Command with wrong flags
    let out = tmux.try_run(&["kill-server", "-Z"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("usage") || stderr.contains("unknown flag"),
        "expected usage/flag error, got: {stderr}"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn cmd_alias() {
    let tmux = TmuxServer::new("cmd_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Use aliases
    tmux.run(&["neww", "-d"]);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count, "2");

    tmux.run(&["killw", "-t", ":1"]);
    let count = tmux.display("#{session_windows}");
    assert_eq!(count, "1");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn cmd_template_replace() {
    let tmux = TmuxServer::new("cmd_tmpl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // confirm-before uses template replacement (%%/etc.)
    // Just exercise the path
    tmux.run(&["set", "-g", "@tmpl", "test%%value"]);
    let out = tmux.run(&["show", "-gv", "@tmpl"]);
    assert_eq!(out.trim(), "test%%value");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn cmd_list_commands() {
    let tmux = TmuxServer::new("cmd_listcmd");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // list-commands exercises command table iteration
    let out = tmux.run(&["list-commands"]);
    assert!(out.contains("set-option"));
    assert!(out.contains("new-window"));
    assert!(out.contains("send-keys"));
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn cmd_list_commands_format() {
    let tmux = TmuxServer::new("cmd_listcmdf");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // list-commands -F with custom format
    let out = tmux.run(&["list-commands", "-F", "#{command_list_name}"]);
    assert!(out.contains("set-option"), "got: {}", &out[..out.len().min(200)]);
}
