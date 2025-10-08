// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
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

pub static CMD_COMMAND_PROMPT_ENTRY: cmd_entry = cmd_entry {
    name: "command-prompt",
    alias: None,

    args: args_parse::new("1bFkiI:Np:t:T:", 0, 1, Some(cmd_command_prompt_args_parse)),
    usage: "[-1bFkiN] [-I inputs] [-p prompts] [-t target-pane] [-T type] [template]",

    flags: cmd_flag::CMD_CLIENT_TFLAG,
    exec: cmd_command_prompt_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

struct cmd_command_prompt_prompt {
    input: *mut u8,
    prompt: *mut u8,
}

pub struct cmd_command_prompt_cdata<'a> {
    item: *mut cmdq_item,
    state: *mut args_command_state<'a>,

    flags: prompt_flags,
    prompt_type: prompt_type,

    prompts: *mut cmd_command_prompt_prompt,
    count: u32,
    current: u32,

    argc: i32,
    argv: *mut *mut u8,
}

fn cmd_command_prompt_args_parse(
    _args: *mut args,
    _idx: u32,
    _cause: *mut *mut u8,
) -> args_parse_type {
    args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING
}

unsafe fn cmd_command_prompt_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);
        let target = cmdq_get_target(item);
        let prompts;
        let mut prompt: *const u8;
        let mut next_prompt;
        let mut tmp;
        let mut inputs = null_mut();
        let mut next_input;
        let count = args_count(args);
        let mut wait = !args_has(args, 'b');
        let mut space = 1;

        if !(*tc).prompt_string.is_null() {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if args_has(args, 'i') {
            wait = false;
        }

        let mut cdata: Box<cmd_command_prompt_cdata> = Box::default();
        if wait {
            cdata.item = item;
        }
        cdata.state =
            args_make_commands_prepare(self_, item, 0, c!("%1"), wait, args_has(args, 'F'));

        let mut s = args_get(args, b'p');
        if s.is_null() {
            if count != 0 {
                let tmp = args_make_commands_get_command(cdata.state);
                prompts = format_nul!("({})", _s(tmp));
                free_(tmp);
            } else {
                prompts = xstrdup_(c":").as_ptr();
                space = 0;
            }
            next_prompt = prompts;
        } else {
            prompts = xstrdup(s).as_ptr();
            next_prompt = prompts;
        }
        s = args_get(args, b'I');
        if !s.is_null() {
            inputs = xstrdup(s).as_ptr();
            next_input = inputs;
        } else {
            next_input = null_mut();
        }
        while {
            prompt = strsep(&raw mut next_prompt as _, c!(","));
            !prompt.is_null()
        } {
            cdata.prompts = xreallocarray_::<cmd_command_prompt_prompt>(
                cdata.prompts,
                cdata.count as usize + 1,
            )
            .as_ptr();
            tmp = if space == 0 {
                xstrdup(prompt).as_ptr()
            } else {
                format_nul!("{} ", _s(prompt))
            };
            (*cdata.prompts.add(cdata.count as usize)).prompt = tmp;

            let mut input: *const u8;
            if !next_input.is_null() {
                input = strsep(&raw mut next_input as _, c!(","));
                if input.is_null() {
                    input = c!("");
                }
            } else {
                input = c!("");
            }
            (*cdata.prompts.add(cdata.count as usize)).input = xstrdup(input).as_ptr();

            cdata.count += 1;
        }
        free_(inputs);
        free_(prompts);

        let type_ = args_get(args, b'T');
        if !type_.is_null() {
            cdata.prompt_type = status_prompt_type(type_);
            if cdata.prompt_type == prompt_type::PROMPT_TYPE_INVALID {
                cmdq_error!(item, "unknown type: {}", _s(type_));
                cmd_command_prompt_free(NonNull::new_unchecked(Box::into_raw(cdata)));
                return cmd_retval::CMD_RETURN_ERROR;
            }
        } else {
            cdata.prompt_type = prompt_type::PROMPT_TYPE_COMMAND;
        }

        if args_has(args, '1') {
            cdata.flags |= prompt_flags::PROMPT_SINGLE;
        } else if args_has(args, 'N') {
            cdata.flags |= prompt_flags::PROMPT_NUMERIC;
        } else if args_has(args, 'i') {
            cdata.flags |= prompt_flags::PROMPT_INCREMENTAL;
        } else if args_has(args, 'k') {
            cdata.flags |= prompt_flags::PROMPT_KEY;
        }

        let flags = cdata.flags;
        let prompt_type = cdata.prompt_type;
        status_prompt_set(
            tc,
            target,
            (*cdata.prompts).prompt,
            (*cdata.prompts).input,
            cmd_command_prompt_callback,
            cmd_command_prompt_free,
            Box::into_raw(cdata),
            flags,
            prompt_type,
        );

        if !wait {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        cmd_retval::CMD_RETURN_WAIT
    }
}

unsafe fn cmd_command_prompt_callback(
    c: *mut client,
    cdata: NonNull<cmd_command_prompt_cdata>,
    s: *const u8,
    done: i32,
) -> i32 {
    unsafe {
        let cdata = cdata.as_ptr();
        let mut error: *mut u8 = null_mut();
        let item: *mut cmdq_item = (*cdata).item;

        'out: {
            if s.is_null() {
                break 'out;
            }

            if done != 0 {
                if (*cdata).flags.intersects(prompt_flags::PROMPT_INCREMENTAL) {
                    break 'out;
                }
                cmd_append_argv(&raw mut (*cdata).argc, &raw mut (*cdata).argv, s);
                (*cdata).current += 1;
                if (*cdata).current != (*cdata).count {
                    let prompt = (*cdata).prompts.add((*cdata).current as usize);
                    status_prompt_update(c, (*prompt).prompt, (*prompt).input);
                    return 1;
                }
            }

            let mut argc = (*cdata).argc;
            let mut argv = cmd_copy_argv((*cdata).argc, (*cdata).argv);
            if done == 0 {
                cmd_append_argv(&raw mut argc, &raw mut argv, s);
            }

            if done != 0 {
                cmd_free_argv((*cdata).argc, (*cdata).argv);
                (*cdata).argc = argc;
                (*cdata).argv = cmd_copy_argv(argc, argv);
            }

            let cmdlist = args_make_commands((*cdata).state, argc, argv, &raw mut error);
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
            cmd_free_argv(argc, argv);

            // Check for intervening call to status_prompt_set()
            if (*c).prompt_data != cdata.cast() {
                return 1;
            }

            break 'out;
        }
        // out:
        if !item.is_null() {
            cmdq_continue(item);
        }
        0
    }
}

unsafe fn cmd_command_prompt_free(cdata: NonNull<cmd_command_prompt_cdata>) {
    unsafe {
        for i in 0u32..(*cdata.as_ptr()).count {
            free_((*(*cdata.as_ptr()).prompts.add(i as usize)).prompt);
            free_((*(*cdata.as_ptr()).prompts.add(i as usize)).input);
        }
        free_((*cdata.as_ptr()).prompts);
        cmd_free_argv((*cdata.as_ptr()).argc, (*cdata.as_ptr()).argv);
        args_make_commands_free((*cdata.as_ptr()).state);
        free_(cdata.as_ptr());
    }
}
