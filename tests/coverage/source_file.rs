use super::*;

/// Test source-file: basic source, -q (quiet), -v (verbose),
/// nonexistent file, multiple files, -F (format), nested limit.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_basic() {
    let tmux = TmuxServer::new("srcfile_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("set -g @srctest yes\n");
    tmux.run(&["source-file", tmp.path_str()]);

    let val = tmux.run(&["show-options", "-gv", "@srctest"]);
    assert_eq!(val.trim(), "yes");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_multiple_commands() {
    let tmux = TmuxServer::new("srcfile_multi");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("set -g @src1 a\nset -g @src2 b\n");
    tmux.run(&["source-file", tmp.path_str()]);

    let v1 = tmux.run(&["show-options", "-gv", "@src1"]);
    let v2 = tmux.run(&["show-options", "-gv", "@src2"]);
    assert_eq!(v1.trim(), "a");
    assert_eq!(v2.trim(), "b");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_nonexistent() {
    let tmux = TmuxServer::new("srcfile_noexist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let out = tmux.try_run(&["source-file", "/nonexistent/path/file.conf"]);
    assert!(!out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_quiet() {
    let tmux = TmuxServer::new("srcfile_quiet");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // -q: quiet mode — nonexistent file should not error
    let out = tmux.try_run(&["source-file", "-q", "/nonexistent/path/file.conf"]);
    assert!(
        out.status.success(),
        "source-file -q should succeed on missing file"
    );
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_verbose() {
    let tmux = TmuxServer::new("srcfile_verbose");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("set -g @srcverb yes\n");
    // -v: verbose (exercises the verbose flag path)
    tmux.run(&["source-file", "-v", tmp.path_str()]);

    let val = tmux.run(&["show-options", "-gv", "@srcverb"]);
    assert_eq!(val.trim(), "yes");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_format_path() {
    let tmux = TmuxServer::new("srcfile_fmt");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let tmp = tmux.write_temp("set -g @srcfmt yes\n");
    // -F: interpret path as format string (but we pass a literal path)
    tmux.run(&["source-file", "-F", tmp.path_str()]);

    let val = tmux.run(&["show-options", "-gv", "@srcfmt"]);
    assert_eq!(val.trim(), "yes");
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_glob() {
    let tmux = TmuxServer::new("srcfile_glob");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Source with glob pattern that matches nothing — -q suppresses error
    let out = tmux.try_run(&["source-file", "-q", "/tmp/tmux_nonexistent_glob_*.conf"]);
    assert!(out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_empty() {
    let tmux = TmuxServer::new("srcfile_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Source an empty file
    let tmp = tmux.write_temp("");
    tmux.run(&["source-file", tmp.path_str()]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_nested() {
    let tmux = TmuxServer::new("srcfile_nested");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create a file that sources another file
    let inner = tmux.write_temp("set -g @nested_inner yes\n");
    let outer_content = format!(
        "source-file {}\nset -g @nested_outer yes\n",
        inner.path_str()
    );
    let outer = tmux.write_temp(&outer_content);
    tmux.run(&["source-file", outer.path_str()]);

    sleep_ms(200);
    let v1 = tmux.run(&["show-options", "-gv", "@nested_outer"]);
    assert_eq!(v1.trim(), "yes");
    let v2 = tmux.run(&["show-options", "-gv", "@nested_inner"]);
    assert_eq!(v2.trim(), "yes");
}
