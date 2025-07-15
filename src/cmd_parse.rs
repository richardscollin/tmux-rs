// Copyright (c) 2019 Nicholas Marriott <nicholas.marriott@gmail.com>
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
#![allow(clippy::uninlined_format_args)]
use crate::compat::S_ISDIR;
use crate::compat::fdforkpty::getptmfd;
use crate::tmux::getshell;
use crate::tmux::usage;
use crate::xmalloc::xstrndup;
// for lalrpop generated code
use crate::*;

use std::io::Read as _;
use std::ops::BitAndAssign as _;
use std::ops::BitOrAssign as _;
use std::sync::atomic::Ordering;

#[cfg(feature = "lalrpop")]
use lalrpop_util::lalrpop_mod;

use crate::cmd_parse_pratt;

use crate::compat::queue::{
    tailq_empty, tailq_first, tailq_foreach, tailq_init, tailq_insert_tail, tailq_last,
    tailq_remove,
};
use crate::xmalloc::xrecallocarray__;

fn yyparse(ps: &mut cmd_parse_state) -> Result<Option<&'static mut cmd_parse_commands>, ()> {
    #[cfg(feature = "lalrpop")]
    {
        yyparse_lalrpop(ps)
    }
    #[cfg(not(feature = "lalrpop"))]
    {
        yyparse_pratt(ps)
    }
}

#[cfg(feature = "lalrpop")]
fn yyparse_lalrpop(
    ps: &mut cmd_parse_state,
) -> Result<Option<&'static mut cmd_parse_commands>, ()> {
    log_debug!("yyparse_lalrpop");
    let mut parser = cmd_parse::LinesParser::new();

    let mut ps = NonNull::new(ps).unwrap();
    let mut lexer = lexer::Lexer::new(ps);

    match parser.parse(ps, lexer) {
        Ok(cmds) => Ok(cmds),
        Err(parse_err) => {
            log_debug!("parsing error {parse_err:?}");
            Err(())
        }
    }
}

fn yyparse_pratt(ps: &mut cmd_parse_state) -> Result<Option<&'static mut cmd_parse_commands>, ()> {
    log_debug!("yyparse_pratt");
    cmd_parse_pratt::parse_lines(ps)
}

#[cfg(feature = "lalrpop")]
lalrpop_mod!(cmd_parse);

pub struct yystype_elif {
    flag: i32,
    commands: &'static mut cmd_parse_commands,
}

crate::compat::impl_tailq_entry!(cmd_parse_scope, entry, tailq_entry<cmd_parse_scope>);
#[repr(C)]
pub struct cmd_parse_scope {
    pub flag: i32,
    // #[entry]
    pub entry: tailq_entry<cmd_parse_scope>,
}

#[repr(i32)]
pub enum cmd_parse_argument_type {
    /// string
    String(*mut u8),
    /// commands
    Commands(&'static mut cmd_parse_commands),
    /// cmdlist
    ParsedCommands(*mut cmd_list),
}

crate::compat::impl_tailq_entry!(cmd_parse_argument, entry, tailq_entry<cmd_parse_argument>);
#[repr(C)]
pub struct cmd_parse_argument {
    pub type_: cmd_parse_argument_type,

    // #[entry]
    pub entry: tailq_entry<cmd_parse_argument>,
}
pub type cmd_parse_arguments = tailq_head<cmd_parse_argument>;

crate::compat::impl_tailq_entry!(cmd_parse_command, entry, tailq_entry<cmd_parse_command>);
#[repr(C)]
pub struct cmd_parse_command {
    pub line: u32,
    pub arguments: cmd_parse_arguments,

    // #[entry]
    pub entry: tailq_entry<cmd_parse_command>,
}
pub type cmd_parse_commands = tailq_head<cmd_parse_command>;

#[repr(C)]
pub struct cmd_parse_state<'a> {
    pub f: Option<&'a mut std::io::BufReader<std::fs::File>>,
    pub unget_buf: Option<i32>,

    pub buf: Option<&'a [u8]>,
    pub off: usize,

    pub condition: i32,
    pub eol: i32,
    pub eof: i32,
    pub input: Option<&'a cmd_parse_input<'a>>,
    pub escapes: u32,

    pub error: *mut u8,

    pub scope: Option<&'a mut cmd_parse_scope>,
    pub stack: tailq_head<cmd_parse_scope>,
}

pub unsafe fn cmd_parse_get_error(file: Option<&str>, line: u32, error: &str) -> *mut u8 {
    match file {
        None => {
            let mut s = error.to_string();
            s.push('\0');
            s.leak().as_mut_ptr().cast()
        }
        Some(file) => format_nul!("{}:{}: {}", file, line, error),
    }
}

pub fn cmd_parse_print_commands(pi: &cmd_parse_input, cmdlist: &mut cmd_list) {
    if pi.item.is_null()
        || !pi
            .flags
            .intersects(cmd_parse_input_flags::CMD_PARSE_VERBOSE)
    {
        return;
    }

    let s = cmd_list_print(cmdlist, 0);

    unsafe {
        if let Some(file) = pi.file {
            cmdq_print!(
                pi.item,
                "{}:{}: {}",
                file,
                pi.line.load(Ordering::SeqCst),
                _s(s)
            );
        } else {
            cmdq_print!(pi.item, "{}: {}", pi.line.load(Ordering::SeqCst), _s(s));
        }
        free_(s)
    }
}

pub unsafe fn cmd_parse_free_argument(arg: *mut cmd_parse_argument) {
    unsafe {
        match &mut (*arg).type_ {
            cmd_parse_argument_type::String(string) => free_(*string),
            cmd_parse_argument_type::Commands(commands) => cmd_parse_free_commands(*commands),
            cmd_parse_argument_type::ParsedCommands(cmdlist) => cmd_list_free(*cmdlist),
        }
        free_(arg);
    }
}

pub unsafe fn cmd_parse_free_arguments(args: &mut cmd_parse_arguments) {
    unsafe {
        for arg in tailq_foreach(args).map(NonNull::as_ptr) {
            tailq_remove(args, arg);
            cmd_parse_free_argument(arg);
        }
    }
}

pub unsafe fn cmd_parse_free_command(cmd: *mut cmd_parse_command) {
    unsafe {
        cmd_parse_free_arguments(&mut (*cmd).arguments);
        free_(cmd);
    }
}

pub fn cmd_parse_new_commands() -> &'static mut cmd_parse_commands {
    unsafe {
        let cmds = Box::leak(Box::new(zeroed()));
        tailq_init(cmds);
        cmds
    }
}

pub unsafe fn cmd_parse_free_commands(cmds: *mut cmd_parse_commands) {
    unsafe {
        for cmd in tailq_foreach(cmds).map(NonNull::as_ptr) {
            tailq_remove(cmds, cmd);
            cmd_parse_free_command(cmd);
        }
        free_(cmds);
    }
}

pub unsafe fn cmd_parse_run_parser(
    ps: &mut cmd_parse_state,
) -> Result<&'static mut cmd_parse_commands, *mut u8> {
    unsafe {
        tailq_init(&mut ps.stack);

        let retval = yyparse(ps);
        for scope in tailq_foreach(&mut ps.stack).map(NonNull::as_ptr) {
            tailq_remove(&mut ps.stack, scope);
            free_(scope);
        }

        match retval {
            Ok(Some(cmds)) => Ok(cmds),
            Ok(None) => Ok(cmd_parse_new_commands()),
            Err(_) => Err(ps.error),
        }
    }
}

pub unsafe fn cmd_parse_run_parser_pratt(
    ps: &mut cmd_parse_state,
) -> Result<&'static mut cmd_parse_commands, *mut u8> {
    unsafe {
        tailq_init(&mut ps.stack);

        let retval = yyparse_pratt(ps);
        for scope in tailq_foreach(&mut ps.stack).map(NonNull::as_ptr) {
            tailq_remove(&mut ps.stack, scope);
            free_(scope);
        }

        match retval {
            Ok(Some(cmds)) => Ok(cmds),
            Ok(None) => Ok(cmd_parse_new_commands()),
            Err(_) => Err(ps.error),
        }
    }
}

pub unsafe fn cmd_parse_do_file<'a>(
    f: &'a mut std::io::BufReader<std::fs::File>,
    pi: &'a cmd_parse_input<'a>,
) -> Result<&'static mut cmd_parse_commands, *mut u8> {
    unsafe {
        let mut ps: Box<cmd_parse_state> = Box::new(zeroed());
        ps.input = Some(pi);
        ps.f = Some(f);
        cmd_parse_run_parser(&mut ps)
    }
}

pub unsafe fn cmd_parse_do_buffer<'a>(
    buf: &'a [u8],
    pi: &'a cmd_parse_input<'a>,
) -> Result<&'static mut cmd_parse_commands, *mut u8> {
    unsafe {
        let mut ps: Box<cmd_parse_state> = Box::new(zeroed());

        ps.input = Some(pi);
        ps.buf = Some(buf);
        cmd_parse_run_parser(&mut ps)
    }
}

pub unsafe fn cmd_parse_do_buffer_pratt<'a>(
    buf: &'a [u8],
    pi: &'a cmd_parse_input<'a>,
) -> Result<&'static mut cmd_parse_commands, *mut u8> {
    unsafe {
        let mut ps: Box<cmd_parse_state> = Box::new(zeroed());

        ps.input = Some(pi);
        ps.buf = Some(buf);
        cmd_parse_run_parser_pratt(&mut ps)
    }
}

pub unsafe fn cmd_parse_log_commands(cmds: *mut cmd_parse_commands, prefix: *const u8) {
    unsafe {
        for (i, cmd) in tailq_foreach(cmds).map(NonNull::as_ptr).enumerate() {
            for (j, arg) in tailq_foreach(&raw mut (*cmd).arguments)
                .map(NonNull::as_ptr)
                .enumerate()
            {
                match &mut (*arg).type_ {
                    cmd_parse_argument_type::String(string) => {
                        log_debug!("{} {}:{}: {}", _s(prefix), i, j, _s(*string))
                    }
                    cmd_parse_argument_type::Commands(commands) => {
                        let s = format_nul!("{} {}:{}", _s(prefix), i, j);
                        cmd_parse_log_commands(*commands, s);
                        free_(s);
                    }
                    cmd_parse_argument_type::ParsedCommands(cmdlist) => {
                        let s = cmd_list_print(&mut **cmdlist, 0);
                        log_debug!("{} {}:{}: {}", _s(prefix), i, j, _s(s));
                        free_(s);
                    }
                }
            }
        }
    }
}

