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
use crate::compat::queue::tailq_foreach;
use crate::*;

pub static CMD_DISPLAY_PANES_ENTRY: cmd_entry = cmd_entry {
    name: "display-panes",
    alias: Some("displayp"),

    args: args_parse::new("bd:Nt:", 0, 1, Some(cmd_display_panes_args_parse)),
    usage: "[-bN] [-d duration] [-t target-client] [template]",

    flags: cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_TFLAG),
    exec: cmd_display_panes_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

#[repr(C)]
pub struct cmd_display_panes_data<'a> {
    pub item: *mut cmdq_item,
    pub state: *mut args_command_state<'a>,
}

fn cmd_display_panes_args_parse(_: *mut args, _: u32, _: *mut *mut u8) -> args_parse_type {
    args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING
}

unsafe fn cmd_display_panes_draw_pane(ctx: *mut screen_redraw_ctx, wp: *mut window_pane) {
    unsafe {
        let c = (*ctx).c;
        let tty = &raw mut (*c).tty;
        let s = (*c).session;
        let oo = (*s).options;
        let w = (*wp).window;
        // u_int			 pane, idx, px, py, i, j, xoff, yoff, sx, sy;
        // int			 colour, active_colour;
        // char			 buf[16], lbuf[16], rbuf[16], *ptr;
        // size_t			 len, llen, rlen;

        'out: {
            if (*wp).xoff + (*wp).sx <= (*ctx).ox
                || (*wp).xoff >= (*ctx).ox + (*ctx).sx
                || (*wp).yoff + (*wp).sy <= (*ctx).oy
                || (*wp).yoff >= (*ctx).oy + (*ctx).sy
            {
                return;
            }

            let mut yoff: u32;
            let sy: u32;

            let xoff: u32;
            let sx: u32;
            if (*wp).xoff >= (*ctx).ox && (*wp).xoff + (*wp).sx <= (*ctx).ox + (*ctx).sx {
                // All visible.
                xoff = (*wp).xoff - (*ctx).ox;
                sx = (*wp).sx;
            } else if (*wp).xoff < (*ctx).ox && (*wp).xoff + (*wp).sx > (*ctx).ox + (*ctx).sx {
                // Both left and right not visible.
                xoff = 0;
                sx = (*ctx).sx;
            } else if (*wp).xoff < (*ctx).ox {
                // Left not visible.
                xoff = 0;
                sx = (*wp).sx - ((*ctx).ox - (*wp).xoff);
            } else {
                // Right not visible.
                xoff = (*wp).xoff - (*ctx).ox;
                sx = (*wp).sx - xoff;
            }
            if (*wp).yoff >= (*ctx).oy && (*wp).yoff + (*wp).sy <= (*ctx).oy + (*ctx).sy {
                // All visible.
                yoff = (*wp).yoff - (*ctx).oy;
                sy = (*wp).sy;
            } else if (*wp).yoff < (*ctx).oy && (*wp).yoff + (*wp).sy > (*ctx).oy + (*ctx).sy {
                // Both top and bottom not visible.
                yoff = 0;
                sy = (*ctx).sy;
            } else if (*wp).yoff < (*ctx).oy {
                // Top not visible.
                yoff = 0;
                sy = (*wp).sy - ((*ctx).oy - (*wp).yoff);
            } else {
                // Bottom not visible.
                yoff = (*wp).yoff - (*ctx).oy;
                sy = (*wp).sy - yoff;
            }

            if (*ctx).statustop != 0 {
                yoff += (*ctx).statuslines;
            }
            let mut px = sx / 2;
            let mut py = sy / 2;

            let mut pane = 0;
            if window_pane_index(wp, &raw mut pane) != 0 {
                fatalx("index not found");
            }
            let mut buf = [0i8; 16];
            let bufsize = 16;
            let mut len: usize = xsnprintf_!(&raw mut buf as _, bufsize, "{}", pane).unwrap() as _;

            if (sx as usize) < len {
                return;
            }

            let colour: i32 = options_get_number_(oo, "display-panes-colour") as _;
            let active_colour: i32 = options_get_number_(oo, "display-panes-active-colour") as _;

            let mut fgc = GRID_DEFAULT_CELL;
            let mut bgc = GRID_DEFAULT_CELL;
            if (*w).active == wp {
                fgc.fg = active_colour;
                bgc.bg = active_colour;
            } else {
                fgc.fg = colour;
                bgc.bg = colour;
            }

            let mut rbuf = [0i8; 16];
            let mut lbuf = [0i8; 16];
            let rlen: usize =
                xsnprintf_!(&raw mut rbuf as _, bufsize, "{}x{}", (*wp).sx, (*wp).sy).unwrap() as _;
            let llen: usize = if pane > 9 && pane < 35 {
                xsnprintf_!(
                    &raw mut lbuf as _,
                    bufsize,
                    "{}",
                    (b'a' as u32 + (pane - 10)) as u8 as char,
                )
                .unwrap() as _
            } else {
                0
            };

            if (sx as usize) < len * 6 || sy < 5 {
                tty_attributes(
                    tty,
                    &raw mut fgc,
                    &raw const GRID_DEFAULT_CELL,
                    null_mut(),
                    null_mut(),
                );
                #[expect(clippy::int_plus_one)]
                if (sx as usize) >= len + llen + 1 {
                    len += llen + 1;
                    tty_cursor(tty, xoff + px - (len / 2) as u32, yoff + py);
                    tty_putn(tty, &raw mut buf as _, len, len as _);
                    tty_putn(tty, c!(" ").cast(), 1, 1);
                    tty_putn(tty, &raw mut lbuf as _, llen, llen as _);
                } else {
                    tty_cursor(tty, xoff + px - (len / 2) as u32, yoff + py);
                    tty_putn(tty, &raw mut buf as _, len, len as _);
                }
                break 'out;
            }

            px -= (len * 3) as u32;
            py -= 2;

            tty_attributes(
                tty,
                &raw mut bgc,
                &raw const GRID_DEFAULT_CELL,
                null_mut(),
                null_mut(),
            );
            let mut ptr = &raw mut buf as *mut u8;
            while *ptr != b'\0' {
                if *ptr < b'0' || *ptr > b'9' {
                    ptr = ptr.add(1);
                    continue;
                }
                let idx = *ptr - b'0';

                for j in 0..5 {
                    let mut i = px;
                    while i < px + 5 {
                        tty_cursor(tty, xoff + i, yoff + py + j);
                        if WINDOW_CLOCK_TABLE[idx as usize][j as usize][(i - px) as usize] != 0 {
                            tty_putc(tty, b' ');
                        }
                        i += 1;
                    }
                }
                px += 6;
                ptr = ptr.add(1);
            }

            if sy <= 6 {
                break 'out;
            }
            tty_attributes(
                tty,
                &raw mut fgc,
                &raw const GRID_DEFAULT_CELL,
                null_mut(),
                null_mut(),
            );
            if rlen != 0 && sx as usize >= rlen {
                tty_cursor(tty, xoff + sx - rlen as u32, yoff);
                tty_putn(tty, &raw mut rbuf as _, rlen, rlen as _);
            }
            if llen != 0 {
                tty_cursor(
                    tty,
                    xoff + sx / 2 + len as u32 * 3 - llen as u32 - 1,
                    yoff + py + 5,
                );
                tty_putn(tty, &raw mut lbuf as _, llen, llen as _);
            }
        }

        // out:
        tty_cursor(tty, 0, 0);
    }
}

