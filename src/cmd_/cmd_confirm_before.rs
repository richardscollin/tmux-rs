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
    name: SyncCharPtr::new(c"confirm-before"),
    alias: SyncCharPtr::new(c"confirm"),

    args: args_parse::new(c"bc:p:t:y", 1, 1, Some(cmd_confirm_before_args_parse)),
    usage: SyncCharPtr::new(c"[-by] [-c confirm_key] [-p prompt] [-t target-pane] command"),

    flags: cmd_flag::CMD_CLIENT_TFLAG,
    exec: cmd_confirm_before_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

pub struct cmd_confirm_before_data {
    item: *mut cmdq_item,
    cmdlist: *mut cmd_list,
    confirm_key: c_uchar,
    default_yes: i32,
}

unsafe fn cmd_confirm_before_args_parse(_: *mut args, _: u32, _: *mut *mut u8) -> args_parse_type {
    args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING
}

unsafe fn cmd_confirm_before_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);
        let target = cmdq_get_target(item);
        let mut new_prompt = null_mut();
        let wait = !args_has(args, b'b');

        let cdata = xcalloc_::<cmd_confirm_before_data>(1).as_ptr();
        (*cdata).cmdlist = args_make_commands_now(self_, item, 0, 1);
        if (*cdata).cmdlist.is_null() {
            free_(cdata);
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if wait != 0 {
            (*cdata).item = item;
        }

        (*cdata).default_yes = args_has(args, b'y');
        let confirm_key = args_get(args, b'c');
        if !confirm_key.is_null() {
            if *confirm_key.add(1) == b'\0' as _ && *confirm_key > 31 && *confirm_key < 127 {
                (*cdata).confirm_key = *confirm_key as _;
            } else {
                cmdq_error!(item, "invalid confirm key");
                free_(cdata);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        } else {
            (*cdata).confirm_key = b'y';
        }

        let prompt = args_get(args, b'p');
        if !prompt.is_null() {
            new_prompt = format_nul!("{} ", _s(prompt));
        } else {
            let cmd = cmd_get_entry(cmd_list_first((*cdata).cmdlist))
                .name
                .as_ptr();
            new_prompt = format_nul!(
                "Confirm '{}'? ({}/n) ",
                _s(cmd),
                (*cdata).confirm_key as char
            );
        }

        status_prompt_set(
            tc,
            target,
            new_prompt,
            null_mut(),
            Some(cmd_confirm_before_callback),
            Some(cmd_confirm_before_free),
            cdata as _,
            PROMPT_SINGLE,
            prompt_type::PROMPT_TYPE_COMMAND,
        );
        free_(new_prompt);

        if wait == 0 {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        cmd_retval::CMD_RETURN_WAIT
    }
}

unsafe fn cmd_confirm_before_callback(
    c: *mut client,
    data: NonNull<c_void>,
    s: *const u8,
    _done: i32,
) -> i32 {
    unsafe {
        let cdata: NonNull<cmd_confirm_before_data> = data.cast();
        let item = (*cdata.as_ptr()).item;
        let mut retcode: i32 = 1;

        'out: {
            if (*c).flags.intersects(client_flag::DEAD) {
                break 'out;
            }

            if s.is_null() {
                break 'out;
            }
            if *s != (*cdata.as_ptr()).confirm_key as _
                && (*s != b'\0' as _ || (*cdata.as_ptr()).default_yes == 0)
            {
                break 'out;
            }
            retcode = 0;

            let mut new_item = null_mut();
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

unsafe fn cmd_confirm_before_free(data: NonNull<c_void>) {
    unsafe {
        let cdata: NonNull<cmd_confirm_before_data> = data.cast();
        cmd_list_free((*cdata.as_ptr()).cmdlist);
        free_(cdata.as_ptr());
    }
}