pub unsafe fn cmd_parse_expand_alias<'a>(
    cmd: *mut cmd_parse_command,
    pi: &'a cmd_parse_input<'a>,
    pr: &mut cmd_parse_result,
) -> i32 {
    let __func__ = c!("cmd_parse_expand_alias");
    unsafe {
        if pi
            .flags
            .intersects(cmd_parse_input_flags::CMD_PARSE_NOALIAS)
        {
            return 0;
        }
        *pr = Err(null_mut());

        let first = tailq_first(&raw mut (*cmd).arguments);
        if first.is_null() || !matches!((*first).type_, cmd_parse_argument_type::String(_)) {
            *pr = Ok(cmd_list_new());
            return 1;
        }

        let name = match (*first).type_ {
            cmd_parse_argument_type::String(string) => string,
            _ => panic!(),
        };

        let alias = cmd_get_alias(name);
        if alias.is_null() {
            return 0;
        }
        log_debug!(
            "{}: {} alias {} = {}",
            _s(__func__),
            pi.line.load(Ordering::SeqCst),
            _s(name),
            _s(alias)
        );

        let result = cmd_parse_do_buffer(
            std::slice::from_raw_parts(alias.cast(), libc::strlen(alias)),
            pi,
        );
        free_(alias);
        let cmds = match result {
            Ok(cmds) => cmds,
            Err(cause) => {
                *pr = Err(cause);
                return 1;
            }
        };

        let last = tailq_last(cmds);
        if last.is_null() {
            *pr = Ok(cmd_list_new());
            return 1;
        }

        tailq_remove(&raw mut (*cmd).arguments, first);
        cmd_parse_free_argument(first);

        for arg in tailq_foreach(&raw mut (*cmd).arguments).map(NonNull::as_ptr) {
            tailq_remove(&raw mut (*cmd).arguments, arg);
            tailq_insert_tail(&raw mut (*last).arguments, arg);
        }
        cmd_parse_log_commands(cmds, __func__);

        (&pi.flags).bitor_assign(cmd_parse_input_flags::CMD_PARSE_NOALIAS);
        cmd_parse_build_commands(cmds, pi, pr);
        (&pi.flags).bitand_assign(!cmd_parse_input_flags::CMD_PARSE_NOALIAS);
        1
    }
}

pub unsafe fn cmd_parse_build_command(
    cmd: *mut cmd_parse_command,
    pi: &cmd_parse_input,
    pr: &mut cmd_parse_result,
) {
    unsafe {
        let mut values: *mut args_value = null_mut();
        let mut count: u32 = 0;
        let idx = 0u32;
        *pr = cmd_parse_result::Err(null_mut());

        if cmd_parse_expand_alias(cmd, pi, pr) != 0 {
            return;
        }

        'out: {
            for arg in tailq_foreach(&raw mut (*cmd).arguments).map(NonNull::as_ptr) {
                values = xrecallocarray__::<args_value>(values, count as usize, count as usize + 1)
                    .as_ptr();
                match &mut (*arg).type_ {
                    cmd_parse_argument_type::String(string) => {
                        (*values.add(count as usize)).type_ = args_type::ARGS_STRING;
                        (*values.add(count as usize)).union_.string = xstrdup(*string).as_ptr();
                    }
                    cmd_parse_argument_type::Commands(commands) => {
                        cmd_parse_build_commands(commands, pi, pr);
                        match *pr {
                            Err(_) => break 'out,
                            Ok(cmdlist) => {
                                (*values.add(count as _)).type_ = args_type::ARGS_COMMANDS;
                                (*values.add(count as _)).union_.cmdlist = cmdlist;
                            }
                        }
                    }
                    cmd_parse_argument_type::ParsedCommands(cmdlist) => {
                        (*values.add(count as _)).type_ = args_type::ARGS_COMMANDS;
                        (*values.add(count as _)).union_.cmdlist = *cmdlist;
                        (*(*values.add(count as _)).union_.cmdlist).references += 1;
                    }
                }
                count += 1;
            }

            match cmd_parse(values, count, pi.file, pi.line.load(Ordering::SeqCst)) {
                Ok(add) => {
                    let cmdlist = cmd_list_new();
                    *pr = Ok(cmdlist);
                    cmd_list_append(cmdlist, add);
                }
                Err(cause) => {
                    *pr = Err(cmd_parse_get_error(
                        pi.file,
                        pi.line.load(Ordering::SeqCst),
                        cstr_to_str(cause),
                    ));
                    free_(cause);
                    break 'out;
                }
            }
        }
        // out:
        for idx in 0..count {
            args_free_value(values.add(idx as usize));
        }
        free_(values);
    }
}

pub unsafe fn cmd_parse_build_commands(
    cmds: &mut cmd_parse_commands,
    pi: &cmd_parse_input,
    pr: &mut cmd_parse_result,
) {
    unsafe {
        let mut line = u32::MAX;
        let mut current: *mut cmd_list = null_mut();

        *pr = Err(null_mut());

        // Check for an empty list.
        if tailq_empty(cmds) {
            *pr = Ok(cmd_list_new());
            return;
        }
        cmd_parse_log_commands(cmds, c!("cmd_parse_build_commands"));

        // Parse each command into a command list. Create a new command list
        // for each line (unless the flag is set) so they get a new group (so
        // the queue knows which ones to remove if a command fails when
        // executed).
        let result = cmd_list_new();
        for cmd in tailq_foreach(cmds).map(NonNull::as_ptr) {
            if !pi
                .flags
                .intersects(cmd_parse_input_flags::CMD_PARSE_ONEGROUP)
                && (*cmd).line != line
            {
                if !current.is_null() {
                    cmd_parse_print_commands(pi, &mut *current);
                    cmd_list_move(result, current);
                    cmd_list_free(current);
                }
                current = cmd_list_new();
            }
            if current.is_null() {
                current = cmd_list_new();
            }
            line = (*cmd).line;
            pi.line.store((*cmd).line, Ordering::SeqCst);

            cmd_parse_build_command(cmd, pi, pr);
            match *pr {
                Err(err) => {
                    cmd_list_free(result);
                    cmd_list_free(current);
                    return;
                }
                Ok(cmdlist) => {
                    cmd_list_append_all(current, cmdlist);
                    cmd_list_free(cmdlist);
                }
            }
        }

        if !current.is_null() {
            cmd_parse_print_commands(pi, &mut *current);
            cmd_list_move(result, current);
            cmd_list_free(current);
        }

        let s = cmd_list_print(result, 0);
        log_debug!("cmd_parse_build_commands: {}", _s(s));
        free_(s);

        *pr = Ok(result);
    }
}

pub unsafe fn cmd_parse_from_file<'a>(
    f: &'a mut std::io::BufReader<std::fs::File>,
    pi: Option<&'a cmd_parse_input<'a>>,
) -> cmd_parse_result {
    unsafe {
        let mut input: cmd_parse_input = zeroed();
        let pi = pi.unwrap_or(&input);

        let cmds = cmd_parse_do_file(f, pi)?;
        let mut pr = Err(null_mut());
        cmd_parse_build_commands(cmds, pi, &mut pr);
        cmd_parse_free_commands(cmds);
        pr
    }
}

pub unsafe fn cmd_parse_from_string(s: &str, pi: Option<&cmd_parse_input>) -> cmd_parse_result {
    unsafe {
        let mut input: cmd_parse_input = zeroed();
        let pi = pi.unwrap_or(&input);

        (&pi.flags).bitor_assign(cmd_parse_input_flags::CMD_PARSE_ONEGROUP);
        cmd_parse_from_buffer(s.as_bytes(), Some(pi))
    }
}

pub unsafe fn cmd_parse_and_insert(
    s: &str,
    pi: Option<&cmd_parse_input>,
    after: *mut cmdq_item,
    state: *mut cmdq_state,
    error: *mut *mut u8,
) -> cmd_parse_status {
    unsafe {
        match cmd_parse_from_string(s, pi) {
            Err(err) => {
                if !error.is_null() {
                    *error = err;
                } else {
                    free_(err);
                }
                cmd_parse_status::CMD_PARSE_ERROR
            }
            Ok(cmdlist) => {
                let item = cmdq_get_command(cmdlist, state);
                cmdq_insert_after(after, item);
                cmd_list_free(cmdlist);
                cmd_parse_status::CMD_PARSE_SUCCESS
            }
        }
    }
}

pub unsafe fn cmd_parse_and_append(
    s: &str,
    pi: Option<&cmd_parse_input>,
    c: *mut client,
    state: *mut cmdq_state,
    error: *mut *mut u8,
) -> cmd_parse_status {
    unsafe {
        match cmd_parse_from_string(s, pi) {
            Err(err) => {
                if !error.is_null() {
                    *error = err;
                } else {
                    free_(err);
                }
                cmd_parse_status::CMD_PARSE_ERROR
            }
            Ok(cmdlist) => {
                let item = cmdq_get_command(cmdlist, state);
                cmdq_append(c, item);
                cmd_list_free(cmdlist);
                cmd_parse_status::CMD_PARSE_SUCCESS
            }
        }
    }
}

pub unsafe fn cmd_parse_from_buffer(buf: &[u8], pi: Option<&cmd_parse_input>) -> cmd_parse_result {
    unsafe {
        let mut input: cmd_parse_input = zeroed();
        let pi = pi.unwrap_or(&input);

        if buf.is_empty() {
            return Ok(cmd_list_new());
        }

        let cmds = match cmd_parse_do_buffer(buf, pi) {
            Ok(cmds) => cmds,
            Err(cause) => {
                return Err(cause);
            }
        };
        let mut pr = Err(null_mut());
        cmd_parse_build_commands(cmds, pi, &mut pr);
        cmd_parse_free_commands(cmds);
        pr
    }
}

