// Copyright (c) 2009 Tiago Cunha <me@tiagocunha.org>
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

pub static CMD_CONFIRM_BEFORE_ENTRY: cmd_entry = cmd_entry {
    name: "confirm-before",
    alias: Some("confirm"),

    args: args_parse::new(c"bc:p:t:y", 1, 1, Some(cmd_confirm_before_args_parse)),
    usage: "[-by] [-c confirm_key] [-p prompt] [-t target-pane] command",

    flags: cmd_flag::CMD_CLIENT_TFLAG,
    exec: cmd_confirm_before_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

#[derive(Default)]
pub struct cmd_confirm_before_data {
    item: *mut cmdq_item,
    cmdlist: *mut cmd_list,
    confirm_key: u8,
    default_yes: bool,
}

unsafe fn cmd_confirm_before_args_parse(_: *mut args, _: u32, _: *mut *mut u8) -> args_parse_type {
    args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING
}

unsafe fn cmd_confirm_before_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);
        let target = cmdq_get_target(item);
        let wait = !args_has(args, 'b');

        let mut cdata: Box<cmd_confirm_before_data> = Box::default();
        cdata.cmdlist = args_make_commands_now(self_, item, 0, true);
        if cdata.cmdlist.is_null() {
            // free_(cdata);
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if wait {
            cdata.item = item;
        }

        cdata.default_yes = args_has(args, 'y');
        let confirm_key = args_get(args, b'c');
        if !confirm_key.is_null() {
            if *confirm_key.add(1) == b'\0' && *confirm_key > 31 && *confirm_key < 127 {
                cdata.confirm_key = *confirm_key as _;
            } else {
                cmdq_error!(item, "invalid confirm key");
                // free_(cdata);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        } else {
            cdata.confirm_key = b'y';
        }

        let prompt = args_get(args, b'p');
        let new_prompt = if !prompt.is_null() {
            format_nul!("{} ", _s(prompt))
        } else {
            let cmd = cmd_get_entry(cmd_list_first(cdata.cmdlist)).name;
            format_nul!("Confirm '{}'? ({}/n) ", cmd, cdata.confirm_key as char)
        };

        status_prompt_set(
            tc,
            target,
            new_prompt,
            null_mut(),
            cmd_confirm_before_callback,
            cmd_confirm_before_free,
            Box::into_raw(cdata),
            prompt_flags::PROMPT_SINGLE,
            prompt_type::PROMPT_TYPE_COMMAND,
        );
        free_(new_prompt);

        if !wait {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        cmd_retval::CMD_RETURN_WAIT
    }
}

unsafe fn cmd_confirm_before_callback(
    c: *mut client,
    cdata: NonNull<cmd_confirm_before_data>,
    s: *const u8,
    _done: i32,
) -> i32 {
    unsafe {
        let item = (*cdata.as_ptr()).item;
        let mut retcode: i32 = 1;

        'out: {
            if (*c).flags.intersects(client_flag::DEAD) {
                break 'out;
            }

            if s.is_null() {
                break 'out;
            }
            if *s != (*cdata.as_ptr()).confirm_key
                && (*s != b'\0' || !(*cdata.as_ptr()).default_yes)
            {
                break 'out;
            }
            retcode = 0;

            let new_item;
            if item.is_null() {
                new_item = cmdq_get_command((*cdata.as_ptr()).cmdlist, null_mut());
                cmdq_append(c, new_item);
            } else {
                new_item = cmdq_get_command((*cdata.as_ptr()).cmdlist, cmdq_get_state(item));
                cmdq_insert_after(item, new_item);
            }
        }

        // out:
        if !item.is_null() {
            if !cmdq_get_client(item).is_null() && (*cmdq_get_client(item)).session.is_null() {
                (*cmdq_get_client(item)).retval = retcode;
            }
            cmdq_continue(item);
        }
        0
    }
}

unsafe fn cmd_confirm_before_free(cdata: NonNull<cmd_confirm_before_data>) {
    unsafe {
        cmd_list_free((*cdata.as_ptr()).cmdlist);
        free_(cdata.as_ptr());
    }
}
