// Copyright (c) 2021 Anindya Mukherjee <anindya49@hotmail.com>
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

pub static mut cmd_show_prompt_history_entry: cmd_entry = cmd_entry {
    name: c"show-prompt-history".as_ptr(),
    alias: c"showphist".as_ptr(),

    args: args_parse::new(c"T:", 0, 0, None),
    usage: c"[-T type]".as_ptr(),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_show_prompt_history_exec),
    ..unsafe { zeroed() }
};

pub static mut cmd_clear_prompt_history_entry: cmd_entry = cmd_entry {
    name: c"clear-prompt-history".as_ptr(),
    alias: c"clearphist".as_ptr(),

    args: args_parse::new(c"T:", 0, 0, None),
    usage: c"[-T type]".as_ptr(),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_show_prompt_history_exec),
    ..unsafe { zeroed() }
};

unsafe fn cmd_show_prompt_history_exec(
    self_: *mut cmd,
    item: *mut cmdq_item,
) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let typestr = args_get(args, b'T');
        let type_: prompt_type;

        if cmd_get_entry(self_) == &raw mut cmd_clear_prompt_history_entry {
            if typestr.is_null() {
                for tidx in 0..PROMPT_NTYPES {
                    free_(status_prompt_hlist[tidx as usize]);
                    status_prompt_hlist[tidx as usize] = null_mut();
                    status_prompt_hsize[tidx as usize] = 0;
                }
            } else {
                type_ = status_prompt_type(typestr);
                if type_ == prompt_type::PROMPT_TYPE_INVALID {
                    cmdq_error!(item, "invalid type: {}", _s(typestr));
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                free_(status_prompt_hlist[type_ as usize]);
                status_prompt_hlist[type_ as usize] = null_mut();
                status_prompt_hsize[type_ as usize] = 0;
            }

            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if typestr.is_null() {
            for tidx in 0..PROMPT_NTYPES {
                cmdq_print!(
                    item,
                    "History for {}:\n",
                    _s(status_prompt_type_string(tidx)),
                );
                for hidx in 0u32..status_prompt_hsize[tidx as usize] {
                    cmdq_print!(
                        item,
                        "{}: {}",
                        hidx + 1,
                        _s(*status_prompt_hlist[tidx as usize].add(hidx as usize)),
                    );
                }
                cmdq_print!(item, "");
            }
        } else {
            type_ = status_prompt_type(typestr);
            if type_ == prompt_type::PROMPT_TYPE_INVALID {
                cmdq_error!(item, "invalid type: {}", _s(typestr));
                return cmd_retval::CMD_RETURN_ERROR;
            }
            cmdq_print!(
                item,
                "History for {}:\n",
                _s(status_prompt_type_string(type_ as u32)),
            );
            for hidx in 0u32..status_prompt_hsize[type_ as usize] {
                cmdq_print!(
                    item,
                    "{}: {}",
                    hidx + 1,
                    _s(*status_prompt_hlist[type_ as usize].add(hidx as usize)),
                );
            }
            cmdq_print!(item, "");
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
