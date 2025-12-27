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
use std::io::Read as _;
use std::ops::BitAndAssign as _;
use std::ops::BitOrAssign as _;

use crate::xmalloc::xrecallocarray__;
use crate::*;

#[expect(unused_imports)]
#[allow(clippy::all)]
#[allow(clippy::pedantic)]
#[allow(clippy::restriction)]
mod lalrpop {
    use lalrpop_util::lalrpop_mod;
    lalrpop_mod!(pub(crate) cmd_parse);
}
use lalrpop::cmd_parse;

fn yyparse(ps: &mut cmd_parse_state) -> Result<Option<&'static mut cmd_parse_commands>, ()> {
    let parser = cmd_parse::LinesParser::new();

    let ps = NonNull::new(ps).unwrap();
    let lexer = lexer::Lexer::new(ps);

    match parser.parse(ps, lexer) {
        Ok(cmds) => Ok(cmds),
        Err(parse_err) => {
            log_debug!("parsing error {parse_err:?}");
            Err(())
        }
    }
}

pub struct yystype_elif {
    flag: i32,
    commands: &'static mut cmd_parse_commands,
}

impl_tailq_entry!(cmd_parse_scope, entry, tailq_entry<cmd_parse_scope>);
#[repr(C)]
pub struct cmd_parse_scope {
    pub flag: bool,
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

impl_tailq_entry!(cmd_parse_argument, entry, tailq_entry<cmd_parse_argument>);
#[repr(C)]
pub struct cmd_parse_argument {
    pub type_: cmd_parse_argument_type,

