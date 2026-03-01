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

    // -------------------------------------------------------
    // Comparison operators: !=, <, >, <=, >=
    // -------------------------------------------------------
    assert_format(&tmux, "#{!=:abc,abc}", "0");
    assert_format(&tmux, "#{!=:abc,xyz}", "1");
    assert_format(&tmux, "#{<:abc,xyz}", "1");
    assert_format(&tmux, "#{<:xyz,abc}", "0");
    assert_format(&tmux, "#{<:abc,abc}", "0");
    assert_format(&tmux, "#{>:xyz,abc}", "1");
    assert_format(&tmux, "#{>:abc,xyz}", "0");
    assert_format(&tmux, "#{>:abc,abc}", "0");
    assert_format(&tmux, "#{<=:abc,xyz}", "1");
    assert_format(&tmux, "#{<=:abc,abc}", "1");
    assert_format(&tmux, "#{<=:xyz,abc}", "0");
    assert_format(&tmux, "#{>=:xyz,abc}", "1");
    assert_format(&tmux, "#{>=:abc,abc}", "1");
    assert_format(&tmux, "#{>=:abc,xyz}", "0");

    // -------------------------------------------------------
    // Match modifier (m): glob and regex
    // -------------------------------------------------------
    assert_format(&tmux, "#{m:Sum*,Summer}", "1");
    assert_format(&tmux, "#{m:Win*,Summer}", "0");
    assert_format(&tmux, "#{m/ri:^summer$,Summer}", "1");

    // -------------------------------------------------------
    // Substitution modifier (s)
    // -------------------------------------------------------
    assert_format(&tmux, "#{s/Sum/Win/:#{session_name}}", "Winmer");
    assert_format(&tmux, "#{s/xyz/abc/:#{session_name}}", "Summer");

    // -------------------------------------------------------
    // Arithmetic expressions (e)
    // -------------------------------------------------------
    assert_format(&tmux, "#{e|+:3,4}", "7");
    assert_format(&tmux, "#{e|-:10,3}", "7");
    assert_format(&tmux, "#{e|*:3,4}", "12");
    assert_format(&tmux, "#{e|/:12,4}", "3");
    assert_format(&tmux, "#{e|%:10,3}", "1");
    assert_format(&tmux, "#{e|<:3,4}", "1");
    assert_format(&tmux, "#{e|<:4,3}", "0");
    assert_format(&tmux, "#{e|>:4,3}", "1");
    assert_format(&tmux, "#{e|>:3,4}", "0");
    assert_format(&tmux, "#{e|<=:3,3}", "1");
    assert_format(&tmux, "#{e|<=:4,3}", "0");
    assert_format(&tmux, "#{e|>=:3,3}", "1");
    assert_format(&tmux, "#{e|>=:2,3}", "0");
    assert_format(&tmux, "#{e|==:5,5}", "1");
    assert_format(&tmux, "#{e|==:5,6}", "0");
    assert_format(&tmux, "#{e|!=:5,6}", "1");

    // Floating point expressions
    assert_format(&tmux, "#{e|+|f:1.5,2.5}", "4.00");
    assert_format(&tmux, "#{e|*|f|4:3.0,2.0}", "6.0000");

    // -------------------------------------------------------
    // Basename (b) and dirname (d) modifiers
    // -------------------------------------------------------
    tmux.run(&["set", "@path", "/usr/bin/bash"]);
    assert_format(&tmux, "#{b:@path}", "bash");
    assert_format(&tmux, "#{d:@path}", "/usr/bin");

    // -------------------------------------------------------
    // Shell quote (q) and style quote (q/h) modifiers
    // -------------------------------------------------------
    assert_format(&tmux, "#{q:session_name}", "Summer"); // no special chars
    tmux.run(&["set", "@special", "a b"]);
    assert_format(&tmux, "#{q:@special}", "a\\ b");
    tmux.run(&["set", "@hash", "fg=#ff"]);
    assert_format(&tmux, "#{q/h:@hash}", "fg=##ff");

    // -------------------------------------------------------
    // Length (n) and width (w) modifiers
    // -------------------------------------------------------
    assert_format(&tmux, "#{n:session_name}", "6"); // "Summer" = 6
    assert_format(&tmux, "#{w:session_name}", "6");

    // -------------------------------------------------------
    // Padding/truncation (= modifier)
    // -------------------------------------------------------
    assert_format(&tmux, "#{=3:session_name}", "Sum");
    assert_format(&tmux, "#{=-3:session_name}", "mer");
    assert_format(&tmux, "#{=10:session_name}", "Summer");

    // -------------------------------------------------------
    // Expand modifier (E)
    // -------------------------------------------------------
    tmux.run(&["set", "@nested", "#{session_name}"]);
    assert_format(&tmux, "#{E:#{@nested}}", "Summer");

    // -------------------------------------------------------
    // Time modifier (t)
    // -------------------------------------------------------
    // t with pretty formatting (p flag)
    assert_format(
        &tmux,
        "#{t/p:session_activity}",
        &tmux.display("#{t/p:session_activity}"),
    );

    // -------------------------------------------------------
    // Character modifier (a) - number to ASCII
    // -------------------------------------------------------
    assert_format(&tmux, "#{a:65}", "A");
    assert_format(&tmux, "#{a:32}", " ");

    // -------------------------------------------------------
    // Loop modifiers: S (sessions), W (windows), P (panes)
    // -------------------------------------------------------
    // Create a second window for loop tests
    tmux.run(&["new-window"]);
    assert_format(&tmux, "#{W:#{window_index}}", "01");
    assert_format(&tmux, "#{W/n:#{window_index}}", "01"); // sort by name

    // Pane loop
    tmux.run(&["split-window", "-d"]);
    let pane_ids = tmux.display("#{P:#{pane_index}}");
    assert_eq!(
        pane_ids.matches('0').count() + pane_ids.matches('1').count(),
        2
    );

    // Session loop
    let session_list = tmux.display("#{S:#{session_name}}");
    assert!(
        session_list.contains("Summer"),
        "session loop should contain 'Summer', got '{session_list}'"
    );

    // Client loop (may be empty in detached mode, just verify no crash)
    let _client_list = tmux.display("#{C:#{client_name}}");

    // Colour modifier
    assert_format(&tmux, "#{c:red}", "800000");

    // -------------------------------------------------------
    // Pane format variables (format_cb_* callbacks)
    // -------------------------------------------------------
    // cursor position
    let cx = tmux.display("#{cursor_x}");
    assert!(!cx.is_empty(), "cursor_x should be non-empty");
    let cy = tmux.display("#{cursor_y}");
    assert!(!cy.is_empty(), "cursor_y should be non-empty");

    // pane dimensions
    let pane_height = tmux.display("#{pane_height}");
    assert!(pane_height.parse::<u32>().unwrap() > 0);
    let pane_width = tmux.display("#{pane_width}");
    assert!(pane_width.parse::<u32>().unwrap() > 0);

    // pane id, pid, index, tty
    let pane_id = tmux.display("#{pane_id}");
    assert!(
        pane_id.starts_with('%'),
        "pane_id should start with %, got '{pane_id}'"
    );
    let pane_pid = tmux.display("#{pane_pid}");
    assert!(pane_pid.parse::<u32>().unwrap() > 0);
    let pane_index = tmux.display("#{pane_index}");
    assert!(!pane_index.is_empty());
    let pane_tty = tmux.display("#{pane_tty}");
    assert!(!pane_tty.is_empty(), "pane_tty should be non-empty");

    // pane position
    let _pane_top = tmux.display("#{pane_top}");
    let _pane_bottom = tmux.display("#{pane_bottom}");
    let _pane_left = tmux.display("#{pane_left}");
    let _pane_right = tmux.display("#{pane_right}");

    // pane flags
    assert_format(&tmux, "#{pane_active}", "1"); // current pane is active
    assert_format(&tmux, "#{pane_input_off}", "0");
    assert_format(&tmux, "#{pane_pipe}", "0");
    assert_format(&tmux, "#{pane_synchronized}", "0");
    assert_format(&tmux, "#{pane_dead}", "0");
    assert_format(&tmux, "#{pane_marked}", "0");
    assert_format(&tmux, "#{pane_marked_set}", "0");
    assert_format(&tmux, "#{pane_unseen_changes}", "0");

    // pane at edges
    let _at_top = tmux.display("#{pane_at_top}");
    let _at_bottom = tmux.display("#{pane_at_bottom}");
    let _at_left = tmux.display("#{pane_at_left}");
    let _at_right = tmux.display("#{pane_at_right}");

    // cursor mode flags
    let _cursor_flag = tmux.display("#{cursor_flag}");
    let _insert_flag = tmux.display("#{insert_flag}");
    let _keypad_cursor_flag = tmux.display("#{keypad_cursor_flag}");
    let _keypad_flag = tmux.display("#{keypad_flag}");
    let _origin_flag = tmux.display("#{origin_flag}");
    let _wrap_flag = tmux.display("#{wrap_flag}");
    let _mouse_any_flag = tmux.display("#{mouse_any_flag}");
    let _mouse_button_flag = tmux.display("#{mouse_button_flag}");
    let _mouse_sgr_flag = tmux.display("#{mouse_sgr_flag}");
    let _mouse_standard_flag = tmux.display("#{mouse_standard_flag}");
    let _mouse_utf8_flag = tmux.display("#{mouse_utf8_flag}");
    let _mouse_all_flag = tmux.display("#{mouse_all_flag}");
    let _cursor_blinking = tmux.display("#{cursor_blinking}");

    // pane mode/title/path
    let _pane_title = tmux.display("#{pane_title}");
    let _pane_mode = tmux.display("#{pane_mode}");
    let _pane_path = tmux.display("#{pane_path}");
    let _pane_search_string = tmux.display("#{pane_search_string}");

    // scroll region
    let _scroll_region_upper = tmux.display("#{scroll_region_upper}");
    let _scroll_region_lower = tmux.display("#{scroll_region_lower}");

    // history limit
    let hlimit = tmux.display("#{history_limit}");
    assert!(hlimit.parse::<u32>().unwrap() > 0);

    // history size
    let _hsize = tmux.display("#{history_size}");

    // alternate screen
    assert_format(&tmux, "#{alternate_on}", "0");

    // pane start command
    let _start_cmd = tmux.display("#{pane_start_command}");
    let _pane_current_cmd = tmux.display("#{pane_current_command}");
    let _pane_current_path = tmux.display("#{pane_current_path}");

    // cursor character and shape
    let _cursor_character = tmux.display("#{cursor_character}");
    let _cursor_shape = tmux.display("#{cursor_shape}");
    let _cursor_very_visible = tmux.display("#{cursor_very_visible}");
    let _cursor_colour = tmux.display("#{cursor_colour}");

    // extended keys mode
    let _extended_keys = tmux.display("#{extended_keys}");

    // last pane
    let _last = tmux.display("#{pane_last}");

    // pane memory/grid info
    let _mem_used = tmux.display("#{pane_memory_used}");

    // -------------------------------------------------------
    // Window format variables
    // -------------------------------------------------------
    let win_id = tmux.display("#{window_id}");
    assert!(
        win_id.starts_with('@'),
        "window_id should start with @, got '{win_id}'"
    );
    let win_name = tmux.display("#{window_name}");
    assert!(!win_name.is_empty());
    let win_height = tmux.display("#{window_height}");
    assert!(win_height.parse::<u32>().unwrap() > 0);
    let win_width = tmux.display("#{window_width}");
    assert!(win_width.parse::<u32>().unwrap() > 0);
    let _win_flags = tmux.display("#{window_flags}");
    let _win_raw_flags = tmux.display("#{window_raw_flags}");
    assert_format(&tmux, "#{window_zoomed_flag}", "0");
    let _pane_count = tmux.display("#{window_panes}");
    let _win_layout = tmux.display("#{window_layout}");
    let _win_visible_layout = tmux.display("#{window_visible_layout}");
    let _win_active = tmux.display("#{window_active}");
    let _win_activity = tmux.display("#{window_activity}");

    // window position flags
    let _win_start_flag = tmux.display("#{window_start_flag}");
    let _win_end_flag = tmux.display("#{window_end_flag}");
    let _win_last_flag = tmux.display("#{window_last_flag}");
    let _win_linked = tmux.display("#{window_linked}");
    let _win_linked_sessions = tmux.display("#{window_linked_sessions}");
    let _win_linked_sessions_list = tmux.display("#{window_linked_sessions_list}");
    let _win_marked_flag = tmux.display("#{window_marked_flag}");

    // window alert flags
    let _win_activity_flag = tmux.display("#{window_activity_flag}");
    let _win_bell_flag = tmux.display("#{window_bell_flag}");
    let _win_silence_flag = tmux.display("#{window_silence_flag}");

    // active_window_index
    assert_format(&tmux, "#{window_active}", "1");

    // -------------------------------------------------------
    // Session format variables
    // -------------------------------------------------------
    let session_id = tmux.display("#{session_id}");
    assert!(
        session_id.starts_with('$'),
        "session_id should start with $, got '{session_id}'"
    );
    let _session_windows = tmux.display("#{session_windows}");
    let _session_created = tmux.display("#{session_created}");
    let _session_activity = tmux.display("#{session_activity}");
    let _session_path = tmux.display("#{session_path}");
    let _session_stack = tmux.display("#{session_stack}");
    let _session_attached = tmux.display("#{session_attached}");
    assert_format(&tmux, "#{session_many_attached}", "0");
    assert_format(&tmux, "#{session_grouped}", "0");
    let _session_group = tmux.display("#{session_group}");
    let _session_group_attached = tmux.display("#{session_group_attached}");
    let _session_group_many_attached = tmux.display("#{session_group_many_attached}");
    let _session_group_size = tmux.display("#{session_group_size}");
    let _session_marked = tmux.display("#{session_marked}");

    // active_sessions format check
    assert_format(
        &tmux,
        "#{active_window_index}",
        &tmux.display("#{window_index}"),
    );

    // -------------------------------------------------------
    // N modifier: check session/window name existence
    // -------------------------------------------------------
    assert_format(&tmux, "#{N/s:Summer}", "1"); // session "Summer" exists
    assert_format(&tmux, "#{N/s:NoSuchSession}", "0");
    // N/w checks window name - default window name might vary
    let cur_win_name = tmux.display("#{window_name}");
    let nw_check = format!("#{{N/w:{cur_win_name}}}");
    assert_format(&tmux, &nw_check, "1");
    assert_format(&tmux, "#{N/w:NoSuchWindow}", "0");
    // N without flags defaults to window name check
    assert_format(&tmux, &format!("#{{N:{cur_win_name}}}"), "1");

    // -------------------------------------------------------
    // T modifier: expand time
    // -------------------------------------------------------
    tmux.run(&["set", "@timefmt", "%H:%M"]);
    let t_result = tmux.display("#{T:#{@timefmt}}");
    // Should expand strftime-like sequences
    assert!(
        t_result.contains(':'),
        "T modifier should expand time, got '{t_result}'"
    );

    // -------------------------------------------------------
    // R modifier: repeat
    // -------------------------------------------------------
    assert_format(&tmux, "#{R:abc,3}", "abcabcabc");
    assert_format(&tmux, "#{R:x,5}", "xxxxx");
    assert_format(&tmux, "#{R:hi,1}", "hi");

    // -------------------------------------------------------
    // Loop sort variations
    // -------------------------------------------------------
    // Window loop with sort options
    assert_format(&tmux, "#{W/i:#{window_index}}", "01"); // sort by index
    assert_format(&tmux, "#{W/ir:#{window_index}}", "10"); // sort by index, reversed
    let _w_by_time = tmux.display("#{W/t:#{window_index}}"); // sort by time
    let _w_by_name = tmux.display("#{W/n:#{window_name}}"); // sort by name

    // Session loop with sort options
    let _s_by_name = tmux.display("#{S/n:#{session_name}}");
    let _s_by_time = tmux.display("#{S/t:#{session_name}}");
    let _s_reversed = tmux.display("#{S/ir:#{session_name}}");

    // Pane loop reversed
    let _p_reversed = tmux.display("#{P/r:#{pane_index}}");

    // -------------------------------------------------------
    // Padding with marker
    // -------------------------------------------------------
    assert_format(&tmux, "#{=/3/...:session_name}", "Sum...");
    assert_format(&tmux, "#{=/-3/...:session_name}", "...mer");

    // Padding with width (positive = pad right, negative = pad left)
    let p_right = tmux.display("#{p10:session_name}");
    assert!(
        p_right.starts_with("Summer"),
        "p10 should right-pad, got '{p_right}'"
    );
    assert!(
        p_right.len() >= 10,
        "p10 should pad to at least 10, got len {}",
        p_right.len()
    );
    let p_left = tmux.display("#{p-10:session_name}");
    assert!(
        p_left.ends_with("Summer"),
        "p-10 should left-pad, got '{p_left}'"
    );
    assert!(
        p_left.len() >= 10,
        "p-10 should pad to at least 10, got len {}",
        p_left.len()
    );

    // -------------------------------------------------------
    // ! (NOT) modifier
    // -------------------------------------------------------
    assert_format(&tmux, "#{!:#{pane_dead}}", "1"); // pane_dead is 0, so !0 = 1
    assert_format(&tmux, "#{!:#{pane_active}}", "0"); // pane_active is 1, so !1 = 0

    // -------------------------------------------------------
    // Hostname and user format variables
    // -------------------------------------------------------
    let hostname = tmux.display("#{host}");
    assert!(!hostname.is_empty());
    let short_hostname = tmux.display("#{host_short}");
    assert!(!short_hostname.is_empty());
    let user = tmux.display("#{user}");
    assert!(!user.is_empty());

    // -------------------------------------------------------
    // is_format_type callbacks
    // -------------------------------------------------------
    let _is_session = tmux.display("#{session_format}");
    let _is_window = tmux.display("#{window_format}");
    let _is_pane = tmux.display("#{pane_format}");
    let _is_last = tmux.display("#{last_format}");

    // -------------------------------------------------------
    // Match modifier with flags via format strings
    // -------------------------------------------------------
    // m/i: case-insensitive glob
    assert_format(&tmux, "#{m/i:summer,Summer}", "1");
    assert_format(&tmux, "#{m/i:SUMMER,Summer}", "1");
    // m/r: regex match
    assert_format(&tmux, "#{m/r:^Sum,Summer}", "1");
    assert_format(&tmux, "#{m/r:^Win,Summer}", "0");
    // m/ri: case-insensitive regex
    assert_format(&tmux, "#{m/ri:^summer$,Summer}", "1");

    // -------------------------------------------------------
    // Search modifier (C) - search pane content
    // -------------------------------------------------------
    // Send text to the pane so we can search for it
    tmux.run(&["send-keys", "echo SEARCHABLE_TEXT_12345", "Enter"]);
    // Give shell time to process
    tmux.run(&["run-shell", "sleep 0.1"]);
    // C returns the line number where found (non-empty result)
    let search_result = tmux.display("#{C/r:SEARCHABLE_TEXT}");
    assert!(
        !search_result.is_empty(),
        "C modifier should find text in pane"
    );
    let line_num: u32 = search_result
        .parse()
        .expect("C modifier should return a line number");
    assert!(
        line_num > 0,
        "search result line should be > 0, got {line_num}"
    );

    // -------------------------------------------------------
    // Substitution with case-insensitive flag
    // -------------------------------------------------------
    assert_format(&tmux, "#{s/summer/Winter/i:#{session_name}}", "Winter");

    // -------------------------------------------------------
    // Window loop sort by time and reversed
    // -------------------------------------------------------
    let _w_by_time = tmux.display("#{W/t:#{window_name} }");
    let _w_by_time_r = tmux.display("#{W/tr:#{window_name} }");

    // -------------------------------------------------------
    // Pane loop with format showing active pane
    // -------------------------------------------------------
    let pane_active_list = tmux.display("#{P:#{pane_active}}");
    assert!(
        pane_active_list.contains('1'),
        "one pane should be active in '{pane_active_list}'"
    );

    // -------------------------------------------------------
    // Time modifier with custom format (t/f)
    // -------------------------------------------------------
    let time_custom = tmux.display("#{t/f/%Y:session_created}");
    assert!(
        time_custom.len() == 4 && time_custom.chars().all(|c| c.is_ascii_digit()),
        "t/f/%Y should produce 4-digit year, got '{time_custom}'"
    );

    // -------------------------------------------------------
    // L (client loop) modifier - no clients in detached mode
    // -------------------------------------------------------
    let _client_loop = tmux.display("#{L:#{client_name} }");
    let _client_loop_n = tmux.display("#{L/n:#{client_name} }");

    // -------------------------------------------------------
    // Session loop with sort by name and time
    // (create extra sessions last since they change context)
    // -------------------------------------------------------
    tmux.run(&["new-session", "-d", "-s", "AAA_first"]);
    tmux.run(&["new-session", "-d", "-s", "ZZZ_last"]);

    // Sort by name should put AAA_first before Summer before ZZZ_last
    let s_by_name = tmux.display("#{S/n:#{session_name} }");
    assert!(
        s_by_name.contains("AAA_first") && s_by_name.contains("ZZZ_last"),
        "S/n should list sessions, got '{s_by_name}'"
    );
    let aaa_pos = s_by_name.find("AAA_first").unwrap();
    let zzz_pos = s_by_name.find("ZZZ_last").unwrap();
    assert!(
        aaa_pos < zzz_pos,
        "S/n: AAA should come before ZZZ in '{s_by_name}'"
    );

    // Sort by name reversed
    let s_by_name_r = tmux.display("#{S/nr:#{session_name} }");
    let aaa_pos_r = s_by_name_r.find("AAA_first").unwrap();
    let zzz_pos_r = s_by_name_r.find("ZZZ_last").unwrap();
    assert!(
        zzz_pos_r < aaa_pos_r,
        "S/nr: ZZZ should come before AAA in '{s_by_name_r}'"
    );

    // Sort by time
    let _s_by_time = tmux.display("#{S/t:#{session_name} }");

    // Sort by index reversed
    let _s_by_idx_r = tmux.display("#{S/ir:#{session_name} }");

    // Clean up extra sessions
    tmux.run(&["kill-session", "-t", "AAA_first"]);
    tmux.run(&["kill-session", "-t", "ZZZ_last"]);

    // -------------------------------------------------------
    // Environment variable lookup in format_find
    // -------------------------------------------------------
    // Set an environment variable in the server
    tmux.run(&["set-environment", "TMUX_TEST_FMT_VAR", "hello_env"]);
    assert_format(&tmux, "#{TMUX_TEST_FMT_VAR}", "hello_env");

    // -------------------------------------------------------
    // Pane foreground/background colours
    // -------------------------------------------------------
    let _pfg = tmux.display("#{pane_fg}");
    let _pbg = tmux.display("#{pane_bg}");

    // -------------------------------------------------------
    // Tab stops
    // -------------------------------------------------------
    let _tabs = tmux.display("#{pane_tabs}");

    // -------------------------------------------------------
    // History bytes
    // -------------------------------------------------------
    let _hbytes = tmux.display("#{history_bytes}");
    let _all_bytes = tmux.display("#{history_all_bytes}");

    // -------------------------------------------------------
    // Window stack index
    // -------------------------------------------------------
    let _stack_idx = tmux.display("#{window_stack_index}");

    // -------------------------------------------------------
    // Session alerts/flags
    // -------------------------------------------------------
    let _session_alerts = tmux.display("#{session_alerts}");
    let _session_alert = tmux.display("#{session_alert}");
    let _session_activity_flag = tmux.display("#{session_activity_flag}");
    let _session_bell_flag = tmux.display("#{session_bell_flag}");
    let _session_silence_flag = tmux.display("#{session_silence_flag}");

    // -------------------------------------------------------
    // Window active clients
    // -------------------------------------------------------
    let _win_active_clients = tmux.display("#{window_active_clients}");
    let _win_active_clients_list = tmux.display("#{window_active_clients_list}");
    let _win_active_sessions = tmux.display("#{window_active_sessions}");
    let _win_active_sessions_list = tmux.display("#{window_active_sessions_list}");
}
