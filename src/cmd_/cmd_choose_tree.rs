// Copyright (c) 2012 Thomas Adam <thomas@xteddy.org>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

use crate::*;

pub static mut cmd_choose_tree_entry: cmd_entry = cmd_entry {
    name: c"choose-tree".as_ptr(),
    alias: null_mut(),

    args: args_parse::new(c"F:f:GK:NO:rst:wZ", 0, 1, Some(cmd_choose_tree_args_parse)),
    usage: c"[-GNrswZ] [-F format] [-f filter] [-K key-format] [-O sort-order] [-t target-pane] [template]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),
    source: unsafe { zeroed() },

    flags: cmd_flag::empty(),
    exec: Some(cmd_choose_tree_exec),
};

pub static mut cmd_choose_client_entry: cmd_entry = cmd_entry {
    name: c"choose-client".as_ptr(),
    alias: null_mut(),

    args: args_parse::new(c"F:f:K:NO:rt:Z", 0, 1, Some(cmd_choose_tree_args_parse)),
    usage: c"[-NrZ] [-F format] [-f filter] [-K key-format] [-O sort-order] [-t target-pane] [template]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),
    source: unsafe { zeroed() },

    flags: cmd_flag::empty(),
    exec: Some(cmd_choose_tree_exec),
};

pub static mut cmd_choose_buffer_entry: cmd_entry = cmd_entry {
    name: c"choose-buffer".as_ptr(),
    alias: null_mut(),

    args: args_parse::new(c"F:f:K:NO:rt:Z", 0, 1, Some(cmd_choose_tree_args_parse)),
    usage: c"[-NrZ] [-F format] [-f filter] [-K key-format] [-O sort-order] [-t target-pane] [template]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),
    source: unsafe { zeroed() },

    flags: cmd_flag::empty(),
    exec: Some(cmd_choose_tree_exec),
};

pub static mut cmd_customize_mode_entry: cmd_entry = cmd_entry {
    name: c"customize-mode".as_ptr(),
    alias: null_mut(),

    args: args_parse::new(c"F:f:Nt:Z", 0, 0, None),
    usage: c"[-NZ] [-F format] [-f filter] [-t target-pane]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),
    source: unsafe { zeroed() },

    flags: cmd_flag::empty(),
    exec: Some(cmd_choose_tree_exec),
};

unsafe fn cmd_choose_tree_args_parse(
    _args: *mut args,
    _idx: u32,
    _cause: *mut *mut c_char,
) -> args_parse_type {
    args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING
}

unsafe fn cmd_choose_tree_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let wp = (*target).wp;

        let mode = if cmd_get_entry(self_) == &raw mut cmd_choose_buffer_entry {
            if paste_is_empty() != 0 {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            &raw const window_buffer_mode
        } else if cmd_get_entry(self_) == &raw mut cmd_choose_client_entry {
            if server_client_how_many() == 0 {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            &raw const window_client_mode
        } else if cmd_get_entry(self_) == &raw mut cmd_customize_mode_entry {
            &raw const window_customize_mode
        } else {
            &raw const window_tree_mode
        };

        window_pane_set_mode(wp, null_mut(), mode, target, args);
        cmd_retval::CMD_RETURN_NORMAL
    }
}
