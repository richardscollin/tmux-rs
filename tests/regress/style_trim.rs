use super::*;

/// Check that display-message -p expands the format string correctly.
fn check_display(tmux: &TmuxServer, format: &str, expected: &str) {
    let v = tmux.display(format);
    assert_eq!(
        v, expected,
        "format '{}': display got '{}', expected '{}'",
        format, v, expected
    );
}

/// Check that the rendered status line (captured via an outer tmux server)
/// matches the expected escape-stripped output.
fn check_rendered(tmux: &TmuxServer, tmux2: &TmuxServer, format: &str, expected_rendered: &str) {
    tmux.run(&["set", "-g", "status-format[0]", format]);
    sleep_secs(1);
    let output = tmux2.run(&["capturep", "-Cep"]);
    let last_line = output.lines().last().unwrap_or("");
    // Strip \033[ sequences - capturep -C outputs escape-notation (literal \033[)
    // The shell does: sed 's|\\033\[||g' which strips the literal text \033[
    let rendered = last_line.replace("\\033[", "");
    assert_eq!(
        rendered, expected_rendered,
        "format '{}': rendered got '{}', expected '{}'",
        format, rendered, expected_rendered
    );
}

/// Full check: display output and rendered output.
fn check(
    tmux: &TmuxServer,
    tmux2: &TmuxServer,
    format: &str,
    expected_display: &str,
    expected_rendered: &str,
) {
    check_display(tmux, format, expected_display);
    check_rendered(tmux, tmux2, format, expected_rendered);
}

/// Ported from style-trim.sh
///
/// Tests format string width/trimming with style codes using two tmux servers.
/// The outer server (tmux2) captures the inner server's (tmux) status line output.
#[test]
#[cfg_attr(not(feature = "slow-tests"), ignore)]
fn style_trim() {
    let binary = TmuxServer::binary_path();

    let tmux = TmuxServer::new("style_trim");
    let tmux2 = TmuxServer::new("style_trim2");

    // Use bash if available for a clean shell
    let shell = if std::path::Path::new("/bin/bash").exists() {
        "bash --noprofile --norc +o history"
    } else {
        "sh"
    };

    // Outer server creates a session running inner tmux
    let inner_cmd = format!(
        "{} -L{} -f/dev/null new -- {}",
        binary,
        tmux.socket(),
        shell
    );
    tmux2.run(&["-f/dev/null", "new", "-d", &inner_cmd]);
    sleep_secs(2);
    tmux.run(&["set", "-g", "status-style", "fg=default,bg=default"]);

    // V = #0, drawn as #0
    tmux.run(&["setenv", "-g", "V", "#0"]);
    check(&tmux, &tmux2, "#{V} #{w:V}", "#0 2", "#0 2");
    check(&tmux, &tmux2, "#{=3:V}", "#0", "#0");
    check(&tmux, &tmux2, "#{=-3:V}", "#0", "#0");

    // V = ###[bg=yellow]0, drawn as #0 (with bg=yellow style)
    tmux.run(&["setenv", "-g", "V", "###[bg=yellow]0"]);
    check(
        &tmux,
        &tmux2,
        "#{V} #{w:V}",
        "###[bg=yellow]0 2",
        "#43m0 249m",
    );
    check(&tmux, &tmux2, "#{=3:V}", "###[bg=yellow]0", "#43m049m");
    check(&tmux, &tmux2, "#{=-3:V}", "###[bg=yellow]0", "#43m049m");

    // V = #0123456, drawn as #0123456
    tmux.run(&["setenv", "-g", "V", "#0123456"]);
    check(&tmux, &tmux2, "#{V} #{w:V}", "#0123456 8", "#0123456 8");
    check(&tmux, &tmux2, "#{=3:V}", "#01", "#01");
    check(&tmux, &tmux2, "#{=-3:V}", "456", "456");

    // V = ##0123456, drawn as #0123456
    tmux.run(&["setenv", "-g", "V", "##0123456"]);
    check(&tmux, &tmux2, "#{V} #{w:V}", "##0123456 8", "#0123456 8");
    check(&tmux, &tmux2, "#{=3:V}", "##01", "#01");
    check(&tmux, &tmux2, "#{=-3:V}", "456", "456");

    // V = ###0123456, drawn as ##0123456
    tmux.run(&["setenv", "-g", "V", "###0123456"]);
    check(&tmux, &tmux2, "#{V} #{w:V}", "###0123456 9", "##0123456 9");
    check(&tmux, &tmux2, "#{=3:V}", "####0", "##0");
    check(&tmux, &tmux2, "#{=-3:V}", "456", "456");

    // V = #[bg=yellow]0123456, drawn as 0123456 (with bg=yellow style)
    tmux.run(&["setenv", "-g", "V", "#[bg=yellow]0123456"]);
    check(
        &tmux,
        &tmux2,
        "#{V} #{w:V}",
        "#[bg=yellow]0123456 7",
        "43m0123456 749m",
    );
    check(&tmux, &tmux2, "#{=3:V}", "#[bg=yellow]012", "43m01249m");
    check(&tmux, &tmux2, "#{=-3:V}", "#[bg=yellow]456", "43m45649m");

    // V = ##[bg=yellow]0123456, drawn as #[bg=yellow]0123456 (literal)
    tmux.run(&["setenv", "-g", "V", "##[bg=yellow]0123456"]);
    check(
        &tmux,
        &tmux2,
        "#{V} #{w:V}",
        "##[bg=yellow]0123456 19",
        "#[bg=yellow]0123456 19",
    );
    check(&tmux, &tmux2, "#{=3:V}", "##[b", "#[b");
    check(&tmux, &tmux2, "#{=-3:V}", "456", "456");

    // V = ###[bg=yellow]0123456, drawn as #0123456 (with bg=yellow style)
    tmux.run(&["setenv", "-g", "V", "###[bg=yellow]0123456"]);
    check(
        &tmux,
        &tmux2,
        "#{V} #{w:V}",
        "###[bg=yellow]0123456 8",
        "#43m0123456 849m",
    );
    check(&tmux, &tmux2, "#{=3:V}", "###[bg=yellow]01", "#43m0149m");
    check(&tmux, &tmux2, "#{=-3:V}", "#[bg=yellow]456", "43m45649m");

    // V = ####[bg=yellow]0123456, drawn as ##[bg=yellow]0123456 (literal)
    tmux.run(&["setenv", "-g", "V", "####[bg=yellow]0123456"]);
    check(
        &tmux,
        &tmux2,
        "#{V} #{w:V}",
        "####[bg=yellow]0123456 20",
        "##[bg=yellow]0123456 20",
    );
    check(&tmux, &tmux2, "#{=3:V}", "####[", "##[");
    check(&tmux, &tmux2, "#{=-3:V}", "456", "456");

    // V = #####[bg=yellow]0123456, drawn as ###0123456 (with bg=yellow style)
    tmux.run(&["setenv", "-g", "V", "#####[bg=yellow]0123456"]);
    check(
        &tmux,
        &tmux2,
        "#{V} #{w:V}",
        "#####[bg=yellow]0123456 9",
        "##43m0123456 949m",
    );
    check(&tmux, &tmux2, "#{=3:V}", "#####[bg=yellow]0", "##43m049m");
    check(&tmux, &tmux2, "#{=-3:V}", "#[bg=yellow]456", "43m45649m");

    tmux.kill_server();
    tmux2.kill_server();
}