unsafe fn cmd_display_panes_draw(c: *mut client, _data: *mut c_void, ctx: *mut screen_redraw_ctx) {
    unsafe {
        let w: *mut window = (*(*(*c).session).curw).window;

        log_debug!(
            "{}: {} @{}",
            "cmd_display_panes_draw",
            _s((*c).name),
            (*w).id
        );

        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            if window_pane_visible(wp) {
                cmd_display_panes_draw_pane(ctx, wp);
            }
        }
    }
}

unsafe fn cmd_display_panes_free(_c: *mut client, data: *mut c_void) {
    unsafe {
        let cdata = data as *mut cmd_display_panes_data;

        if !(*cdata).item.is_null() {
            cmdq_continue((*cdata).item);
        }
        args_make_commands_free((*cdata).state);
        free_(cdata);
    }
}

unsafe fn cmd_display_panes_key(c: *mut client, data: *mut c_void, event: *mut key_event) -> i32 {
    unsafe {
        let cdata = data as *mut cmd_display_panes_data;
        let item = (*cdata).item;
        let w = (*(*(*c).session).curw).window;

        let index: u32;
        let key: key_code;

        if (*event).key >= b'0' as _ && (*event).key <= b'9' as _ {
            index = ((*event).key - b'0' as u64) as u32;
        } else if ((*event).key & KEYC_MASK_MODIFIERS) == 0 {
            key = (*event).key & KEYC_MASK_KEY;
            if key >= b'a' as _ && key <= b'z' as _ {
                index = 10 + (key as u32 - b'a' as u32);
            } else {
                return -1;
            }
        } else {
            return -1;
        }

        let wp = window_pane_at_index(w, index);
        if wp.is_null() {
            return 1;
        }
        window_unzoom(w, 1);

        let mut expanded = format_nul!("%{}", (*wp).id);
        let mut error = null_mut();
        let cmdlist = args_make_commands((*cdata).state, 1, &raw mut expanded, &raw mut error);
        if cmdlist.is_null() {
            cmdq_append(c, cmdq_get_error(error).as_ptr());
            free_(error);
        } else if item.is_null() {
            let new_item = cmdq_get_command(cmdlist, null_mut());
            cmdq_append(c, new_item);
        } else {
            let new_item = cmdq_get_command(cmdlist, cmdq_get_state(item));
            cmdq_insert_after(item, new_item);
        }

        free_(expanded);
        1
    }
}

unsafe fn cmd_display_panes_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);
        let s = (*tc).session;
        let delay: u32;
        let mut cause = null_mut();
        let wait = !args_has(args, 'b');

        if (*tc).overlay_draw.is_some() {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has(args, 'd') {
            delay = args_strtonum(args, b'd', 0, u32::MAX as i64, &raw mut cause) as u32;
            if !cause.is_null() {
                cmdq_error!(item, "delay {}", _s(cause));
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        } else {
            delay = options_get_number_((*s).options, "display-panes-time") as u32;
        }

        let cdata = xcalloc_::<cmd_display_panes_data>(1).as_ptr();
        if wait {
            (*cdata).item = item;
        }
        (*cdata).state =
            args_make_commands_prepare(self_, item, 0, c!("select-pane -t \"%%%\""), wait, false);

        if args_has(args, 'N') {
            server_client_set_overlay(
                tc,
                delay,
                None,
                None,
                Some(cmd_display_panes_draw),
                None,
                Some(cmd_display_panes_free),
                None,
                cdata as _,
            );
        } else {
            server_client_set_overlay(
                tc,
                delay,
                None,
                None,
                Some(cmd_display_panes_draw),
                Some(cmd_display_panes_key),
                Some(cmd_display_panes_free),
                None,
                cdata as _,
            );
        }

        if !wait {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        cmd_retval::CMD_RETURN_WAIT
    }
}
