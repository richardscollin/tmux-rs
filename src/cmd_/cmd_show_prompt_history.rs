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

pub static CMD_SHOW_PROMPT_HISTORY_ENTRY: cmd_entry = cmd_entry {
    name: "show-prompt-history",
    alias: Some("showphist"),

    args: args_parse::new(c"T:", 0, 0, None),
    usage: "[-T type]",

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_show_prompt_history_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

pub static CMD_CLEAR_PROMPT_HISTORY_ENTRY: cmd_entry = cmd_entry {
    name: "clear-prompt-history",
    alias: Some("clearphist"),

    args: args_parse::new(c"T:", 0, 0, None),
    usage: "[-T type]",

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_show_prompt_history_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_show_prompt_history_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let typestr = args_get(args, b'T');
        let type_: prompt_type;

        if std::ptr::eq(cmd_get_entry(self_), &CMD_CLEAR_PROMPT_HISTORY_ENTRY) {
            if typestr.is_null() {
                for tidx in 0..PROMPT_NTYPES {
                    free_(STATUS_PROMPT_HLIST[tidx as usize]);
                    STATUS_PROMPT_HLIST[tidx as usize] = null_mut();
                    STATUS_PROMPT_HSIZE[tidx as usize] = 0;
                }
            } else {
                type_ = status_prompt_type(typestr);
                if type_ == prompt_type::PROMPT_TYPE_INVALID {
                    cmdq_error!(item, "invalid type: {}", _s(typestr));
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                free_(STATUS_PROMPT_HLIST[type_ as usize]);
                STATUS_PROMPT_HLIST[type_ as usize] = null_mut();
                STATUS_PROMPT_HSIZE[type_ as usize] = 0;
            }

            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if typestr.is_null() {
            for tidx in 0..PROMPT_NTYPES {
                cmdq_print!(item, "History for {}:\n", status_prompt_type_string(tidx),);
                for hidx in 0u32..STATUS_PROMPT_HSIZE[tidx as usize] {
                    cmdq_print!(
                        item,
                        "{}: {}",
                        hidx + 1,
                        _s(*STATUS_PROMPT_HLIST[tidx as usize].add(hidx as usize)),
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
                status_prompt_type_string(type_ as u32),
            );
            for hidx in 0u32..STATUS_PROMPT_HSIZE[type_ as usize] {
                cmdq_print!(
                    item,
                    "{}: {}",
                    hidx + 1,
                    _s(*STATUS_PROMPT_HLIST[type_ as usize].add(hidx as usize)),
                );
            }
            cmdq_print!(item, "");
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
