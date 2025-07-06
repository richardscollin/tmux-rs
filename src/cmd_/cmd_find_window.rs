// Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
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

pub static cmd_find_window_entry: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"find-window"),
    alias: SyncCharPtr::new(c"findw"),

    args: args_parse::new(c"CiNrt:TZ", 1, 1, None),
    usage: SyncCharPtr::new(c"[-CiNrTZ] [-t target-pane] match-string"),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: cmd_flag::empty(),
    exec: cmd_find_window_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_find_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let wp = (*target).wp;
        let s = args_string(args, 0);
        let mut suffix = c!("");
        let mut star = c!("*");

        let mut c = args_has_(args, 'C');
        let mut n = args_has_(args, 'N');
        let mut t = args_has_(args, 'T');

        if args_has(args, b'r') != 0 {
            star = c!("");
        }
        if args_has(args, b'r') != 0 && args_has(args, b'i') != 0 {
            suffix = c!("/ri");
        } else if args_has(args, b'r') != 0 {
            suffix = c!("/r");
        } else if args_has(args, b'i') != 0 {
            suffix = c!("/i");
        }

        if !c && !n && !t {
            c = true;
            n = true;
            t = true;
        }

        let filter = xcalloc_::<args_value>(1).as_ptr();
        (*filter).type_ = args_type::ARGS_STRING;

        (*filter).union_.string = if c && n && t {
            format_nul!(
                "#{{||:#{{C{}:{}}},#{{||:#{{m{}:{}{}{},#{{window_name}}}},#{{m{}:{}{}{},#{{pane_title}}}}}}}}",
                _s(suffix),
                _s(s),
                _s(suffix),
                _s(star),
                _s(s),
                _s(star),
                _s(suffix),
                _s(star),
                _s(s),
                _s(star),
            )
        } else if c && n {
            format_nul!(
                "#{{{{||:#{{{{C{}:{}}}}},#{{{{m{}:{}{}{},#{{{{window_name}}}}}}}}}}}}",
                _s(suffix),
                _s(s),
                _s(suffix),
                _s(star),
                _s(s),
                _s(star)
            )
        } else if c && t {
            format_nul!(
                "#{{||:#{{C{}:{}}},#{{m{}:{}{}{},#{{pane_title}}}}}}",
                _s(suffix),
                _s(s),
                _s(suffix),
                _s(star),
                _s(s),
                _s(star),
            )
        } else if n && t {
            format_nul!(
                "#{{||:#{{m{}:{}{}{},#{{window_name}}}},#{{m{}:{}{}{},#{{pane_title}}}}}}",
                _s(suffix),
                _s(star),
                _s(s),
                _s(star),
                _s(suffix),
                _s(star),
                _s(s),
                _s(star),
            )
        } else if c {
            format_nul!("#{{C{}:{}}}", _s(suffix), _s(s),)
        } else if n {
            format_nul!(
                "#{{m{}:{}{}{},#{{window_name}}}}",
                _s(suffix),
                _s(star),
                _s(s),
                _s(star),
            )
        } else {
            format_nul!(
                "#{{m{}:{}{}{},#{{pane_title}}}}",
                _s(suffix),
                _s(star),
                _s(s),
                _s(star),
            )
        };

        let new_args: *mut args = args_create();
        if args_has_(args, 'Z') {
            args_set(new_args, b'Z', null_mut(), 0);
        }
        args_set(new_args, b'f', filter, 0);

        window_pane_set_mode(
            wp,
            null_mut(),
            &raw const window_tree_mode,
            target,
            new_args,
        );
        args_free(new_args);

        cmd_retval::CMD_RETURN_NORMAL
    }
}
