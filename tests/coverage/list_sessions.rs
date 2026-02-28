use super::*;

/// Test list-sessions with -f filter (coverage: cmd_list_sessions.rs lines 55-66)
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn list_sessions_filter() {
    let tmux = TmuxServer::new("list_sessions_filter");

    let conf = tmux.write_temp("new -sfoo\nnew -sbar\nnew -sbaz\n");
    let f_flag = format!("-f{}", conf.path_str());
    tmux.run(&[&f_flag, "start"]);

    // Filter that matches only sessions starting with "ba"
    let output = tmux.run(&[
        "ls",
        "-f",
        "#{m:ba*,#{session_name}}",
        "-F",
        "#{session_name}",
    ]);
    let mut sessions: Vec<&str> = output.lines().collect();
    sessions.sort();
    assert_eq!(sessions, vec!["bar", "baz"]);

    // Filter that matches nothing
    let output = tmux.run(&[
        "ls",
        "-f",
        "#{m:zzz*,#{session_name}}",
        "-F",
        "#{session_name}",
    ]);
    assert_eq!(
        output.trim(),
        "",
        "filter matching nothing should produce no output"
    );
}