pub unsafe fn cmd_parse_from_buffer_pratt(
    buf: &[u8],
    pi: Option<&cmd_parse_input>,
) -> cmd_parse_result {
    unsafe {
        let mut input: cmd_parse_input = zeroed();
        let pi = pi.unwrap_or(&input);

        if buf.is_empty() {
            return Ok(cmd_list_new());
        }

        let cmds = match cmd_parse_do_buffer_pratt(buf, pi) {
            Ok(cmds) => cmds,
            Err(cause) => {
                return Err(cause);
            }
        };
        let mut pr = Err(null_mut());
        cmd_parse_build_commands(cmds, pi, &mut pr);
        cmd_parse_free_commands(cmds);
        pr
    }
}

pub unsafe fn cmd_parse_from_arguments(
    values: *mut args_value,
    count: u32,
    pi: Option<&mut cmd_parse_input>,
) -> cmd_parse_result {
    unsafe {
        let mut input: cmd_parse_input = zeroed();
        let pi = pi.unwrap_or(&mut input);
        let mut pr = Err(null_mut());
        let cmds = cmd_parse_new_commands();

        let mut cmd = xcalloc1::<cmd_parse_command>() as *mut cmd_parse_command;
        (*cmd).line = pi.line.load(Ordering::SeqCst);
        tailq_init(&raw mut (*cmd).arguments);

        for i in 0..count {
            let mut end = 0;
            if (*values.add(i as usize)).type_ == args_type::ARGS_STRING {
                let copy = xstrdup((*values.add(i as usize)).union_.string).as_ptr();
                let mut size = strlen(copy);
                if size != 0 && *copy.add(size - 1) == b';' as _ {
                    size -= 1;
                    *copy.add(size) = b'\0' as _;
                    if size > 0 && *copy.add(size - 1) == b'\\' as _ {
                        *copy.add(size - 1) = b';' as _;
                    } else {
                        end = 1;
                    }
                }
                if end == 0 || size != 0 {
                    let arg = xcalloc1::<cmd_parse_argument>() as *mut cmd_parse_argument;
                    (*arg).type_ = cmd_parse_argument_type::String(copy);
                    tailq_insert_tail(&raw mut (*cmd).arguments, arg);
                } else {
                    free_(copy);
                }
            } else if (*values.add(i as usize)).type_ == args_type::ARGS_COMMANDS {
                let arg = xcalloc1::<cmd_parse_argument>() as *mut cmd_parse_argument;
                let cmdlist = (*values.add(i as usize)).union_.cmdlist;
                (*cmdlist).references += 1;
                (*arg).type_ = cmd_parse_argument_type::ParsedCommands(cmdlist);
                tailq_insert_tail(&raw mut (*cmd).arguments, arg);
            } else {
                fatalx("unknown argument type");
            }
            if end != 0 {
                tailq_insert_tail(cmds, cmd);
                cmd = xcalloc1::<cmd_parse_command>();
                (*cmd).line = pi.line.load(Ordering::SeqCst);
                tailq_init(&raw mut (*cmd).arguments);
            }
        }
        if !tailq_empty(&raw mut (*cmd).arguments) {
            tailq_insert_tail(cmds, cmd);
        } else {
            free_(cmd);
        }

        cmd_parse_build_commands(cmds, pi, &mut pr);
        cmd_parse_free_commands(cmds);
        pr
    }
}

pub mod lexer {
    use crate::{cmd_parse_state, transmute_ptr};
    use core::ptr::NonNull;

    pub struct Lexer<'a> {
        ps: NonNull<cmd_parse_state<'a>>,
    }
    impl<'a> Lexer<'a> {
        pub fn new(ps: NonNull<cmd_parse_state<'a>>) -> Self {
            Lexer { ps }
        }
    }

    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub enum Tok {
        Zero, // invalid
        Newline,
        Semicolon,
        LeftBrace,
        RightBrace,

        Error,
        Hidden,
        If,
        Else,
        Elif,
        Endif,

        Format(Option<NonNull<u8>>),
        Token(Option<NonNull<u8>>),
        Equals(Option<NonNull<u8>>),
    }
    impl std::fmt::Display for Tok {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Tok::Zero => write!(f, "zero"),
                Tok::Newline => write!(f, "\\n"),
                Tok::Semicolon => write!(f, ";"),
                Tok::LeftBrace => write!(f, "{{"),
                Tok::RightBrace => write!(f, "}}"),
                Tok::Error => write!(f, "%error"),
                Tok::Hidden => write!(f, "%hidden"),
                Tok::If => write!(f, "%if"),
                Tok::Else => write!(f, "%else"),
                Tok::Elif => write!(f, "%elif"),
                Tok::Endif => write!(f, "%endif"),
                Tok::Format(non_null) => {
                    write!(f, "format({})", unsafe {
                        crate::_s(transmute_ptr(*non_null))
                    })
                }
                Tok::Token(non_null) => write!(f, "token({})", unsafe {
                    crate::_s(transmute_ptr(*non_null))
                }),
                Tok::Equals(non_null) => {
                    write!(f, "equals({})", unsafe {
                        crate::_s(transmute_ptr(*non_null))
                    })
                }
            }
        }
    }

    #[derive(Debug)]
    pub enum LexicalError {
        // Not possible
    }
    type Loc = usize;
    impl Iterator for Lexer<'_> {
        type Item = Result<(Loc, Tok, Loc), LexicalError>;

        fn next(&mut self) -> Option<Result<(Loc, Tok, Loc), LexicalError>> {
            unsafe { super::yylex_(&mut *self.ps.as_ptr()).map(|tok| Ok((0, tok, 0))) }
        }
    }
}

macro_rules! yyerror {
   ($ps:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::cmd_parse::yyerror_(&mut *$ps, format_args!($fmt $(, $args)*))
    };
}
unsafe fn yyerror_(ps: &mut cmd_parse_state, args: std::fmt::Arguments) -> i32 {
    unsafe {
        if !ps.error.is_null() {
            return 0;
        }

        let mut pi = ps.input.as_mut().unwrap();

        let mut error = args.to_string();
        error.push('\0');

        ps.error = cmd_parse_get_error(pi.file, pi.line.load(Ordering::SeqCst), &error);
        0
    }
}

fn yylex_is_var(ch: u8, first: bool) -> bool {
    if ch == b'=' || (first && ch.is_ascii_digit()) {
        false
    } else {
        ch.is_ascii_alphanumeric() || ch == b'_'
    }
}

unsafe fn yylex_append(buf: *mut *mut u8, len: *mut usize, add: *const u8, addlen: usize) {
    unsafe {
        if (addlen > usize::MAX - 1 || *len > usize::MAX - 1 - addlen) {
            fatalx("buffer is too big");
        }
        *buf = xrealloc_(*buf, (*len) + 1 + addlen).as_ptr();
        libc::memcpy((*buf).add(*len).cast(), add.cast(), addlen);
        (*len) += addlen;
    }
}

unsafe fn yylex_append1(buf: *mut *mut u8, len: *mut usize, add: u8) {
    unsafe {
        yylex_append(buf, len, &raw const add, 1);
    }
}

fn yylex_getc1(ps: &mut cmd_parse_state) -> i32 {
    let ch;
    if let Some(f) = ps.f.as_mut() {
        if let Some(c) = ps.unget_buf.take() {
            return c;
        }
        let mut buf: [u8; 1] = [0];
        match f.read(&mut buf) {
            Ok(count) => {
                if count == 0 {
                    ch = libc::EOF;
                } else if count == 1 {
                    ch = buf[0] as i32;
                } else {
                    panic!("unexecpted read size");
                }
            }
            Err(err) => {
                ch = libc::EOF;
            }
        }
    } else if ps.off == ps.buf.unwrap().len() {
        ch = libc::EOF;
    } else {
        ch = ps.buf.unwrap()[ps.off] as i32;
        ps.off += 1;
    }

    ch
}

fn yylex_ungetc(ps: &mut cmd_parse_state, ch: i32) {
    if let Some(f) = ps.f.as_mut() {
        ps.unget_buf = Some(ch)
    } else if ps.off > 0 && ch != libc::EOF {
        ps.off -= 1;
    }
}

fn yylex_getc(ps: &mut cmd_parse_state) -> i32 {
    if ps.escapes != 0 {
        ps.escapes -= 1;
        return '\\' as i32;
    }
    loop {
        let ch = yylex_getc1(ps);
        if ch == '\\' as i32 {
            ps.escapes += 1;
            continue;
        }
        if ch == '\n' as i32 && ps.escapes % 2 == 1 {
            ps.input
                .as_mut()
                .unwrap()
                .line
                .fetch_add(1, Ordering::SeqCst);
            ps.escapes -= 1;
            continue;
        }

        if ps.escapes != 0 {
            yylex_ungetc(ps, ch);
            ps.escapes -= 1;
            return '\\' as i32;
        }
        return ch;
    }
}

unsafe fn yylex_get_word(ps: &mut cmd_parse_state, mut ch: i32) -> *mut u8 {
    unsafe {
        let mut len = 0;
        let mut buf: *mut u8 = xmalloc(1).cast().as_ptr();

        loop {
            yylex_append1(&raw mut buf, &raw mut len, ch as u8);
            ch = yylex_getc(ps);
            if ch == libc::EOF || !libc::strchr(c!(" \t\n"), ch).is_null() {
                break;
            }
        }
        yylex_ungetc(ps, ch);

        *buf.add(len) = b'\0';
        // log_debug("%s: %s", __func__, buf);
        buf
    }
}

use lexer::Tok;