    // #[entry]
    pub entry: tailq_entry<cmd_parse_argument>,
}
pub type cmd_parse_arguments = tailq_head<cmd_parse_argument>;

impl_tailq_entry!(cmd_parse_command, entry, tailq_entry<cmd_parse_command>);
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

pub unsafe fn cmd_parse_get_error(file: Option<&str>, line: u32, error: &str) -> CString {
    match file {
        None => CString::new(error).unwrap(),
        Some(file) => CString::new(format!("{file}:{line}: {error}")).unwrap(),
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
                pi.line.load(atomic::Ordering::SeqCst),
                _s(s)
            );
        } else {
            cmdq_print!(
                pi.item,
                "{}: {}",
                pi.line.load(atomic::Ordering::SeqCst),
                _s(s)
            );
        }
        free_(s);
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
            Err(()) => Err(ps.error),
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

pub unsafe fn cmd_parse_log_commands(cmds: *mut cmd_parse_commands, prefix: *const u8) {
    unsafe {
        for (i, cmd) in tailq_foreach(cmds).map(NonNull::as_ptr).enumerate() {
            for (j, arg) in tailq_foreach(&raw mut (*cmd).arguments)
                .map(NonNull::as_ptr)
                .enumerate()
            {
                match &mut (*arg).type_ {
                    cmd_parse_argument_type::String(string) => {
                        log_debug!("{} {}:{}: {}", _s(prefix), i, j, _s(*string));
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
) -> bool {
    let __func__ = c!("cmd_parse_expand_alias");
    unsafe {
        if pi
            .flags
            .intersects(cmd_parse_input_flags::CMD_PARSE_NOALIAS)
        {
            return false;
        }
        *pr = Err(null_mut());

        let first = tailq_first(&raw mut (*cmd).arguments);
        if first.is_null() {
            *pr = Ok(cmd_list_new());
            return true;
        }
        let cmd_parse_argument_type::String(name) = (*first).type_ else {
            *pr = Ok(cmd_list_new());
            return true;
        };

        let alias = cmd_get_alias(name);
        if alias.is_null() {
            return false;
        }
        log_debug!(
            "{}: {} alias {} = {}",
            _s(__func__),
            pi.line.load(atomic::Ordering::SeqCst),
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
                return true;
            }
        };

        let last = tailq_last(cmds);
        if last.is_null() {
            *pr = Ok(cmd_list_new());
            return true;
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
        true
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
        *pr = cmd_parse_result::Err(null_mut());

        if cmd_parse_expand_alias(cmd, pi, pr) {
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

            match cmd_parse(
                values,
                count,
                pi.file,
                pi.line.load(atomic::Ordering::SeqCst),
            ) {
                Ok(add) => {
                    let cmdlist = cmd_list_new();
                    *pr = Ok(cmdlist);
                    cmd_list_append(cmdlist, add);
                }
                Err(cause) => {
                    *pr = Err(cmd_parse_get_error(
                        pi.file,
                        pi.line.load(atomic::Ordering::SeqCst),
                        &cause,
                    )
                    .into_raw()
                    .cast());
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
            pi.line.store((*cmd).line, atomic::Ordering::SeqCst);

            cmd_parse_build_command(cmd, pi, pr);
            match *pr {
                Err(_err) => {
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
        let input: cmd_parse_input = zeroed();
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
        let input: cmd_parse_input = cmd_parse_input::default();
        let pi = pi.unwrap_or(&input);

        (&pi.flags).bitor_assign(cmd_parse_input_flags::CMD_PARSE_ONEGROUP);
        cmd_parse_from_buffer(s.as_bytes(), Some(pi))
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
        let input: cmd_parse_input = zeroed();
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
        (*cmd).line = pi.line.load(atomic::Ordering::SeqCst);
        tailq_init(&raw mut (*cmd).arguments);

        for i in 0..count {
            let mut end = 0;
            if (*values.add(i as usize)).type_ == args_type::ARGS_STRING {
                let copy = xstrdup((*values.add(i as usize)).union_.string).as_ptr();
                let mut size = strlen(copy);
                if size != 0 && *copy.add(size - 1) == b';' {
                    size -= 1;
                    *copy.add(size) = b'\0' as _;
                    if size > 0 && *copy.add(size - 1) == b'\\' {
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
                (*cmd).line = pi.line.load(atomic::Ordering::SeqCst);
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

mod lexer {
    use core::ptr::NonNull;

    use crate::{cmd_parse_state, transmute_ptr};

    pub struct Lexer<'a> {
        ps: NonNull<cmd_parse_state<'a>>,
    }
    impl<'a> Lexer<'a> {
        pub fn new(ps: NonNull<cmd_parse_state<'a>>) -> Self {
            Lexer { ps }
        }
    }

    #[derive(Copy, Clone, Debug)]
    pub enum Tok {
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

        let pi = ps.input.as_mut().unwrap();

        let mut error = args.to_string();
        error.push('\0');

        ps.error = cmd_parse_get_error(pi.file, pi.line.load(atomic::Ordering::SeqCst), &error)
            .into_raw()
            .cast();
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

fn yylex_append(buf: &mut Vec<u8>, add: &[u8]) {
    if add.len() > usize::MAX - 1 || buf.len() > usize::MAX - 1 - add.len() {
        fatalx("buffer is too big");
    }
    buf.extend_from_slice(add);
}

fn yylex_append1(buf: &mut Vec<u8>, add: u8) {
    yylex_append(buf, &[add]);
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
                assert!(count == 0 || count == 1, "unexpected read size");
                if count == 0 {
                    ch = libc::EOF;
                } else {
                    ch = buf[0] as i32;
                }
            }
            Err(_) => {
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
    if let Some(_f) = ps.f.as_mut() {
        ps.unget_buf = Some(ch);
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
                .fetch_add(1, atomic::Ordering::SeqCst);
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
        let mut buf = Vec::new();

        loop {
            yylex_append1(&mut buf, ch as u8);
            ch = yylex_getc(ps);
            if ch == libc::EOF || !libc::strchr(c!(" \t\n"), ch).is_null() {
                break;
            }
        }
        yylex_ungetc(ps, ch);

        buf.push(b'\0');
        // log_debug("%s: %s", __func__, buf.as_ptr());
        Box::leak(buf.into_boxed_slice()).as_mut_ptr()
    }
}

use lexer::Tok;

unsafe fn yylex_(ps: &mut cmd_parse_state) -> Option<Tok> {
    unsafe {
        if ps.eol != 0 {
            ps.input
                .as_mut()
                .unwrap()
                .line
                .fetch_add(1, atomic::Ordering::SeqCst);
        }
        ps.eol = 0;

        let condition = ps.condition;
        ps.condition = 0;

        loop {
            let mut ch = yylex_getc(ps);

            if ch == libc::EOF {
                // Ensure every file or string is terminated by a
                // newline. This keeps the parser simpler and avoids
                // having to add a newline to each string.
                if ps.eof != 0 {
                    break;
                }
                ps.eof = 1;
                return Some(Tok::Newline);
            }

            if ch == ' ' as i32 || ch == '\t' as i32 {
                // Ignore whitespace.
                continue;
            }

            if ch == '\r' as i32 {
                // Treat \r\n as \n.
                ch = yylex_getc(ps);
                if ch != '\n' as i32 {
                    yylex_ungetc(ps, ch);
                    ch = '\r' as i32;
                }
            }
            if ch == '\n' as i32 {
                // End of line. Update the line number.
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

            if ch == '#' as i32 {
                // #{ after a condition opens a format; anything else
                // is a comment, ignore up to the end of the line.
                let mut next = yylex_getc(ps);
                if condition != 0 && next == '{' as i32 {
                    let yylval_token = yylex_format(ps);
                    if yylval_token.is_none() {
                        return Some(Tok::Error);
                    }
                    return Some(Tok::Format(yylval_token));
                }
                while next != '\n' as i32 && next != libc::EOF {
                    next = yylex_getc(ps);
                }
                if next == '\n' as i32 {
                    ps.input
                        .as_mut()
                        .unwrap()
                        .line
                        .fetch_add(1, atomic::Ordering::SeqCst);
                    return Some(Tok::Newline);
                }
                continue;
            }

            if ch == '%' as i32 {
                // % is a condition unless it is all % or all numbers,
                // then it is a token.
                let yylval_token = yylex_get_word(ps, '%' as i32);
                let mut cp = yylval_token;
                while *cp != b'\0' {
                    if *cp != b'%' && !(*cp as u8).is_ascii_digit() {
                        break;
                    }
                    cp = cp.add(1);
                }
                if *cp == b'\0' {
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
    let mut brackets = 1;
    let mut buf = Vec::new();

    'error: {
        yylex_append(&mut buf, b"#{");
        loop {
            let mut ch = yylex_getc(ps);
            if ch == libc::EOF || ch == '\n' as i32 {
                break 'error;
            }
            if ch == '#' as i32 {
                ch = yylex_getc(ps);
                if ch == libc::EOF || ch == '\n' as i32 {
                    break 'error;
                }
                if ch == '{' as i32 {
                    brackets += 1;
                }
                yylex_append1(&mut buf, b'#');
            } else if (ch == '}' as i32)
                && brackets != 0
                && ({
                    brackets -= 1;
                    brackets == 0
                })
            {
                yylex_append1(&mut buf, ch as u8);
                break;
            }
            yylex_append1(&mut buf, ch as u8);
        }
        if brackets != 0 {
            break 'error;
        }

        buf.push(b'\0');
        // log_debug("%s: %s", __func__, buf.as_ptr());
        return NonNull::new(Box::leak(buf.into_boxed_slice()).as_mut_ptr());
    } // error:

    None
}

unsafe fn yylex_token_variable(ps: &mut cmd_parse_state, buf: &mut Vec<u8>) -> bool {
    unsafe {
        let mut namelen: usize = 0;
        let mut name: [u8; 1024] = [0; 1024];
        const SIZEOF_NAME: usize = 1024;
        let mut brackets = 0;

        let mut ch = yylex_getc(ps);
        if ch == libc::EOF {
            return false;
        }
        if ch == '{' as i32 {
            brackets = 1;
        } else {
            if !yylex_is_var(ch as u8, true) {
                yylex_append1(buf, b'$');
                yylex_ungetc(ps, ch);
                return true;
            }
            name[namelen] = ch as u8;
            namelen += 1;
        }

        loop {
            ch = yylex_getc(ps);
            if brackets != 0 && ch == '}' as i32 {
                break;
            }
            if ch == libc::EOF || !yylex_is_var(ch as u8, false) {
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

        let envent = environ_find(GLOBAL_ENVIRON, (&raw const name).cast());
        if !envent.is_null() && (*envent).value.is_some() {
            let value = (*envent).value;
            // log_debug("%s: %s -> %s", __func__, name, value);
            let value_ptr: *const u8 = transmute_ptr(value);
            let value_len = libc::strlen(transmute_ptr(value));
            yylex_append(buf, core::slice::from_raw_parts(value_ptr, value_len));
        }
        true
    }
}

unsafe fn yylex_token_tilde(ps: &mut cmd_parse_state, buf: &mut Vec<u8>) -> bool {
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
            if !envent.is_null() && (*(*envent).value.unwrap().as_ptr()) != b'\0' {
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
        let home_len = strlen(home);
        yylex_append(buf, core::slice::from_raw_parts(home, home_len));
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

        let mut buf = Vec::new();

        'error: {
            'aloop: loop {
                'next: {
                    'skip: {
                        // EOF or \n are always the end of the token.
                        if ch == libc::EOF {
                            // log_debug("%s: end at EOF", __func__);
                            break 'aloop;
                        }
                        if state == State::None && ch == '\r' as i32 {
                            ch = yylex_getc(ps);
                            if ch != '\n' as i32 {
                                yylex_ungetc(ps, ch);
                                ch = '\r' as i32;
                            }
                        }
                        if state == State::None && ch == '\n' as i32 {
                            // log_debug("%s: end at EOL", __func__);
                            break 'aloop;
                        }

                        // Whitespace or ; or } ends a token unless inside quotes.
                        if state == State::None && (ch == ' ' as i32 || ch == '\t' as i32) {
                            // log_debug("%s: end at WS", __func__);
                            break 'aloop;
                        }
                        if state == State::None && (ch == ';' as i32 || ch == '}' as i32) {
                            // log_debug("%s: end at %c", __func__, ch);
                            break 'aloop;
                        }

                        // Spaces and comments inside quotes after \n are removed but
                        // the \n is left.
                        if ch == '\n' as i32 && state != State::None {
                            yylex_append1(&mut buf, b'\n');
                            while ({
                                ch = yylex_getc(ps);
                                ch == b' ' as i32
                            }) || ch == '\t' as i32
                            {}
                            if ch != '#' as i32 {
                                continue 'aloop;
                            }
                            ch = yylex_getc(ps);
                            if !libc::strchr(c!(",#{}:"), ch).is_null() {
                                yylex_ungetc(ps, ch);
                                ch = '#' as i32;
                            } else {
                                while {
                                    ch = yylex_getc(ps);
                                    ch != '\n' as i32 && ch != libc::EOF
                                } { /* nothing */ }
                            }
                            continue 'aloop;
                        }

                        // \ ~ and $ are expanded except in single quotes.
                        if ch == '\\' as i32 && state != State::SingleQuotes {
                            if !yylex_token_escape(ps, &mut buf) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '~' as i32 && last != state && state != State::SingleQuotes {
                            if !yylex_token_tilde(ps, &mut buf) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '$' as i32 && state != State::SingleQuotes {
                            if !yylex_token_variable(ps, &mut buf) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '}' as i32 && state == State::None {
                            break 'error; /* unmatched (matched ones were handled) */
                        }

                        // ' and " starts or end quotes (and is consumed).
                        if ch == '\'' as i32 {
                            if state == State::None {
                                state = State::SingleQuotes;
                                break 'next;
                            }
                            if state == State::SingleQuotes {
                                state = State::None;
                                break 'next;
                            }
                        }
                        if ch == b'"' as i32 {
                            if state == State::None {
                                state = State::DoubleQuotes;
                                break 'next;
                            }
                            if state == State::DoubleQuotes {
                                state = State::None;
                                break 'next;
                            }
                        }

                        // Otherwise add the character to the buffer.
                        yylex_append1(&mut buf, ch as u8);
                    }
                    // skip:
                    last = state;
                }
                // next:
                ch = yylex_getc(ps);
            }
            yylex_ungetc(ps, ch);

            buf.push(b'\0');
            // log_debug("%s: %s", __func__, buf.as_ptr());
            return Box::leak(buf.into_boxed_slice()).as_mut_ptr();
        } // error:

        null_mut()
    }
}

unsafe fn yylex_token_escape(ps: &mut cmd_parse_state, buf: &mut Vec<u8>) -> bool {
    unsafe {
        #[cfg(not(target_os = "macos"))]
        const SIZEOF_M: usize = libc::_SC_MB_LEN_MAX as usize;

        // TODO determine a more stable way to get this value on mac
        #[cfg(target_os = "macos")]
        const SIZEOF_M: usize = 6; // compiled and printed constant from C

        let mut tmp: u32 = 0;
        let mut s: [u8; 9] = [0; 9];
        let mut m: [u8; SIZEOF_M] = [0; SIZEOF_M];
        let size: usize;
        let type_: i32;

        'unicode: {
            let mut ch = yylex_getc(ps);

            if ch >= '4' as i32 && ch <= '7' as i32 {
                yyerror!(ps, "invalid octal escape");
                return false;
            }
            if ch >= '0' as i32 && ch <= '3' as i32 {
                let o2 = yylex_getc(ps);
                if o2 >= '0' as i32 && o2 <= '7' as i32 {
                    let o3 = yylex_getc(ps);
                    if o3 >= '0' as i32 && o3 <= '7' as i32 {
                        ch = 64 * (ch - '0' as i32) + 8 * (o2 - '0' as i32) + (o3 - '0' as i32);
                        yylex_append1(buf, ch as u8);
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

            yylex_append1(buf, ch as u8);
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

        if (size == 4 && libc::sscanf((&raw mut s).cast(), c"%4x".as_ptr(), &raw mut tmp) != 1)
            || (size == 8 && libc::sscanf((&raw mut s).cast(), c"%8x".as_ptr(), &raw mut tmp) != 1)
        {
            yyerror!(ps, "invalid \\{} argument", type_ as u8 as char);
            return false;
        }
        let mlen = wctomb((&raw mut m).cast(), tmp as i32);
        if mlen <= 0 || mlen > SIZEOF_M as i32 {
            yyerror!(ps, "invalid \\{} argument", type_ as u8 as char);
            return false;
        }
        yylex_append(buf, &m[..mlen as usize]);

        true
    }
}

// # Notes:
//
// <https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html>
// <https://github.com/lalrpop/lalrpop/blob/master/README.md>
