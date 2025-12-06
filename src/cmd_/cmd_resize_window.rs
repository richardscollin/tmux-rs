// Copyright (c) 2018 Nicholas Marriott <nicholas.marriott@gmail.com>
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

pub static CMD_RESIZE_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: "resize-window",
    alias: Some("resizew"),

    args: args_parse::new("aADLRt:Ux:y:", 0, 1, None),
    usage: "[-aADLRU] [-x width] [-y height] [-t target-window] [adjustment]",

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_WINDOW,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_resize_window_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_resize_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let wl = (*target).wl;
        let w = (*wl).window;
        let s = (*target).s;
        let mut cause = null_mut();
        let mut xpixel = 0u32;
        let mut ypixel = 0u32;

        let adjust = if args_count(args) == 0 {
            1
        } else {
            match strtonum(args_string(args, 0), 1, i32::MAX) {
                Ok(n) => n as u32,
                Err(errstr) => {
                    cmdq_error!(item, "adjustment {}", _s(errstr.as_ptr()));
                    return cmd_retval::CMD_RETURN_ERROR;
                }
            }
        };

        let mut sx = (*w).sx;
        let mut sy = (*w).sy;

        if args_has(args, 'x') {
            sx = args_strtonum(
                args,
                b'x',
                WINDOW_MINIMUM as _,
                WINDOW_MAXIMUM as _,
                &raw mut cause,
            ) as u32;
            if !cause.is_null() {
                cmdq_error!(item, "width {}", _s(cause));
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }
        if args_has(args, 'y') {
            sy = args_strtonum(
                args,
                b'y',
                WINDOW_MINIMUM as _,
                WINDOW_MAXIMUM as _,
                &raw mut cause,
            ) as u32;
            if !cause.is_null() {
                cmdq_error!(item, "height {}", _s(cause));
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }

        if args_has(args, 'L') {
            if sx >= adjust {
                sx -= adjust;
            }
        } else if args_has(args, 'R') {
            sx += adjust;
        } else if args_has(args, 'U') {
            if sy >= adjust {
                sy -= adjust;
            }
        } else if args_has(args, 'D') {
            sy += adjust;
        }

        if args_has(args, 'A') {
            default_window_size(
                null_mut(),
                s,
                w,
                &raw mut sx,
                &raw mut sy,
                &raw mut xpixel,
                &raw mut ypixel,
                Some(window_size_option::WINDOW_SIZE_LARGEST),
            );
        } else if args_has(args, 'a') {
            default_window_size(
                null_mut(),
                s,
                w,
                &raw mut sx,
                &raw mut sy,
                &raw mut xpixel,
                &raw mut ypixel,
                Some(window_size_option::WINDOW_SIZE_SMALLEST),
            );
        }

        options_set_number(
            (*w).options,
            c!("window-size"),
            window_size_option::WINDOW_SIZE_MANUAL as i64,
        );
        (*w).manual_sx = sx;
        (*w).manual_sy = sy;
        recalculate_size(w, 1);

        cmd_retval::CMD_RETURN_NORMAL
    }
}