unsafe fn yylex_(ps: &mut cmd_parse_state) -> Option<Tok> {
    unsafe {
        let mut next: i32 = 0;

        if (ps.eol != 0) {
            ps.input
                .as_mut()
                .unwrap()
                .line
                .fetch_add(1, Ordering::SeqCst);
        }
        ps.eol = 0;

        let mut condition = ps.condition;
        ps.condition = 0;

        loop {
            let mut ch = yylex_getc(ps);

            if ch == libc::EOF {
                /*
                 * Ensure every file or string is terminated by a
                 * newline. This keeps the parser simpler and avoids
                 * having to add a newline to each string.
                 */
                if ps.eof != 0 {
                    break;
                }
                ps.eof = 1;
                return Some(Tok::Newline);
            }

            if (ch == ' ' as i32 || ch == '\t' as i32) {
                /*
                 * Ignore whitespace.
                 */
                continue;
            }

            if (ch == '\r' as i32) {
                /*
                 * Treat \r\n as \n.
                 */
                ch = yylex_getc(ps);
                if (ch != '\n' as i32) {
                    yylex_ungetc(ps, ch);
                    ch = '\r' as i32;
                }
            }
            if (ch == '\n' as i32) {
                /*
                 * End of line. Update the line number.
                 */
                ps.eol = 1;
                return Some(Tok::Newline);
            }

            if ch == ';' as i32 {
                return Some(Tok::Semicolon);
            }
            if ch == '{' as i32 {
                return Some(Tok::LeftBrace);
            }
            if ch == '}' as i32 {
                return Some(Tok::RightBrace);
            }

            if (ch == '#' as i32) {
                /*
                 * #{ after a condition opens a format; anything else
                 * is a comment, ignore up to the end of the line.
                 */
                next = yylex_getc(ps);
                if (condition != 0 && next == '{' as i32) {
                    let yylval_token = yylex_format(ps);
                    if yylval_token.is_none() {
                        return Some(Tok::Error);
                    }
                    return Some(Tok::Format(yylval_token));
                }
                while (next != '\n' as i32 && next != libc::EOF) {
                    next = yylex_getc(ps);
                }
                if next == '\n' as i32 {
                    ps.input
                        .as_mut()
                        .unwrap()
                        .line
                        .fetch_add(1, Ordering::SeqCst);
                    return Some(Tok::Newline);
                }
                continue;
            }

            if ch == '%' as i32 {
                /*
                 * % is a condition unless it is all % or all numbers,
                 * then it is a token.
                 */
                let yylval_token = yylex_get_word(ps, '%' as i32);
                let mut cp = yylval_token;
                while *cp != b'\0' {
                    if *cp != b'%' && !(*cp as u8).is_ascii_digit() {
                        break;
                    }
                    cp = cp.add(1);
                }
                if (*cp == b'\0') {
                    return Some(Tok::Token(NonNull::new(yylval_token)));
                }
                ps.condition = 1;
                if streq_(yylval_token, "%hidden") {
                    free_(yylval_token);
                    return Some(Tok::Hidden);
                }
                if streq_(yylval_token, "%if") {
                    free_(yylval_token);
                    return Some(Tok::If);
                }
                if streq_(yylval_token, "%else") {
                    free_(yylval_token);
                    return Some(Tok::Else);
                }
                if streq_(yylval_token, "%elif") {
                    free_(yylval_token);
                    return Some(Tok::Elif);
                }
                if streq_(yylval_token, "%endif") {
                    free_(yylval_token);
                    return Some(Tok::Endif);
                }
                free_(yylval_token);
                return Some(Tok::Error);
            }

            // Otherwise this is a token.
            let token = yylex_token(ps, ch);
            if token.is_null() {
                return Some(Tok::Error);
            }
            let yylval_token = token;

            if !libc::strchr(token, b'=' as i32).is_null() && yylex_is_var(*token, true) {
                let mut cp = token.add(1);
                while *cp != b'=' {
                    if !yylex_is_var(*cp, false) {
                        break;
                    }
                    cp = cp.add(1);
                }
                if *cp == b'=' {
                    return Some(Tok::Equals(NonNull::new(yylval_token)));
                }
            }
            return Some(Tok::Token(NonNull::new(yylval_token)));
        }

        None
    }
}

unsafe fn yylex_format(ps: &mut cmd_parse_state) -> Option<NonNull<u8>> {
    unsafe {
        let mut brackets = 1;
        let mut len = 0;
        let mut buf = xmalloc_::<u8>().as_ptr();

        'error: {
            yylex_append(&raw mut buf, &raw mut len, c!("#{"), 2);
            loop {
                let mut ch = yylex_getc(ps);
                if (ch == libc::EOF || ch == '\n' as i32) {
                    break 'error;
                }
                if (ch == '#' as i32) {
                    ch = yylex_getc(ps);
                    if (ch == libc::EOF || ch == '\n' as i32) {
                        break 'error;
                    }
                    if ch == '{' as i32 {
                        brackets += 1;
                    }
                    yylex_append1(&raw mut buf, &raw mut len, b'#');
                } else if (ch == '}' as i32)
                    && brackets != 0
                    && ({
                        brackets -= 1;
                        brackets == 0
                    })
                {
                    yylex_append1(&raw mut buf, &raw mut len, ch as u8);
                    break;
                }
                yylex_append1(&raw mut buf, &raw mut len, ch as u8);
            }
            if (brackets != 0) {
                break 'error;
            }

            *buf.add(len) = b'\0';
            // log_debug("%s: %s", __func__, buf);
            return NonNull::new(buf);
        } // error:

        free_(buf);
        None
    }
}

unsafe fn yylex_token_variable(
    ps: &mut cmd_parse_state,
    buf: *mut *mut u8,
    len: *mut usize,
) -> bool {
    unsafe {
        let mut namelen: usize = 0;
        let mut name: [u8; 1024] = [0; 1024];
        const SIZEOF_NAME: usize = 1024;
        let mut brackets = 0;

        let mut ch = yylex_getc(ps);
        if (ch == libc::EOF) {
            return false;
        }
        if (ch == '{' as i32) {
            brackets = 1;
        } else {
            if !yylex_is_var(ch as u8, true) {
                yylex_append1(buf, len, b'$');
                yylex_ungetc(ps, ch);
                return true;
            }
            name[namelen] = ch as u8;
            namelen += 1;
        }

        loop {
            ch = yylex_getc(ps);
            if (brackets != 0 && ch == '}' as i32) {
                break;
            }
            if (ch == libc::EOF || !yylex_is_var(ch as u8, false)) {
                if brackets == 0 {
                    yylex_ungetc(ps, ch);
                    break;
                }
                yyerror!(ps, "invalid environment variable");
                return false;
            }
            if namelen == SIZEOF_NAME - 2 {
                yyerror!(ps, "environment variable is too long");
                return false;
            }
            name[namelen] = ch as u8;
            namelen += 1;
        }
        name[namelen] = b'\0';

        let mut envent = environ_find(GLOBAL_ENVIRON, (&raw const name).cast());
        if !envent.is_null() && (*envent).value.is_some() {
            let value = (*envent).value;
            // log_debug("%s: %s -> %s", __func__, name, value);
            yylex_append(
                buf,
                len,
                transmute_ptr(value),
                libc::strlen(transmute_ptr(value)),
            );
        }
        true
    }
}

unsafe fn yylex_token_tilde(ps: &mut cmd_parse_state, buf: *mut *mut u8, len: *mut usize) -> bool {
    unsafe {
        let mut home = null();
        let mut namelen: usize = 0;
        let mut name: [u8; 1024] = [0; 1024];
        const SIZEOF_NAME: usize = 1024;

        loop {
            let ch = yylex_getc(ps);
            if ch == libc::EOF || !libc::strchr(c!("/ \t\n\"'"), ch).is_null() {
                yylex_ungetc(ps, ch);
                break;
            }
            if namelen == SIZEOF_NAME - 2 {
                yyerror!(ps, "user name is too long");
                return false;
            }
            name[namelen] = ch as u8;
            namelen += 1;
        }
        name[namelen] = b'\0';

        if name[0] == b'\0' {
            let envent = environ_find(GLOBAL_ENVIRON, c!("HOME"));
            if (!envent.is_null() && (*(*envent).value.unwrap().as_ptr()) != b'\0') {
                home = transmute_ptr((*envent).value);
            } else if let Some(pw) = NonNull::new(libc::getpwuid(libc::getuid())) {
                home = (*pw.as_ptr()).pw_dir.cast();
            }
        } else if let Some(pw) = NonNull::new(libc::getpwnam((&raw const name).cast())) {
            home = (*pw.as_ptr()).pw_dir.cast();
        }
        if home.is_null() {
            return false;
        }

        // log_debug("%s: ~%s -> %s", __func__, name, home);
        yylex_append(buf, len, home, strlen(home));
        true
    }
}

