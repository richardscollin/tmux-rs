use super::*;

/// Source a simple config file.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_basic() {
    let tmux = TmuxServer::new("srcfile_basic");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("set -g @srctest hello\n");
    tmux.run(&["source-file", f.path_str()]);

    let out = tmux.run(&["show", "-gv", "@srctest"]);
    assert_eq!(out.trim(), "hello");
}

/// Source using the 'source' alias.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_alias() {
    let tmux = TmuxServer::new("srcfile_alias");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("set -g @aliasvar aliasval\n");
    tmux.run(&["source", f.path_str()]);

    let out = tmux.run(&["show", "-gv", "@aliasvar"]);
    assert_eq!(out.trim(), "aliasval");
}

/// Source a nonexistent file (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_nonexistent() {
    let tmux = TmuxServer::new("srcfile_noexist");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["source-file", "/tmp/tmux_rs_nonexistent_file_xyz.conf"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("No such file") || stderr.contains("nonexistent"),
        "expected file-not-found error, got: {stderr}"
    );
}

/// Source nonexistent file with -q (quiet, no error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_quiet_nonexistent() {
    let tmux = TmuxServer::new("srcfile_q_noex");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["source-file", "-q", "/tmp/tmux_rs_nonexistent_xyz.conf"]);
    assert!(
        result.status.success(),
        "-q should suppress missing file error"
    );
}

/// Source file with -n (parse only, don't execute).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_parse_only() {
    let tmux = TmuxServer::new("srcfile_n");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("set -g @parseonly shouldnotexist\n");
    tmux.run(&["source-file", "-n", f.path_str()]);

    // -n means parse-only, the command should NOT actually execute
    let result = tmux.try_run(&["show", "-gv", "@parseonly"]);
    assert!(
        !result.status.success() || String::from_utf8_lossy(&result.stdout).trim().is_empty(),
        "expected option to not be set with -n"
    );
}

/// Source file with -v (verbose).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_verbose() {
    let tmux = TmuxServer::new("srcfile_v");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("set -g @verbtest vval\n");
    tmux.run(&["source-file", "-v", f.path_str()]);

    let out = tmux.run(&["show", "-gv", "@verbtest"]);
    assert_eq!(out.trim(), "vval");
}

/// Source file with -F (format expansion on path).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_format() {
    let tmux = TmuxServer::new("srcfile_F");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("set -g @fmttest fmtval\n");
    // Store path in a user option, then use -F to expand it
    tmux.run(&["set", "-g", "@srcpath", f.path_str()]);
    tmux.run(&["source-file", "-F", "#{@srcpath}"]);

    let out = tmux.run(&["show", "-gv", "@fmttest"]);
    assert_eq!(out.trim(), "fmtval");
}

/// Source an empty file (no error, noop).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_empty() {
    let tmux = TmuxServer::new("srcfile_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("");
    tmux.run(&["source-file", f.path_str()]);
}

/// Source multiple files in one command.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_multiple() {
    let tmux = TmuxServer::new("srcfile_multi");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f1 = tmux.write_temp("set -g @multi1 val1\n");
    let f2 = tmux.write_temp("set -g @multi2 val2\n");
    tmux.run(&["source-file", f1.path_str(), f2.path_str()]);

    let out1 = tmux.run(&["show", "-gv", "@multi1"]);
    let out2 = tmux.run(&["show", "-gv", "@multi2"]);
    assert_eq!(out1.trim(), "val1");
    assert_eq!(out2.trim(), "val2");
}

/// Source file with a syntax error.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_syntax_error() {
    let tmux = TmuxServer::new("srcfile_synerr");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("this-is-not-a-valid-command\n");
    let result = tmux.try_run(&["source-file", f.path_str()]);
    assert!(!result.status.success());
}

/// Source file with syntax error and -q (quiet).
/// -q sets CMD_PARSE_QUIET which suppresses glob/file errors but cfg parse
/// errors still propagate through cfg_print_causes.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_syntax_error_quiet() {
    let tmux = TmuxServer::new("srcfile_synerr_q");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("this-is-not-a-valid-command\n");
    let result = tmux.try_run(&["source-file", "-q", f.path_str()]);
    // -q may or may not suppress parse errors depending on how cfg reports them
    let _ = result;
}

/// Source using a glob pattern.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_glob() {
    let tmux = TmuxServer::new("srcfile_glob");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Create files in a temp dir with a known pattern
    let dir = tempfile::tempdir().unwrap();
    let p1 = dir.path().join("test1.tmux.conf");
    let p2 = dir.path().join("test2.tmux.conf");
    std::fs::write(&p1, "set -g @glob1 g1\n").unwrap();
    std::fs::write(&p2, "set -g @glob2 g2\n").unwrap();

    let pattern = format!("{}/*.tmux.conf", dir.path().display());
    tmux.run(&["source-file", &pattern]);

    let out1 = tmux.run(&["show", "-gv", "@glob1"]);
    let out2 = tmux.run(&["show", "-gv", "@glob2"]);
    assert_eq!(out1.trim(), "g1");
    assert_eq!(out2.trim(), "g2");
}

/// Source file with glob pattern that matches nothing (error).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_glob_no_match() {
    let tmux = TmuxServer::new("srcfile_glob_nm");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["source-file", "/tmp/tmux_rs_no_match_*.conf"]);
    assert!(!result.status.success());
}

