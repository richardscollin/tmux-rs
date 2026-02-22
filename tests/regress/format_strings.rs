use super::*;

fn assert_format(tmux: &TmuxServer, format: &str, expected: &str) {
    let out = tmux.display(format);
    assert_eq!(
        out, expected,
        "format '{}': expected '{}', got '{}'",
        format, expected, out
    );
}

fn assert_conditional_pane_mode(tmux: &TmuxServer, format: &str, exp_true: &str, exp_false: &str) {
    tmux.run(&["copy-mode"]);
    assert_format(tmux, format, exp_true);
    tmux.run(&["send-keys", "-X", "cancel"]);
    assert_format(tmux, format, exp_false);
}

fn assert_conditional_session_name(
    tmux: &TmuxServer,
    format: &str,
    exp_summer: &str,
    exp_winter: &str,
) {
    tmux.run(&["rename-session", "Summer"]);
    assert_format(tmux, format, exp_summer);
    tmux.run(&["rename-session", "Winter"]);
    assert_format(tmux, format, exp_winter);
    tmux.run(&["rename-session", "Summer"]);
}

#[test]
fn format_strings() {
    let tmux = TmuxServer::new("format_strings");

    // Setup
    tmux.run(&["-f/dev/null", "new-session", "-d"]);
    tmux.run(&["rename-session", "Summer"]);
    tmux.run(&["set", "@true", "1"]);
    tmux.run(&["set", "@false", "0"]);
    tmux.run(&["set", "@warm", "Summer"]);
    tmux.run(&["set", "@cold", "Winter"]);

    // Basic format tests
    assert_format(&tmux, "abc xyz", "abc xyz");
    assert_format(&tmux, "##", "#");
    assert_format(&tmux, "#,", ",");
    assert_format(&tmux, "{", "{");
    assert_format(&tmux, "##{", "#{");
    assert_format(&tmux, "#}", "}");
    assert_format(&tmux, "###}", "#}");
    assert_format(&tmux, "#{pane_in_mode}", "0");
    assert_format(&tmux, "#{?}", "");
    assert_format(&tmux, "#{?abc}", "abc");

    // Conditional with pane_in_mode
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,abc}", "abc", "");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,abc,xyz}", "abc", "xyz");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,abc,@true,xyz}", "abc", "xyz");
    assert_format(&tmux, "#{?@false,abc,@false,xyz}", "");
    assert_format(&tmux, "#{?@false,abc,@false,xyz,default}", "default");

    // Nested format expansions in conditionals
    assert_format(&tmux, "#{?#{@warm}}", "Summer");
    assert_conditional_pane_mode(&tmux, "#{?#{pane_in_mode},#{@warm}}", "Summer", "");
    assert_conditional_pane_mode(
        &tmux,
        "#{?#{pane_in_mode},#{@warm},#{@cold}}",
        "Summer",
        "Winter",
    );

    // Special characters in true/false branches
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,##,xyz}", "#", "xyz");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,#,,xyz}", ",", "xyz");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,{,xyz}", "{", "xyz");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,##{,xyz}", "#{", "xyz");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,#},xyz}", "}", "xyz");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,###},xyz}", "#}", "xyz");

    // Special characters in false branch
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,abc,##}", "abc", "#");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,abc,#,}", "abc", ",");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,abc,{}", "abc", "{");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,abc,##{}", "abc", "#{");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,abc,#}}", "abc", "}");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,abc,###}}", "abc", "#}");

    // Paired special characters in both branches
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,{,#}}", "{", "}");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,#},{}", "}", "{");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,##{,###}}", "#{", "#}");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,###},##{}", "#}", "#{");

    // Braces and parentheses in branches
    assert_conditional_pane_mode(
        &tmux,
        "#{?pane_in_mode,{abc,xyz},bonus}",
        "{abc,bonus}",
        "xyz,bonus}",
    );
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,(abc,xyz),bonus}", "(abc", "");
    assert_conditional_pane_mode(
        &tmux,
        "#{?pane_in_mode,(abc#,xyz),bonus}",
        "(abc,xyz)",
        "bonus",
    );
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,[abc,xyz],bonus}", "[abc", "");
    assert_conditional_pane_mode(
        &tmux,
        "#{?pane_in_mode,[abc#,xyz],bonus}",
        "[abc,xyz]",
        "bonus",
    );

    // #() command expansion in conditionals
    assert_format(&tmux, "#{?pane_in_mode,#(echo #,),xyz}", "xyz");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,#(echo #,),xyz}", "", "xyz");
    assert_conditional_pane_mode(&tmux, "#{?pane_in_mode,#(echo ,)xyz}", "", ")xyz");

    // #[] style sequences in conditionals
    assert_conditional_pane_mode(
        &tmux,
        "#{?pane_in_mode,#[fg=default#,bg=default]abc,xyz}",
        "#[fg=default,bg=default]abc",
        "xyz",
    );
    assert_conditional_pane_mode(
        &tmux,
        "#{?pane_in_mode,#[fg=default,bg=default]abc}",
        "#[fg=default",
        "bg=default]abc",
    );

    // Nested conditionals with session name comparison
    assert_conditional_session_name(
        &tmux,
        "#{?#{==:#{session_name},Summer},abc,xyz}",
        "abc",
        "xyz",
    );

    // Rename session to "," for comma-in-name test
    tmux.run(&["rename-session", ","]);
    assert_format(&tmux, "#{?#{==:#,,#{session_name}},abc,xyz}", "abc");
    tmux.run(&["rename-session", "Summer"]);

    // Nested conditionals with pane_in_mode and session_name
    assert_conditional_pane_mode(
        &tmux,
        "#{?pane_in_mode,#{?#{==:#{session_name},Summer},ABC,XYZ},xyz}",
        "ABC",
        "xyz",
    );
    assert_conditional_session_name(
        &tmux,
        "#{?pane_in_mode,#{?#{==:#{session_name},Summer},ABC,XYZ},xyz}",
        "xyz",
        "xyz",
    );
    assert_conditional_pane_mode(
        &tmux,
        "#{?pane_in_mode,abc,#{?#{==:#{session_name},Summer},ABC,XYZ}}",
        "abc",
        "ABC",
    );
    assert_conditional_session_name(
        &tmux,
        "#{?pane_in_mode,abc,#{?#{==:#{session_name},Summer},ABC,XYZ}}",
        "ABC",
        "XYZ",
    );
    assert_conditional_pane_mode(
        &tmux,
        "#{?#{==:#{?pane_in_mode,#{session_name},#(echo Spring)},Summer},abc,xyz}",
        "abc",
        "xyz",
    );

    // Boolean tests: !! (double negation / to-boolean)
    assert_format(&tmux, "#{!!:0}", "0");
    assert_format(&tmux, "#{!!:}", "0");
    assert_format(&tmux, "#{!!:1}", "1");
    assert_format(&tmux, "#{!!:2}", "1");
    assert_format(&tmux, "#{!!:non-empty string}", "1");
    assert_format(&tmux, "#{!!:-0}", "1");
    assert_format(&tmux, "#{!!:0.0}", "1");

    // Boolean tests: ! (negation)
    assert_format(&tmux, "#{!:0}", "1");
    assert_format(&tmux, "#{!:1}", "0");

    // Boolean tests: && (logical AND)
    assert_format(&tmux, "#{&&:0}", "0");
    assert_format(&tmux, "#{&&:1}", "1");
    assert_format(&tmux, "#{&&:0,0}", "0");
    assert_format(&tmux, "#{&&:0,1}", "0");
    assert_format(&tmux, "#{&&:1,0}", "0");
    assert_format(&tmux, "#{&&:1,1}", "1");
    assert_format(&tmux, "#{&&:0,0,0}", "0");
    assert_format(&tmux, "#{&&:0,1,1}", "0");
    assert_format(&tmux, "#{&&:1,0,1}", "0");
    assert_format(&tmux, "#{&&:1,1,0}", "0");
    assert_format(&tmux, "#{&&:1,1,1}", "1");

    // Boolean tests: || (logical OR)
    assert_format(&tmux, "#{||:0}", "0");
    assert_format(&tmux, "#{||:1}", "1");
    assert_format(&tmux, "#{||:0,0}", "0");
    assert_format(&tmux, "#{||:0,1}", "1");
    assert_format(&tmux, "#{||:1,0}", "1");
    assert_format(&tmux, "#{||:1,1}", "1");
    assert_format(&tmux, "#{||:0,0,0}", "0");
    assert_format(&tmux, "#{||:1,0,0}", "1");
    assert_format(&tmux, "#{||:0,1,0}", "1");
    assert_format(&tmux, "#{||:0,0,1}", "1");
    assert_format(&tmux, "#{||:1,1,1}", "1");

    // Literal format string (l: modifier)
    assert_format(&tmux, "#{l:#{}}", "#{}");
    assert_format(&tmux, "#{l:#{pane_in_mode}}", "#{pane_in_mode}");
    assert_format(
        &tmux,
        "#{l:#{?pane_in_mode,#{?#{==:#{session_name},Summer},ABC,XYZ},xyz}}",
        "#{?pane_in_mode,#{?#{==:#{session_name},Summer},ABC,XYZ},xyz}",
    );
    assert_format(&tmux, "#{l:##{}", "#{");
    assert_format(&tmux, "#{l:#{#}}}", "#{#}}");
}