unsafe fn yylex_token(ps: &mut cmd_parse_state, mut ch: i32) -> *mut u8 {
    unsafe {
        #[derive(Copy, Clone, Eq, PartialEq)]
        enum State {
            Start,
            None,
            DoubleQuotes,
            SingleQuotes,
        }

        let mut state = State::None;
        let mut last = State::Start;

        let mut len = 0;
        let mut buf = xmalloc_::<u8>().as_ptr();

        'error: {
            'aloop: loop {
                'next: {
                    'skip: {
                        /* EOF or \n are always the end of the token. */
                        if (ch == libc::EOF) {
                            // log_debug("%s: end at EOF", __func__);
                            break 'aloop;
                        }
                        if (state == State::None && ch == '\r' as i32) {
                            ch = yylex_getc(ps);
                            if (ch != '\n' as i32) {
                                yylex_ungetc(ps, ch);
                                ch = '\r' as i32;
                            }
                        }
                        if (state == State::None && ch == '\n' as i32) {
                            // log_debug("%s: end at EOL", __func__);
                            break 'aloop;
                        }

                        /* Whitespace or ; or } ends a token unless inside quotes. */
                        if state == State::None && (ch == ' ' as i32 || ch == '\t' as i32) {
                            // log_debug("%s: end at WS", __func__);
                            break 'aloop;
                        }
                        if (state == State::None && (ch == ';' as i32 || ch == '}' as i32)) {
                            // log_debug("%s: end at %c", __func__, ch);
                            break 'aloop;
                        }

                        /*
                         * Spaces and comments inside quotes after \n are removed but
                         * the \n is left.
                         */
                        if (ch == '\n' as i32 && state != State::None) {
                            yylex_append1(&raw mut buf, &raw mut len, b'\n');
                            while ({
                                ch = yylex_getc(ps);
                                ch == b' ' as i32
                            }) || ch == '\t' as i32
                            {}
                            if (ch != '#' as i32) {
                                continue 'aloop;
                            }
                            ch = yylex_getc(ps);
                            if !libc::strchr(c!(",#{}:"), ch).is_null() {
                                yylex_ungetc(ps, ch);
                                ch = '#' as i32;
                            } else {
                                while ({
                                    ch = yylex_getc(ps);
                                    ch != '\n' as i32 && ch != libc::EOF
                                }) { /* nothing */ }
                            }
                            continue 'aloop;
                        }

                        /* \ ~ and $ are expanded except in single quotes. */
                        if ch == '\\' as i32 && state != State::SingleQuotes {
                            if !yylex_token_escape(ps, &raw mut buf, &raw mut len) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '~' as i32 && last != state && state != State::SingleQuotes {
                            if !yylex_token_tilde(ps, &raw mut buf, &raw mut len) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '$' as i32 && state != State::SingleQuotes {
                            if !yylex_token_variable(ps, &raw mut buf, &raw mut len) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '}' as i32 && state == State::None {
                            break 'error; /* unmatched (matched ones were handled) */
                        }

                        /* ' and " starts or end quotes (and is consumed). */
                        if ch == '\'' as i32 {
                            if (state == State::None) {
                                state = State::SingleQuotes;
                                break 'next;
                            }
                            if (state == State::SingleQuotes) {
                                state = State::None;
                                break 'next;
                            }
                        }
                        if ch == b'"' as i32 {
                            if (state == State::None) {
                                state = State::DoubleQuotes;
                                break 'next;
                            }
                            if (state == State::DoubleQuotes) {
                                state = State::None;
                                break 'next;
                            }
                        }

                        /* Otherwise add the character to the buffer. */
                        yylex_append1(&raw mut buf, &raw mut len, ch as u8);
                    }
                    // skip:
                    last = state;
                }
                // next:
                ch = yylex_getc(ps);
            }
            yylex_ungetc(ps, ch);

            *buf.add(len) = b'\0';
            // log_debug("%s: %s", __func__, buf);
            return (buf);
        } // error:
        free_(buf);

        null_mut()
    }
}

unsafe fn yylex_token_escape(ps: &mut cmd_parse_state, buf: *mut *mut u8, len: *mut usize) -> bool {
    unsafe {
        #[cfg(not(target_os = "macos"))]
        const SIZEOF_M: usize = libc::_SC_MB_LEN_MAX as usize;

        // TODO determine a more stable way to get this value on mac
        #[cfg(target_os = "macos")]
        const SIZEOF_M: usize = 6; // compiled and printed constant from C

        let mut tmp: u32 = 0;
        let mut s: [u8; 9] = [0; 9];
        let mut m: [u8; SIZEOF_M] = [0; SIZEOF_M];
        let mut size: usize = 0;
        let mut type_: i32 = 0;

        'unicode: {
            let mut ch = yylex_getc(ps);

            if (ch >= '4' as i32 && ch <= '7' as i32) {
                yyerror!(ps, "invalid octal escape");
                return false;
            }
            if (ch >= '0' as i32 && ch <= '3' as i32) {
                let o2 = yylex_getc(ps);
                if (o2 >= '0' as i32 && o2 <= '7' as i32) {
                    let o3 = yylex_getc(ps);
                    if (o3 >= '0' as i32 && o3 <= '7' as i32) {
                        ch = 64 * (ch - '0' as i32) + 8 * (o2 - '0' as i32) + (o3 - '0' as i32);
                        yylex_append1(buf, len, ch as u8);
                        return true;
                    }
                }
                yyerror!(ps, "invalid octal escape");
                return false;
            }

            if ch == libc::EOF {
                return false;
            }

            match ch as u8 as char {
                'a' => ch = '\x07' as i32,
                'b' => ch = '\x08' as i32,
                'e' => ch = '\x1B' as i32,
                'f' => ch = '\x0C' as i32,
                's' => ch = ' ' as i32,
                'v' => ch = '\x0B' as i32,
                'r' => ch = '\r' as i32,
                'n' => ch = '\n' as i32,
                't' => ch = '\t' as i32,
                'u' => {
                    type_ = 'u' as i32;
                    size = 4;
                    break 'unicode;
                }
                'U' => {
                    type_ = 'U' as i32;
                    size = 8;
                    break 'unicode;
                }
                _ => (),
            }

            yylex_append1(buf, len, ch as u8);
            return true;
        } // unicode:
        let mut i = 0;
        for i_ in 0..size {
            i = i_;
            let ch = yylex_getc(ps);
            if ch == libc::EOF || ch == '\n' as i32 {
                return false;
            }
            if !(ch as u8).is_ascii_hexdigit() {
                yyerror!(ps, "invalid \\{} argument", type_ as u8 as char);
                return false;
            }
            s[i] = ch as u8;
        }
        s[i] = b'\0';

        if ((size == 4 && libc::sscanf((&raw mut s).cast(), c"%4x".as_ptr(), &raw mut tmp) != 1)
            || (size == 8 && libc::sscanf((&raw mut s).cast(), c"%8x".as_ptr(), &raw mut tmp) != 1))
        {
            yyerror!(ps, "invalid \\{} argument", type_ as u8 as char);
            return false;
        }
        let mlen = wctomb((&raw mut m).cast(), tmp as i32);
        if mlen <= 0 || mlen > SIZEOF_M as i32 {
            yyerror!(ps, "invalid \\{} argument", type_ as u8 as char);
            return false;
        }
        yylex_append(buf, len, (&raw const m).cast(), mlen as usize);

        true
    }
}

pub unsafe fn cmd_parse_commands_equivalent(
    a: *const cmd_parse_commands,
    b: *const cmd_parse_commands,
) -> bool {
    unsafe {
        if a == b {
            return true;
        }
        if a.is_null() || b.is_null() {
            return false;
        }

        let mut cmd_a: *const cmd_parse_command = tailq_first(a as *mut cmd_parse_commands);
        let mut cmd_b: *const cmd_parse_command = tailq_first(b as *mut cmd_parse_commands);

        while !cmd_a.is_null() && !cmd_b.is_null() {
            if !cmd_parse_command_equivalent(cmd_a, cmd_b) {
                return false;
            }
            cmd_a = (*cmd_a).entry.tqe_next;
            cmd_b = (*cmd_b).entry.tqe_next;
        }

        cmd_a.is_null() && cmd_b.is_null()
    }
}

unsafe fn cmd_parse_command_equivalent(
    a: *const cmd_parse_command,
    b: *const cmd_parse_command,
) -> bool {
    unsafe {
        if a == b {
            return true;
        }
        if a.is_null() || b.is_null() {
            return false;
        }

        if (*a).line != (*b).line {
            return false;
        }

        cmd_parse_arguments_equivalent(&(*a).arguments, &(*b).arguments)
    }
}

unsafe fn cmd_parse_arguments_equivalent(
    a: *const cmd_parse_arguments,
    b: *const cmd_parse_arguments,
) -> bool {
    unsafe {
        if a == b {
            return true;
        }
        if a.is_null() || b.is_null() {
            return false;
        }

        let mut arg_a: *const cmd_parse_argument = tailq_first(a as *mut cmd_parse_arguments);
        let mut arg_b: *const cmd_parse_argument = tailq_first(b as *mut cmd_parse_arguments);

        while !arg_a.is_null() && !arg_b.is_null() {
            if !cmd_parse_argument_equivalent(arg_a, arg_b) {
                return false;
            }
            arg_a = (*arg_a).entry.tqe_next;
            arg_b = (*arg_b).entry.tqe_next;
        }

        arg_a.is_null() && arg_b.is_null()
    }
}

unsafe fn cmd_parse_argument_equivalent(
    a: *const cmd_parse_argument,
    b: *const cmd_parse_argument,
) -> bool {
    unsafe {
        if a == b {
            return true;
        }
        if a.is_null() || b.is_null() {
            return false;
        }

        match (&(*a).type_, &(*b).type_) {
            (cmd_parse_argument_type::String(s1), cmd_parse_argument_type::String(s2)) => {
                if s1.is_null() && s2.is_null() {
                    true
                } else if s1.is_null() || s2.is_null() {
                    false
                } else {
                    libc::strcmp(*s1, *s2) == 0
                }
            }
            (cmd_parse_argument_type::Commands(c1), cmd_parse_argument_type::Commands(c2)) => {
                cmd_parse_commands_equivalent(*c1, *c2)
            }
            (
                cmd_parse_argument_type::ParsedCommands(l1),
                cmd_parse_argument_type::ParsedCommands(l2),
            ) => cmd_list_equivalent(*l1, *l2),
            _ => false,
        }
    }
}

unsafe fn cmd_list_equivalent(a: *const cmd_list, b: *const cmd_list) -> bool {
    unsafe {
        if a == b {
            return true;
        }
        if a.is_null() || b.is_null() {
            return false;
        }

        let mut cmd_a: *const cmd = tailq_first((*a).list);
        let mut cmd_b: *const cmd = tailq_first((*b).list);

        while !cmd_a.is_null() && !cmd_b.is_null() {
            if !cmd_equivalent(cmd_a, cmd_b) {
                return false;
            }
            cmd_a = (*cmd_a).qentry.tqe_next;
            cmd_b = (*cmd_b).qentry.tqe_next;
        }

        cmd_a.is_null() && cmd_b.is_null()
    }
}

unsafe fn cmd_equivalent(a: *const cmd, b: *const cmd) -> bool {
    unsafe {
        if a == b {
            return true;
        }
        if a.is_null() || b.is_null() {
            return false;
        }

        if (*a).entry as *const _ != (*b).entry as *const _ {
            return false;
        }

        args_equivalent((*a).args, (*b).args)
    }
}