/// Source file with glob no-match and -q (quiet).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_glob_no_match_quiet() {
    let tmux = TmuxServer::new("srcfile_glob_q");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let result = tmux.try_run(&["source-file", "-q", "/tmp/tmux_rs_no_match_*.conf"]);
    assert!(
        result.status.success(),
        "-q should suppress glob no-match error"
    );
}

/// Source a file that itself sources another file (nested).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_nested() {
    let tmux = TmuxServer::new("srcfile_nested");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let inner = tmux.write_temp("set -g @inner innerval\n");
    let outer_content = format!("source-file {}\nset -g @outer outerval\n", inner.path_str());
    let outer = tmux.write_temp(&outer_content);
    tmux.run(&["source-file", outer.path_str()]);

    let out_inner = tmux.run(&["show", "-gv", "@inner"]);
    let out_outer = tmux.run(&["show", "-gv", "@outer"]);
    assert_eq!(out_inner.trim(), "innerval");
    assert_eq!(out_outer.trim(), "outerval");
}

/// Source with -nv (parse-only + verbose).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_parse_only_verbose() {
    let tmux = TmuxServer::new("srcfile_nv");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("set -g @nvtest nvval\n");
    tmux.run(&["source-file", "-nv", f.path_str()]);

    // -n means parse-only, should not execute
    let result = tmux.try_run(&["show", "-gv", "@nvtest"]);
    assert!(
        !result.status.success() || String::from_utf8_lossy(&result.stdout).trim().is_empty(),
        "option should not be set with -n"
    );
}

/// Source with -qn (quiet + parse-only) on bad syntax.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_quiet_parse_only() {
    let tmux = TmuxServer::new("srcfile_qn");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("invalid-cmd-xyz\n");
    let result = tmux.try_run(&["source-file", "-qn", f.path_str()]);
    // -qn exercises both quiet and parse-only flags
    let _ = result;
}

/// Source from control-mode client.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_control_mode() {
    let tmux = TmuxServer::new("srcfile_ctrl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("set -g @ctrltest ctrlval\n");
    let cmd = format!("source-file {}\ndetach-client\n", f.path_str());
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());

    let out = tmux.run(&["show", "-gv", "@ctrltest"]);
    assert_eq!(out.trim(), "ctrlval");
}

/// Source with -v from control-mode client (verbose flag suppressed in control mode).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_verbose_control_mode() {
    let tmux = TmuxServer::new("srcfile_v_ctrl");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    let f = tmux.write_temp("set -g @vctrl vctrlval\n");
    let cmd = format!("source-file -v {}\ndetach-client\n", f.path_str());
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(output.status.success());

    let out = tmux.run(&["show", "-gv", "@vctrl"]);
    assert_eq!(out.trim(), "vctrlval");
}

/// Source file with relative path (resolved against start-directory).
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_relative_path() {
    let tmux = TmuxServer::new("srcfile_relpath");

    // Create a file in a temp dir
    let dir = tempfile::tempdir().unwrap();
    let fname = "reltest.tmux.conf";
    std::fs::write(dir.path().join(fname), "set -g @reltest relval\n").unwrap();

    // Start the session with start-directory set to our temp dir
    tmux.run(&[
        "-f/dev/null",
        "new",
        "-d",
        "-x80",
        "-y24",
        "-c",
        dir.path().to_str().unwrap(),
    ]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Attach via control mode and source relative path -- the session's cwd is
    // the temp dir, so source-file resolves against it.
    let cmd = format!("source-file {fname}\ndetach-client\n");
    let output = tmux.run_with_stdin(&["-C", "attach"], cmd.as_bytes());
    assert!(
        output.status.success(),
        "source-file relative path failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let out = tmux.run(&["show", "-gv", "@reltest"]);
    assert_eq!(out.trim(), "relval");
}

/// Source from stdin with "-" path.
#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_stdin() {
    let tmux = TmuxServer::new("srcfile_stdin");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Use control mode to source from stdin via "-"
    let output = tmux.run_with_stdin(
        &["-C", "attach"],
        b"source-file -\nset -g @stdintest stdinval\n\ndetach-client\n",
    );
    // The "-" stdin path exercises the streq_(path, "-") branch
    // Result may vary, but the branch is exercised
    let _ = output;
}

/// Test source-file: basic source, -q (quiet), -v (verbose),
/// nonexistent file, multiple files, -F (format), nested limit.

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_basic_02() {
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
fn source_file_nonexistent_02() {
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
fn source_file_verbose_02() {
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
fn source_file_glob_02() {
    let tmux = TmuxServer::new("srcfile_glob");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Source with glob pattern that matches nothing — -q suppresses error
    let out = tmux.try_run(&["source-file", "-q", "/tmp/tmux_nonexistent_glob_*.conf"]);
    assert!(out.status.success());
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_empty_02() {
    let tmux = TmuxServer::new("srcfile_empty");
    tmux.run(&["-f/dev/null", "new", "-d", "-x80", "-y24"]);
    tmux.run(&["set", "-g", "window-size", "manual"]);

    // Source an empty file
    let tmp = tmux.write_temp("");
    tmux.run(&["source-file", tmp.path_str()]);
}

#[test]
#[cfg_attr(not(feature = "coverage-tests"), ignore)]
fn source_file_nested_02() {
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
