// Copyright (c) 2009 Jonathan Alvarado <radobobo@users.u8forge.net>
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
use std::io::Write;

use crate::*;

pub static CMD_CAPTURE_PANE_ENTRY: cmd_entry = cmd_entry {
    name: "capture-pane",
    alias: Some("capturep"),

    args: args_parse::new("ab:CeE:JNpPqS:Tt:", 0, 0, None),
    usage: "[-aCeJNpPqT] [-b buffer-name] [-E end-line] [-S start-line] [-t target-pane]",

    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, cmd_find_flags::empty()),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_capture_pane_exec,
};

pub static CMD_CLEAR_HISTORY_ENTRY: cmd_entry = cmd_entry {
    name: "clear-history",
    alias: Some("clearhist"),

    args: args_parse::new("Ht:", 0, 0, None),
    usage: "[-H] [-t target-pane]",

    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag {
        flag: b't' as _,
        type_: cmd_find_type::CMD_FIND_PANE,
        flags: cmd_find_flags::empty(),
    },

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_capture_pane_exec,
};

unsafe fn cmd_capture_pane_append(
    mut buf: *mut u8,
    len: *mut usize,
    line: *mut u8,
    linelen: usize,
) -> *mut u8 {
    unsafe {
        buf = xrealloc_(buf, *len + linelen + 1).as_ptr();
        memcpy_(buf.add(*len), line, linelen);
        *len += linelen;
        buf
    }
}

unsafe fn cmd_capture_pane_pending(
    args: *mut args,
    wp: *const window_pane,
    len: *mut usize,
) -> *mut u8 {
    let mut tmp: [u8; 5] = [0; 5];

    unsafe {
        let pending = input_pending((*wp).ictx);
        if pending.is_null() {
            return xstrdup(c!("")).as_ptr();
        }

        let mut line = EVBUFFER_DATA(pending);
        let linelen = EVBUFFER_LENGTH(pending);

        let mut buf = xstrdup(c!("")).as_ptr();
        if args_has(args, 'C') {
            for i in 0usize..linelen {
                if *line.add(i) >= b' ' && *line.add(i) != b'\\' {
                    tmp[0] = *line.add(i) as _;
                    tmp[1] = b'\0' as _;
                } else {
                    _ = write!(tmp.as_mut_slice(), "\\{:03o}\0", *line.add(i));
                }
                buf =
                    cmd_capture_pane_append(buf, len, &raw mut tmp as _, strlen(&raw mut tmp as _));
            }
        } else {
            buf = cmd_capture_pane_append(buf, len, &raw mut line as _, linelen);
        }
        buf
    }
}