unsafe fn args_equivalent(a: *const args, b: *const args) -> bool {
    unsafe {
        if a == b {
            return true;
        }
        if a.is_null() || b.is_null() {
            return false;
        }

        if (*a).count != (*b).count {
            return false;
        }

        let mut values_a = (*a).values;
        let mut values_b = (*b).values;

        while !values_a.is_null() && !values_b.is_null() {
            if !args_value_equivalent(values_a, values_b) {
                return false;
            }
            values_a = (*values_a).entry.tqe_next;
            values_b = (*values_b).entry.tqe_next;
        }

        values_a.is_null() && values_b.is_null()
    }
}

unsafe fn args_value_equivalent(a: *const args_value, b: *const args_value) -> bool {
    unsafe {
        if a == b {
            return true;
        }
        if a.is_null() || b.is_null() {
            return false;
        }

        if (*a).type_ != (*b).type_ {
            return false;
        }

        match (*a).type_ {
            args_type::ARGS_NONE => true,
            args_type::ARGS_STRING => {
                let s1 = (*a).union_.string;
                let s2 = (*b).union_.string;
                if s1.is_null() && s2.is_null() {
                    true
                } else if s1.is_null() || s2.is_null() {
                    false
                } else {
                    libc::strcmp(s1, s2) == 0
                }
            }
            args_type::ARGS_COMMANDS => {
                cmd_list_equivalent((*a).union_.cmdlist, (*b).union_.cmdlist)
            }
        }
    }
}

pub unsafe fn cmd_parse_commands_debug(cmds: *const cmd_parse_commands, prefix: &str) {
    unsafe {
        if cmds.is_null() {
            println!("{}NULL", prefix);
            return;
        }

        if tailq_empty(cmds as *mut cmd_parse_commands) {
            println!("{}EMPTY", prefix);
            return;
        }

        let mut cmd_idx = 0;
        let mut cmd = tailq_first(cmds as *mut cmd_parse_commands);
        while !cmd.is_null() {
            println!("{}cmd[{}] line:{}", prefix, cmd_idx, (*cmd).line);
            cmd_parse_arguments_debug(&(*cmd).arguments, &format!("{}  ", prefix));
            cmd = (*cmd).entry.tqe_next;
            cmd_idx += 1;
        }
    }
}

unsafe fn cmd_parse_arguments_debug(args: *const cmd_parse_arguments, prefix: &str) {
    unsafe {
        if args.is_null() {
            println!("{}args: NULL", prefix);
            return;
        }

        if tailq_empty(args as *mut cmd_parse_arguments) {
            println!("{}args: EMPTY", prefix);
            return;
        }

        let mut arg_idx = 0;
        let mut arg = tailq_first(args as *mut cmd_parse_arguments);
        while !arg.is_null() {
            print!("{}arg[{}]: ", prefix, arg_idx);
            cmd_parse_argument_debug(arg);
            arg = (*arg).entry.tqe_next;
            arg_idx += 1;
        }
    }
}

unsafe fn cmd_parse_argument_debug(arg: *const cmd_parse_argument) {
    unsafe {
        if arg.is_null() {
            println!("NULL");
            return;
        }

        match &(*arg).type_ {
            cmd_parse_argument_type::String(s) => {
                if s.is_null() {
                    println!("String(NULL)");
                } else {
                    println!("String(\"{}\")", _s(*s));
                }
            }
            cmd_parse_argument_type::Commands(cmds) => {
                println!("Commands(");
                cmd_parse_commands_debug(*cmds, "    ");
                println!("  )");
            }
            cmd_parse_argument_type::ParsedCommands(cmdlist) => {
                if cmdlist.is_null() {
                    println!("ParsedCommands(NULL)");
                } else {
                    println!("ParsedCommands(refs: {})", (*(*cmdlist)).references);
                    cmd_list_debug(*cmdlist, "    ");
                }
            }
        }
    }
}

unsafe fn cmd_list_debug(cmdlist: *const cmd_list, prefix: &str) {
    unsafe {
        if cmdlist.is_null() {
            println!("{}NULL", prefix);
            return;
        }

        println!(
            "{}group: {}, refs: {}",
            prefix,
            (*cmdlist).group,
            (*cmdlist).references
        );

        let mut cmd_idx = 0;
        let mut cmd = tailq_first((*cmdlist).list);
        while !cmd.is_null() {
            println!("{}cmd[{}]:", prefix, cmd_idx);
            cmd_debug(cmd, &format!("{}  ", prefix));
            cmd = (*cmd).qentry.tqe_next;
            cmd_idx += 1;
        }
    }
}

unsafe fn cmd_debug(cmd: *const cmd, prefix: &str) {
    unsafe {
        if cmd.is_null() {
            println!("{}NULL", prefix);
            return;
        }

        println!("{}entry: {}", prefix, _s((*cmd).entry.name));

        args_debug((*cmd).args, &format!("{}  ", prefix));
    }
}

unsafe fn args_debug(args: *const args, prefix: &str) {
    unsafe {
        if args.is_null() {
            println!("{}args: NULL", prefix);
            return;
        }

        println!("{}args: count={}", prefix, (*args).count);

        let mut value = (*args).values;
        let mut idx = 0;
        while !value.is_null() {
            print!("{}  value[{}]: ", prefix, idx);
            args_value_debug(value);
            value = (*value).entry.tqe_next;
            idx += 1;
        }
    }
}

unsafe fn args_value_debug(value: *const args_value) {
    unsafe {
        if value.is_null() {
            println!("NULL");
            return;
        }

        match (*value).type_ {
            args_type::ARGS_NONE => println!("NONE"),
            args_type::ARGS_STRING => {
                let s = (*value).union_.string;
                if s.is_null() {
                    println!("STRING(NULL)");
                } else {
                    println!("STRING(\"{}\")", _s(s));
                }
            }
            args_type::ARGS_COMMANDS => {
                let cmdlist = (*value).union_.cmdlist;
                if cmdlist.is_null() {
                    println!("COMMANDS(NULL)");
                } else {
                    println!("COMMANDS(");
                    cmd_list_debug(cmdlist, "      ");
                    println!("    )");
                }
            }
        }
    }
}

#[cfg(test)]
mod fuzz_tests {
    use std::sync::Once;

    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_cmd_parse_commands_equivalence_null() {
        unsafe {
            assert!(cmd_parse_commands_equivalent(
                std::ptr::null(),
                std::ptr::null()
            ));

            let cmds = cmd_parse_new_commands();
            assert!(!cmd_parse_commands_equivalent(cmds, std::ptr::null()));
            assert!(!cmd_parse_commands_equivalent(std::ptr::null(), cmds));

            cmd_parse_free_commands(cmds);
        }
    }

    #[test]
    fn test_cmd_parse_commands_equivalence_empty() {
        unsafe {
            let cmds_a = cmd_parse_new_commands();
            let cmds_b = cmd_parse_new_commands();

            assert!(cmd_parse_commands_equivalent(cmds_a, cmds_b));
            assert!(cmd_parse_commands_equivalent(cmds_a, cmds_a));

            cmd_parse_free_commands(cmds_a);
            cmd_parse_free_commands(cmds_b);
        }
    }

    pub unsafe fn create_test_command_string(command: &str, line: u32) -> *mut cmd_parse_command {
        unsafe {
            let cmd = xcalloc1::<cmd_parse_command>();
            (*cmd).line = line;
            tailq_init(&mut (*cmd).arguments);

            let arg = xcalloc1::<cmd_parse_argument>();
            let cmd_str = format!("{}\0", command);
            (*arg).type_ = cmd_parse_argument_type::String(cmd_str.leak().as_mut_ptr());
            tailq_insert_tail(&mut (*cmd).arguments, arg);

            cmd
        }
    }

    #[test]
    fn test_cmd_parse_commands_equivalence_with_commands() {
        unsafe {
            let cmds_a = cmd_parse_new_commands();
            let cmds_b = cmd_parse_new_commands();

            let cmd1_a = create_test_command_string("new-session", 1);
            let cmd1_b = create_test_command_string("new-session", 1);

            tailq_insert_tail(cmds_a, cmd1_a);
            tailq_insert_tail(cmds_b, cmd1_b);

            assert!(cmd_parse_commands_equivalent(cmds_a, cmds_b));

            let cmd2_a = create_test_command_string("list-sessions", 2);
            tailq_insert_tail(cmds_a, cmd2_a);

            assert!(!cmd_parse_commands_equivalent(cmds_a, cmds_b));

            let cmd2_b = create_test_command_string("list-sessions", 2);
            tailq_insert_tail(cmds_b, cmd2_b);

            assert!(cmd_parse_commands_equivalent(cmds_a, cmds_b));

            cmd_parse_free_commands(cmds_a);
            cmd_parse_free_commands(cmds_b);
        }
    }

    #[test]
    fn test_cmd_parse_commands_equivalence_different_lines() {
        unsafe {
            let cmds_a = cmd_parse_new_commands();
            let cmds_b = cmd_parse_new_commands();

            let cmd1_a = create_test_command_string("new-session", 1);
            let cmd1_b = create_test_command_string("new-session", 2);

            tailq_insert_tail(cmds_a, cmd1_a);
            tailq_insert_tail(cmds_b, cmd1_b);

            assert!(!cmd_parse_commands_equivalent(cmds_a, cmds_b));

            cmd_parse_free_commands(cmds_a);
            cmd_parse_free_commands(cmds_b);
        }
    }

    #[test]
    fn test_cmd_parse_commands_equivalence_different_commands() {
        unsafe {
            let cmds_a = cmd_parse_new_commands();
            let cmds_b = cmd_parse_new_commands();

            let cmd1_a = create_test_command_string("new-session", 1);
            let cmd1_b = create_test_command_string("list-sessions", 1);

            tailq_insert_tail(cmds_a, cmd1_a);
            tailq_insert_tail(cmds_b, cmd1_b);

            assert!(!cmd_parse_commands_equivalent(cmds_a, cmds_b));

            cmd_parse_free_commands(cmds_a);
            cmd_parse_free_commands(cmds_b);
        }
    }

