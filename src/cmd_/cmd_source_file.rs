// Copyright (c) 2008 Tiago Cunha <me@tiagocunha.org>
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
use crate::libc::{EINVAL, ENOENT, ENOMEM, GLOB_NOMATCH, GLOB_NOSPACE, glob, glob_t, globfree};
use crate::*;

pub static CMD_SOURCE_FILE_ENTRY: cmd_entry = cmd_entry {
    name: SyncCharPtr::new(c"source-file"),
    alias: SyncCharPtr::new(c"source"),

    args: args_parse::new(c"t:Fnqv", 1, -1, None),
    usage: SyncCharPtr::new(c"[-Fnqv] [-t target-pane] path ..."),

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_PANE,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::empty(),
    exec: cmd_source_file_exec,
    source: cmd_entry_flag::zeroed(),
};

#[repr(C)]
pub struct cmd_source_file_data {
    pub item: *mut cmdq_item,
    pub flags: cmd_parse_input_flags,

    pub after: *mut cmdq_item,
    pub retval: cmd_retval,

    pub current: u32,
    pub files: *mut *mut u8,
    pub nfiles: u32,
}

unsafe fn cmd_source_file_complete_cb(item: *mut cmdq_item, _data: *mut c_void) -> cmd_retval {
    unsafe {
        cfg_print_causes(item);
        cmd_retval::CMD_RETURN_NORMAL
    }
}

unsafe fn cmd_source_file_complete(c: *mut client, cdata: *mut cmd_source_file_data) {
    unsafe {
        if CFG_FINISHED.load(atomic::Ordering::Acquire) {
            if (*cdata).retval == cmd_retval::CMD_RETURN_ERROR
                && !c.is_null()
                && (*c).session.is_null()
            {
                (*c).retval = 1;
            }
            let new_item = cmdq_get_callback!(cmd_source_file_complete_cb, null_mut()).as_ptr();
            cmdq_insert_after((*cdata).after, new_item);
        }

        for i in 0..(*cdata).nfiles {
            free_(*(*cdata).files.add(i as usize));
        }
        free_((*cdata).files);
        free_(cdata);
    }
}

unsafe fn cmd_source_file_done(
    c: *mut client,
    path: *mut u8,
    error: i32,
    closed: i32,
    buffer: *mut evbuffer,
    data: *mut c_void,
) {
    unsafe {
        let cdata = data as *mut cmd_source_file_data;
        let item = (*cdata).item;
        let bdata = EVBUFFER_DATA(buffer);
        let bsize = EVBUFFER_LENGTH(buffer);
        let mut new_item: *mut cmdq_item = null_mut();
        let target = cmdq_get_target(item);

        if closed == 0 {
            return;
        }

        if error != 0 {
            cmdq_error!(item, "{}: {}", _s(path), _s(strerror(error)));
        } else if bsize != 0 {
            if load_cfg_from_buffer(
                std::slice::from_raw_parts(bdata, bsize),
                cstr_to_str(path),
                c,
                (*cdata).after,
                target,
                (*cdata).flags,
                &raw mut new_item,
            ) < 0
            {
                (*cdata).retval = cmd_retval::CMD_RETURN_ERROR;
            } else if !new_item.is_null() {
                (*cdata).after = new_item;
            }
        }

        (*cdata).current += 1;
        let n = (*cdata).current;
        if n < (*cdata).nfiles {
            file_read(
                c,
                *(*cdata).files.add(n as usize),
                Some(cmd_source_file_done),
                cdata.cast(),
            );
        } else {
            cmd_source_file_complete(c, cdata);
            cmdq_continue(item);
        }
    }
}

unsafe fn cmd_source_file_add(cdata: *mut cmd_source_file_data, path: *const u8) {
    unsafe {
        log_debug!("cmd_source_file_add: {}", _s(path));
        (*cdata).files = xreallocarray_((*cdata).files, ((*cdata).nfiles + 1) as usize).as_ptr();
        *(*cdata).files.add((*cdata).nfiles as usize) = xstrdup(path).as_ptr();
        (*cdata).nfiles += 1;
    }
}

unsafe fn cmd_source_file_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    let __func__ = "cmd_source_file_exec";

    unsafe {
        let args = cmd_get_args(self_);
        let c = cmdq_get_client(item);
        let mut retval = cmd_retval::CMD_RETURN_NORMAL;
        let mut cwd = null_mut();
        let mut error: *mut u8;
        let mut g = MaybeUninit::<glob_t>::uninit();
        let mut result;

        let cdata = xcalloc_::<cmd_source_file_data>(1).as_ptr();
        (*cdata).item = item;

        if args_has(args, 'q') {
            (*cdata).flags |= cmd_parse_input_flags::CMD_PARSE_QUIET;
        }
        if args_has(args, 'n') {
            (*cdata).flags |= cmd_parse_input_flags::CMD_PARSE_PARSEONLY;
        }
        if args_has(args, 'v') {
            (*cdata).flags |= cmd_parse_input_flags::CMD_PARSE_VERBOSE;
        }

        utf8_stravis(
            &raw mut cwd,
            server_client_get_cwd(c, null_mut()),
            vis_flags::VIS_GLOB,
        );

        for i in 0..args_count(args) {
            let mut path = args_string_(args, i).unwrap();
            let expanded;
            if args_has(args, 'F') {
                // TODO: note path is backed by nul terminated string
                expanded = format_single_from_target(item, path.as_ptr().cast());
                path = expanded.as_str();
            }
            if path == "-" {
                cmd_source_file_add(cdata, c!("-"));
                continue;
            }

            let mut pattern = if path.starts_with('/') {
                path.to_string()
            } else {
                format!("{}/{}", _s(cwd), path)
            };
            log_debug!("{}: {}", __func__, pattern);

            nul_terminate(&mut pattern);
            result = glob(pattern.as_ptr(), 0, None, g.as_mut_ptr());
            if result != 0 {
                if result != GLOB_NOMATCH
                    || !(*cdata)
                        .flags
                        .intersects(cmd_parse_input_flags::CMD_PARSE_QUIET)
                {
                    if result == GLOB_NOMATCH {
                        error = strerror(ENOENT);
                    } else if result == GLOB_NOSPACE {
                        error = strerror(ENOMEM);
                    } else {
                        error = strerror(EINVAL);
                    }
                    cmdq_error!(item, "{}: {}", path, _s(error));
                    retval = cmd_retval::CMD_RETURN_ERROR;
                }
                globfree(g.as_mut_ptr());
                continue;
            }

            for j in 0..(*g.as_ptr()).gl_pathc {
                cmd_source_file_add(cdata, *(*g.as_ptr()).gl_pathv.add(j).cast());
            }
            globfree(g.as_mut_ptr());
        }

        (*cdata).after = item;
        (*cdata).retval = retval;

        if (*cdata).nfiles != 0 {
            file_read(c, *(*cdata).files, Some(cmd_source_file_done), cdata as _);
            retval = cmd_retval::CMD_RETURN_WAIT;
        } else {
            cmd_source_file_complete(c, cdata);
        }

        free_(cwd);
        retval
    }
}