unsafe fn cmd_capture_pane_history(
    args: *mut args,
    item: *mut cmdq_item,
    wp: *mut window_pane,
    len: *mut usize,
) -> *mut u8 {
    unsafe {
        let gd: *mut grid;
        let mut gl: *const grid_line;
        let mut gc: *mut grid_cell = null_mut();
        let mut n;
        let mut flags = grid_string_flags::empty();

        let tmp: u32;
        let mut bottom: u32;
        let mut cause: *mut u8 = null_mut();
        let mut line: *mut u8;

        let mut linelen: usize;

        let sx = screen_size_x(&raw mut (*wp).base);
        if args_has(args, 'a') {
            gd = (*wp).base.saved_grid;
            if gd.is_null() {
                if !args_has(args, 'q') {
                    cmdq_error!(item, "no alternate screen");
                    return null_mut();
                }
                return xstrdup(c!("")).as_ptr();
            }
        } else {
            gd = (*wp).base.grid;
        }

        let sflag: *const u8 = args_get(args, b'S');
        let mut top;
        if !sflag.is_null() && streq_(sflag, "-") {
            top = 0;
        } else {
            n = args_strtonum_and_expand(
                args,
                b'S',
                i32::MIN as i64,
                i16::MAX as i64,
                item,
                &raw mut cause,
            );
            if !cause.is_null() {
                top = (*gd).hsize;
                free_(cause);
            } else if n < 0 && (-n) as u32 > (*gd).hsize {
                top = 0;
            } else {
                top = (*gd).hsize + n as u32;
            }
            if top > (*gd).hsize + (*gd).sy - 1 {
                top = (*gd).hsize + (*gd).sy - 1;
            }
        }

        let eflag: *const u8 = args_get(args, b'E');
        if !eflag.is_null() && streq_(eflag, "-") {
            bottom = (*gd).hsize + (*gd).sy - 1;
        } else {
            n = args_strtonum_and_expand(
                args,
                b'E',
                i32::MIN as i64,
                i16::MAX as i64,
                item,
                &raw mut cause,
            );
            if !cause.is_null() {
                bottom = (*gd).hsize + (*gd).sy - 1;
                free_(cause);
            } else if n < 0 && (-n) as u32 > (*gd).hsize {
                bottom = 0;
            } else {
                bottom = (*gd).hsize + n as u32;
            }
            if bottom > (*gd).hsize + (*gd).sy - 1 {
                bottom = (*gd).hsize + (*gd).sy - 1;
            }
        }

        if bottom < top {
            tmp = bottom;
            bottom = top;
            top = tmp;
        }

        let join_lines = args_has(args, 'J');
        if args_has(args, 'e') {
            flags |= grid_string_flags::GRID_STRING_WITH_SEQUENCES;
        }
        if args_has(args, 'C') {
            flags |= grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES;
        }
        if !join_lines && !args_has(args, 'T') {
            flags |= grid_string_flags::GRID_STRING_EMPTY_CELLS;
        }
        if !join_lines && !args_has(args, 'N') {
            flags |= grid_string_flags::GRID_STRING_TRIM_SPACES;
        }

        let mut buf = null_mut();
        for i in top..=bottom {
            line = grid_string_cells(gd, 0, i, sx, &raw mut gc, flags, (*wp).screen);
            linelen = strlen(line);

            buf = cmd_capture_pane_append(buf, len, line, linelen);

            gl = grid_peek_line(gd, i);
            if !join_lines || !(*gl).flags.intersects(grid_line_flag::WRAPPED) {
                *buf.add(*len) = b'\n' as _;
                (*len) += 1;
            }

            free_(line);
        }
        buf
    }
}

unsafe fn cmd_capture_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let c = cmdq_get_client(item);
        let wp = (*cmdq_get_target(item)).wp;

        if std::ptr::eq(cmd_get_entry(self_), &CMD_CLEAR_HISTORY_ENTRY) {
            window_pane_reset_mode_all(wp);
            grid_clear_history((*wp).base.grid);
            if args_has(args, 'H') {
                screen_reset_hyperlinks((*wp).screen);
            }
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let mut len = 0;
        let buf = if args_has(args, 'P') {
            cmd_capture_pane_pending(args, wp, &raw mut len)
        } else {
            cmd_capture_pane_history(args, item, wp, &raw mut len)
        };
        if buf.is_null() {
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if args_has(args, 'p') {
            if len > 0 && *buf.add(len - 1) == b'\n' {
                len -= 1;
            }
            if (*c).flags.intersects(client_flag::CONTROL) {
                control_write!(c, "{1:0$}", len, _s(buf));
            } else {
                if !file_can_print(c) {
                    cmdq_error!(item, "can't write to client");
                    free_(buf);
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                file_print_buffer(c, buf as _, len);
                file_print!(c, "\n");
                free_(buf);
            }
        } else {
            let mut bufname = None;
            let mut cause = null_mut();
            if args_has(args, 'b') {
                bufname = cstr_to_str_(args_get(args, b'b'));
            }

            if paste_set(buf, len, bufname, &raw mut cause) != 0 {
                cmdq_error!(item, "{}", _s(cause));
                free_(cause);
                free_(buf);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