    #[test]
    fn test_cmd_parse_commands_debug() {
        unsafe {
            println!("\n=== Debug Test ===");

            // Test NULL
            cmd_parse_commands_debug(std::ptr::null(), "");

            // Test empty
            let empty_cmds = cmd_parse_new_commands();
            cmd_parse_commands_debug(empty_cmds, "");

            // Test with commands
            let cmds = cmd_parse_new_commands();
            let cmd1 = create_test_command_string("new-session", 1);
            let cmd2 = create_test_command_string("list-sessions", 2);

            tailq_insert_tail(cmds, cmd1);
            tailq_insert_tail(cmds, cmd2);

            println!("\nParsed commands:");
            cmd_parse_commands_debug(cmds, "");

            cmd_parse_free_commands(empty_cmds);
            cmd_parse_free_commands(cmds);
            println!("=== End Debug Test ===\n");
        }
    }

    // Strategy for generating simple strings that avoid complex tmux infrastructure
    fn simple_command_string() -> impl Strategy<Value = String> {
        // Use very basic command patterns that don't trigger alias lookup
        let basic_patterns = vec![
            "word",
            "word arg",
            "word arg1 arg2",
            "command",
            "command-name",
            "cmd arg",
        ];
        prop::sample::select(basic_patterns).prop_map(|s| s.to_string())
    }

    fn simple_command_sequence() -> impl Strategy<Value = String> {
        prop::collection::vec(simple_command_string(), 1..3)
            .prop_map(|commands| commands.join("; "))
    }

    // Test equivalence using manually constructed cmd_parse_commands to avoid parser issues
    fn create_simple_test_commands(commands: &[&str]) -> *mut cmd_parse_commands {
        unsafe {
            let cmds = cmd_parse_new_commands();

            for (line_no, cmd_text) in commands.iter().enumerate() {
                let cmd = xcalloc1::<cmd_parse_command>();
                (*cmd).line = (line_no + 1) as u32;
                tailq_init(&mut (*cmd).arguments);

                // Split on whitespace and create arguments
                for word in cmd_text.split_whitespace() {
                    let arg = xcalloc1::<cmd_parse_argument>();
                    let word_str = format!("{}\0", word);
                    (*arg).type_ = cmd_parse_argument_type::String(word_str.leak().as_mut_ptr());
                    tailq_insert_tail(&mut (*cmd).arguments, arg);
                }

                tailq_insert_tail(cmds, cmd);
            }

            cmds
        }
    }

    proptest! {
        #[test]
        fn test_equivalence_consistency(commands in prop::collection::vec(simple_command_string(), 1..4)) {
            unsafe {
                // Create two identical command structures manually
                let cmds1 = create_simple_test_commands(&commands.iter().map(|s| s.as_str()).collect::<Vec<_>>());
                let cmds2 = create_simple_test_commands(&commands.iter().map(|s| s.as_str()).collect::<Vec<_>>());

                // They should be equivalent
                let equivalent = cmd_parse_commands_equivalent(cmds1, cmds2);

                if !equivalent {
                    println!("Equivalence test failed for commands: {:?}", commands);
                    println!("First structure:");
                    cmd_parse_commands_debug(cmds1, "  ");
                    println!("Second structure:");
                    cmd_parse_commands_debug(cmds2, "  ");
                }

                prop_assert!(equivalent, "Identical command structures should be equivalent");

                cmd_parse_free_commands(cmds1);
                cmd_parse_free_commands(cmds2);
            }
        }
    }

    proptest! {
        #[test]
        fn test_self_equivalence(commands in prop::collection::vec(simple_command_string(), 1..4)) {
            unsafe {
                let cmds = create_simple_test_commands(&commands.iter().map(|s| s.as_str()).collect::<Vec<_>>());

                // A structure should be equivalent to itself
                let self_equivalent = cmd_parse_commands_equivalent(cmds, cmds);

                if !self_equivalent {
                    println!("Self-equivalence test failed for commands: {:?}", commands);
                    cmd_parse_commands_debug(cmds, "  ");
                }

                prop_assert!(self_equivalent, "Command structure should be equivalent to itself");

                cmd_parse_free_commands(cmds);
            }
        }
    }

    #[test]
    fn test_empty_commands_equivalent() {
        unsafe {
            let empty1 = cmd_parse_new_commands();
            let empty2 = cmd_parse_new_commands();

            assert!(
                cmd_parse_commands_equivalent(empty1, empty2),
                "Empty command lists should be equivalent"
            );
            assert!(
                cmd_parse_commands_equivalent(empty1, empty1),
                "Empty command list should be self-equivalent"
            );

            cmd_parse_free_commands(empty1);
            cmd_parse_free_commands(empty2);
        }
    }

    proptest! {
        #[test]
        fn test_simple_string_equivalence(s in "[a-zA-Z][a-zA-Z0-9_-]{0,20}") {

            static START: Once = Once::new();
                START.call_once(|| {
                unsafe {
                        init_global_state();
                }
                });


            unsafe {
                // Create two command structures with the same random string
                let cmds1 = cmd_parse_from_buffer(s.as_bytes(), None);
                let cmds2 = cmd_parse_from_buffer_pratt(s.as_bytes(), None);

                match (cmds1, cmds2) {
                    (Ok(c1), Ok(c2)) => {
                        // They should be equivalent
                        prop_assert!(cmd_list_equivalent(c1, c2));

                        cmd_list_free(c1);
                        cmd_list_free(c2);
                    }
                    (Ok(_), Err(_))| (Err(_), Ok(_)) => {
                        prop_assert!(false);
                    }
                    (Err(_), Err(_)) => (),
                }


            }
        }
    }

    // Strategy for generating tmux-like commands
    fn tmux_command_strategy() -> impl Strategy<Value = String> {
        let commands = vec![
            "new-session",
            "new-window",
            "split-window",
            "select-window",
            "kill-window",
            "list-sessions",
            "list-windows",
            "list-panes",
            "attach-session",
            "detach-client",
            "send-keys",
            "copy-mode",
            "paste-buffer",
            "set-option",
            "set-window-option",
            "bind-key",
            "unbind-key",
            "show-options",
            "display-message",
            "run-shell",
            "if-shell",
            "source-file",
            "rename-session",
            "rename-window",
            "move-window",
            "swap-window",
            "swap-pane",
            "resize-pane",
            "select-pane",
            "kill-pane",
            "clear-history",
            "capture-pane",
            "save-buffer",
            "load-buffer",
            "delete-buffer",
        ];

        let arguments = vec![
            "-d",
            "-s",
            "-t",
            "-n",
            "-c",
            "-h",
            "-v",
            "-p",
            "-x",
            "-y",
            "-l",
            "-r",
            "session_name",
            "window_name",
            "pane_target",
            "command",
            "shell_command",
            "key_binding",
            "option_name",
            "option_value",
            "buffer_name",
            "file_path",
            "format_string",
            "pattern",
            "replacement",
            "index",
            "percentage",
        ];

        prop::sample::select(commands).prop_flat_map(move |cmd| {
            prop::collection::vec(prop::sample::select(arguments.clone()), 0..4).prop_map(
                move |args| {
                    if args.is_empty() {
                        cmd.to_string()
                    } else {
                        format!("{} {}", cmd, args.join(" "))
                    }
                },
            )
        })
    }

    fn tmux_config_line_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple commands
            tmux_command_strategy(),
            // Commands with semicolons
            prop::collection::vec(tmux_command_strategy(), 1..3).prop_map(|cmds| cmds.join("; ")),
            // Variable assignments
            "[a-zA-Z_][a-zA-Z0-9_]*=.*".prop_map(|s| s.replace(".*", "value")),
            // Comments (though they may be filtered out)
            "# [a-zA-Z0-9 _-]*".prop_map(|s| s.replace("*", "comment")),
            // Key bindings
            r"bind-key [a-zA-Z0-9] [a-zA-Z-]*".prop_map(|s| s.to_string()),
            // Options
            r"set-option -g [a-zA-Z-]* [a-zA-Z0-9]*".prop_map(|s| s.to_string()),
        ]
    }

    proptest! {
        #[test]
        fn test_tmux_config_equivalence(s in tmux_config_line_strategy()) {
            static START: Once = Once::new();
            START.call_once(|| {
                unsafe {
                    init_global_state();
                }
            });

            unsafe {
                // Create two command structures with the same tmux config line
                let cmds1 = cmd_parse_from_buffer(s.as_bytes(), None);
                let cmds2 = cmd_parse_from_buffer_pratt(s.as_bytes(), None);

                match (cmds1, cmds2) {
                    (Ok(c1), Ok(c2)) => {
                        // They should be equivalent
                        if !cmd_list_equivalent(c1, c2) {
                            println!("Parser mismatch for input: '{}'", s);
                            println!("LALRPOP result:");
                            cmd_list_debug(c1, "  ");
                            println!("Pratt result:");
                            cmd_list_debug(c2, "  ");
                        }
                        prop_assert!(cmd_list_equivalent(c1, c2));

                        cmd_list_free(c1);
                        cmd_list_free(c2);
                    }
                    (Ok(_), Err(_)) => {
                        println!("LALRPOP succeeded but Pratt failed for: '{}'", s);
                        prop_assert!(false);
                    }
                    (Err(_), Ok(_)) => {
                        println!("Pratt succeeded but LALRPOP failed for: '{}'", s);
                        prop_assert!(false);
                    }
                    (Err(_), Err(_)) => {
                        // Both failed - this is acceptable
                    }
                }
            }
        }
    }
}

unsafe fn init_global_state() {
    unsafe {
        // setproctitle_init(argc, argv.cast(), env.cast());
        let mut cause: *mut u8 = null_mut();
        let mut path: *const u8 = null_mut();
        let mut label: *mut u8 = null_mut();
        let mut feat: i32 = 0;
        let mut fflag: i32 = 0;
        let mut flags: client_flag = client_flag::empty();

        if setlocale(LC_CTYPE, c!("en_US.UTF-8")).is_null()
            && setlocale(LC_CTYPE, c!("C.UTF-8")).is_null()
        {
            if setlocale(LC_CTYPE, c!("")).is_null() {
                eprintln!("invalid LC_ALL, LC_CTYPE or LANG");
                std::process::exit(1);
            }
            let s: *mut u8 = nl_langinfo(CODESET).cast();
            if strcasecmp(s, c!("UTF-8")) != 0 && strcasecmp(s, c!("UTF8")) != 0 {
                eprintln!("need UTF-8 locale (LC_CTYPE) but have {}", _s(s));
                std::process::exit(1);
            }
        }

        setlocale(LC_TIME, c!(""));
        tzset();

        GLOBAL_ENVIRON = environ_create().as_ptr();

        let mut var = environ;
        while !(*var).is_null() {
            environ_put(GLOBAL_ENVIRON, *var, 0);
            var = var.add(1);
        }

        let cwd = find_cwd();
        if !cwd.is_null() {
            environ_set!(GLOBAL_ENVIRON, c!("PWD"), 0, "{}", _s(cwd));
        }
        expand_paths(TMUX_CONF, &raw mut CFG_FILES, &raw mut CFG_NFILES, 1);

        PTM_FD = getptmfd();
        if PTM_FD == -1 {
            eprintln!("getptmfd failed!");
            std::process::exit(1);
        }

        /*
        // TODO no pledge on linux
            if pledge("stdio rpath wpath cpath flock fattr unix getpw sendfd recvfd proc exec tty ps", null_mut()) != 0 {
                err(1, "pledge");
        }
        */

        // tmux is a UTF-8 terminal, so if TMUX is set, assume UTF-8.
        // Otherwise, if the user has set LC_ALL, LC_CTYPE or LANG to contain
        // UTF-8, it is a safe assumption that either they are using a UTF-8
        // terminal, or if not they know that output from UTF-8-capable
        // programs may be wrong.
        if !getenv(c!("TMUX")).is_null() {
            flags |= client_flag::UTF8;
        } else {
            let mut s = getenv(c!("LC_ALL")) as *const u8;
            if s.is_null() || *s == b'\0' {
                s = getenv(c!("LC_CTYPE")) as *const u8;
            }
            if s.is_null() || *s == b'\0' {
                s = getenv(c!("LANG")) as *const u8;
            }
            if s.is_null() || *s == b'\0' {
                s = c!("");
            }
            if !strcasestr(s, c!("UTF-8")).is_null() || !strcasestr(s, c!("UTF8")).is_null() {
                flags |= client_flag::UTF8;
            }
        }

        GLOBAL_OPTIONS = options_create(null_mut());
        GLOBAL_S_OPTIONS = options_create(null_mut());
        GLOBAL_W_OPTIONS = options_create(null_mut());

        let mut oe: *const options_table_entry = &raw const OPTIONS_TABLE as _;
        while !(*oe).name.is_null() {
            if (*oe).scope & OPTIONS_TABLE_SERVER != 0 {
                options_default(GLOBAL_OPTIONS, oe);
            }
            if (*oe).scope & OPTIONS_TABLE_SESSION != 0 {
                options_default(GLOBAL_S_OPTIONS, oe);
            }
            if (*oe).scope & OPTIONS_TABLE_WINDOW != 0 {
                options_default(GLOBAL_W_OPTIONS, oe);
            }
            oe = oe.add(1);
        }

        // The default shell comes from SHELL or from the user's passwd entry if available.
        options_set_string!(
            GLOBAL_S_OPTIONS,
            c!("default-shell"),
            0,
            "{}",
            _s(getshell()),
        );

        // Override keys to vi if VISUAL or EDITOR are set.
        let mut s = getenv(c!("VISUAL"));
        if !s.is_null()
            || ({
                s = getenv(c!("EDITOR"));
                !s.is_null()
            })
        {
            options_set_string!(GLOBAL_OPTIONS, c!("editor"), 0, "{}", _s(s));
            if !strrchr(s, b'/' as _).is_null() {
                s = strrchr(s, b'/' as _).add(1);
            }
            let keys = if !strstr(s, c!("vi")).is_null() {
                modekey::MODEKEY_VI
            } else {
                modekey::MODEKEY_EMACS
            };
            options_set_number(GLOBAL_S_OPTIONS, c!("status-keys"), keys as _);
            options_set_number(GLOBAL_W_OPTIONS, c!("mode-keys"), keys as _);
        }

        // If socket is specified on the command-line with -S or -L, it is
        // used. Otherwise, $TMUX is checked and if that fails "default" is
        // used.
        if path.is_null() && label.is_null() {
            s = getenv(c!("TMUX"));
            if !s.is_null() && *s != b'\0' && *s != b',' {
                let tmp: *mut u8 = xstrdup(s).cast().as_ptr();
                *tmp.add(strcspn(tmp, c!(","))) = b'\0';
                path = tmp;
            }
        }
        if path.is_null() {
            path = make_label(label.cast(), &raw mut cause);
            if path.is_null() {
                if !cause.is_null() {
                    eprintln!("{}", _s(cause));
                    free(cause as _);
                }
                std::process::exit(1);
            }
            flags |= client_flag::DEFAULTSOCKET;
        }
        SOCKET_PATH = path;
        free_(label);
    }
}

pub unsafe fn expand_path(path: *const u8, home: *const u8) -> *mut u8 {
    unsafe {
        let mut expanded: *mut u8 = null_mut();
        let mut end: *const u8 = null_mut();

        if strncmp(path, c!("~/"), 2) == 0 {
            if home.is_null() {
                return null_mut();
            }
            return format_nul!("{}{}", _s(home), _s(path.add(1)));
        }

        if *path == b'$' {
            end = strchr(path, b'/' as i32);
            let name = if end.is_null() {
                xstrdup(path.add(1)).cast().as_ptr()
            } else {
                xstrndup(path.add(1), end.addr() - path.addr() - 1)
                    .cast()
                    .as_ptr()
            };
            let value = environ_find(GLOBAL_ENVIRON, name);
            free_(name);
            if value.is_null() {
                return null_mut();
            }
            if end.is_null() {
                end = c!("");
            }
            return format_nul!("{}{}", _s(transmute_ptr((*value).value)), _s(end));
        }

        xstrdup(path).cast().as_ptr()
    }
}
unsafe fn expand_paths(s: &str, paths: *mut *mut *mut u8, n: *mut u32, ignore_errors: i32) {
    unsafe {
        let home = find_home();
        let mut next: *const u8 = null_mut();
        let mut resolved: [u8; PATH_MAX as usize] = zeroed(); // TODO use unint version
        let mut path = null_mut();

        let func = "expand_paths";

        *paths = null_mut();
        *n = 0;

        let mut tmp: *mut u8 = xstrdup__(s);
        let copy = tmp;
        while {
            next = strsep(&raw mut tmp as _, c!(":").cast());
            !next.is_null()
        } {
            let expanded = expand_path(next, home);
            if expanded.is_null() {
                log_debug!("{}: invalid path: {}", func, _s(next));
                continue;
            }
            if realpath(expanded.cast(), resolved.as_mut_ptr()).is_null() {
                log_debug!(
                    "{}: realpath(\"{}\") failed: {}",
                    func,
                    _s(expanded),
                    _s(strerror(errno!())),
                );
                if ignore_errors != 0 {
                    free_(expanded);
                    continue;
                }
                path = expanded;
            } else {
                path = xstrdup(resolved.as_ptr()).cast().as_ptr();
                free_(expanded);
            }
            let mut i = 0;
            for j in 0..*n {
                i = j;
                if libc::strcmp(path as _, *(*paths).add(i as usize)) == 0 {
                    break;
                }
            }
            if i != *n {
                log_debug!("{}: duplicate path: {}", func, _s(path));
                free_(path);
                continue;
            }
            *paths = xreallocarray_::<*mut u8>(*paths, (*n + 1) as usize).as_ptr();
            *(*paths).add((*n) as usize) = path;
            *n += 1;
        }
        free_(copy);
    }
}

unsafe fn make_label(mut label: *const u8, cause: *mut *mut u8) -> *const u8 {
    let mut paths: *mut *mut u8 = null_mut();
    let mut path: *mut u8 = null_mut();
    let mut base: *mut u8 = null_mut();
    let mut sb: stat = unsafe { zeroed() }; // TODO use uninit
    let mut n: u32 = 0;

    unsafe {
        'fail: {
            *cause = null_mut();
            if label.is_null() {
                label = c!("default");
            }
            let uid = getuid();

            expand_paths(TMUX_SOCK, &raw mut paths, &raw mut n, 1);
            if n == 0 {
                *cause = format_nul!("no suitable socket path");
                return null_mut();
            }
            path = *paths; /* can only have one socket! */
            for i in 1..n {
                free_(*paths.add(i as usize));
            }
            free_(paths);

            base = format_nul!("{}/tmux-{}", _s(path), uid);
            free_(path);
            if mkdir(base.cast(), S_IRWXU) != 0 && errno!() != EEXIST {
                *cause = format_nul!(
                    "couldn't create directory {} ({})",
                    _s(base),
                    _s(strerror(errno!()))
                );
                break 'fail;
            }
            if lstat(base.cast(), &raw mut sb) != 0 {
                *cause = format_nul!(
                    "couldn't read directory {} ({})",
                    _s(base),
                    _s(strerror(errno!())),
                );
                break 'fail;
            }
            if !S_ISDIR(sb.st_mode) {
                *cause = format_nul!("{} is not a directory", _s(base));
                break 'fail;
            }
            if sb.st_uid != uid || (sb.st_mode & S_IRWXO) != 0 {
                *cause = format_nul!("directory {} has unsafe permissions", _s(base));
                break 'fail;
            }
            path = format_nul!("{}/{}", _s(base), _s(label));
            free_(base);
            return path;
        }

        // fail:
        free_(base);
        null_mut()
    }
}

// # Notes:
//
// <https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html>
// <https://github.com/lalrpop/lalrpop/blob/master/README.md>
