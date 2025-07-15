use crate::*;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;

use crate::cmd_parse::lexer;
use crate::cmd_parse::lexer::Tok;
use crate::cmd_parse::{cmd_parse_argument_type, cmd_parse_command, cmd_parse_state, yystype_elif};
use crate::compat::queue::{
    tailq_concat, tailq_empty, tailq_init, tailq_insert_head, tailq_insert_tail, tailq_remove,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Lowest = 0,
    Statement = 1, // newline
    Command = 2,   // semicolon
    Argument = 3,  // tokens, equals
}

struct PrattParser<'a> {
    lexer: lexer::Lexer<'a>,
    current_token: Option<Tok>,
    peek_token: Option<Tok>,
    ps: NonNull<cmd_parse_state<'a>>,
    errors: Vec<String>,
}

impl<'a> PrattParser<'a> {
    fn new(ps: NonNull<cmd_parse_state<'a>>) -> Self {
        let mut lexer = lexer::Lexer::new(ps);
        let current_token = lexer.next().and_then(|r| r.ok()).map(|t| t.1);
        let peek_token = lexer.next().and_then(|r| r.ok()).map(|t| t.1);

        PrattParser {
            lexer,
            current_token,
            peek_token,
            ps,
            errors: Vec::new(),
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.peek_token.take();
        self.peek_token = self.lexer.next().and_then(|r| r.ok()).map(|t| t.1);
    }

    fn current_precedence(&self) -> Precedence {
        match self.current_token {
            Some(Tok::Newline) => Precedence::Statement,
            Some(Tok::Semicolon) => Precedence::Command,
            Some(Tok::Token(_)) | Some(Tok::Equals(_)) | Some(Tok::Format(_)) => {
                Precedence::Argument
            }
            _ => Precedence::Lowest,
        }
    }

    fn peek_precedence(&self) -> Precedence {
        match self.peek_token {
            Some(Tok::Newline) => Precedence::Statement,
            Some(Tok::Semicolon) => Precedence::Command,
            Some(Tok::Token(_)) | Some(Tok::Equals(_)) | Some(Tok::Format(_)) => {
                Precedence::Argument
            }
            _ => Precedence::Lowest,
        }
    }

    fn expect_token(&mut self, expected: Tok) -> Result<(), String> {
        if std::mem::discriminant(&self.current_token.unwrap_or(Tok::Error))
            == std::mem::discriminant(&expected)
        {
            self.next_token();
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, got {:?}",
                expected, self.current_token
            ))
        }
    }

    fn parse_lines(&mut self) -> Result<Option<&'static mut cmd_parse_commands>, ()> {
        if self.current_token.is_none() {
            return Ok(None);
        }

        let statements = self.parse_statements()?;
        Ok(Some(statements))
    }

    fn parse_statements(&mut self) -> Result<&'static mut cmd_parse_commands, ()> {
        unsafe {
            let mut statements = cmd_parse_new_commands();

            // Handle empty input
            if self.current_token.is_none() {
                return Ok(statements);
            }

            // Parse first statement
            let mut stmt = self.parse_statement()?;
            if !tailq_empty(stmt) {
                tailq_concat(statements, stmt);
            }
            free_(stmt);

            // Parse additional statements separated by newlines
            while let Some(Tok::Newline) = self.current_token {
                self.next_token();
                if self.current_token.is_none() {
                    break;
                }

                let stmt = self.parse_statement()?;
                if !tailq_empty(stmt) {
                    tailq_concat(statements, stmt);
                }
                free_(stmt);
            }

            Ok(statements)
        }
    }

    fn parse_statement(&mut self) -> Result<&'static mut cmd_parse_commands, ()> {
        // Handle empty statement
        if matches!(self.current_token, Some(Tok::Newline) | None) {
            return Ok(cmd_parse_new_commands());
        }

        // Handle assignment
        if let Some(Tok::Equals(equals_ptr)) = self.current_token {
            self.handle_assignment(equals_ptr)?;
            self.next_token();
            return Ok(cmd_parse_new_commands());
        }

        // Handle hidden assignment
        if let Some(Tok::Hidden) = self.current_token {
            self.next_token();
            if let Some(Tok::Equals(equals_ptr)) = self.current_token {
                self.handle_hidden_assignment(equals_ptr)?;
                self.next_token();
                return Ok(cmd_parse_new_commands());
            } else {
                return Err(());
            }
        }

        // Handle conditions and commands
        match self.current_token {
            Some(Tok::If) => self.parse_condition(),
            _ => self.parse_commands(),
        }
    }

    fn handle_assignment(&mut self, equals_ptr: Option<NonNull<u8>>) -> Result<(), ()> {
        unsafe {
            let flags = &(*self.ps.as_ptr()).input.as_ref().unwrap().flags;
            if !flags.intersects(cmd_parse_input_flags::CMD_PARSE_PARSEONLY)
                && (*self.ps.as_ptr())
                    .scope
                    .as_ref()
                    .map_or(true, |scope| scope.flag != 0)
            {
                if let Some(ptr) = equals_ptr {
                    environ_put(GLOBAL_ENVIRON, ptr.as_ptr(), 0);
                }
            }
            if let Some(ptr) = equals_ptr {
                free_(ptr.as_ptr());
            }
        }
        Ok(())
    }

    fn handle_hidden_assignment(&mut self, equals_ptr: Option<NonNull<u8>>) -> Result<(), ()> {
        unsafe {
            let flags = &(*self.ps.as_ptr()).input.as_ref().unwrap().flags;
            if !flags.intersects(cmd_parse_input_flags::CMD_PARSE_PARSEONLY)
                && (*self.ps.as_ptr())
                    .scope
                    .as_ref()
                    .map_or(true, |scope| scope.flag != 0)
            {
                if let Some(ptr) = equals_ptr {
                    environ_put(GLOBAL_ENVIRON, ptr.as_ptr(), ENVIRON_HIDDEN);
                }
            }
            if let Some(ptr) = equals_ptr {
                free_(ptr.as_ptr());
            }
        }
        Ok(())
    }

    fn parse_condition(&mut self) -> Result<&'static mut cmd_parse_commands, ()> {
        unsafe {
            // This is a simplified condition parser - full implementation would handle
            // %if/%else/%elif/%endif with proper nesting
            if let Some(Tok::If) = self.current_token {
                self.next_token();

                // Parse condition expression (simplified)
                let condition_result = self.parse_condition_expr()?;

                // Expect newline after condition
                if let Some(Tok::Newline) = self.current_token {
                    self.next_token();
                }

                // Parse statements inside condition
                let mut statements = cmd_parse_new_commands();
                while !matches!(self.current_token, Some(Tok::Endif) | None) {
                    let stmt = self.parse_statement()?;
                    if !tailq_empty(stmt) {
                        tailq_concat(statements, stmt);
                    }
                    free_(stmt);
                }

                // Expect %endif
                if let Some(Tok::Endif) = self.current_token {
                    self.next_token();
                }

                if condition_result {
                    Ok(statements)
                } else {
                    cmd_parse_free_commands(statements);
                    Ok(cmd_parse_new_commands())
                }
            } else {
                Err(())
            }
        }
    }

    fn parse_condition_expr(&mut self) -> Result<bool, ()> {
        // Simplified condition parsing - just look for format tokens
        if let Some(Tok::Format(format_ptr)) = self.current_token {
            self.next_token();
            if let Some(ptr) = format_ptr {
                // Expand format and evaluate
                let result = unsafe { format_true(ptr.as_ptr()) };
                unsafe {
                    free_(ptr.as_ptr());
                }
                Ok(result != 0)
            } else {
                Ok(false)
            }
        } else {
            Ok(true) // Default to true for simple cases
        }
    }

    fn parse_commands(&mut self) -> Result<&'static mut cmd_parse_commands, ()> {
        let mut commands = cmd_parse_new_commands();

        // Parse first command
        let cmd = self.parse_command()?;
        unsafe {
            if !tailq_empty(&cmd.arguments) {
                tailq_insert_tail(commands, cmd);
            } else {
                cmd_parse_free_command(cmd);
            }
        }

        // Parse additional commands separated by semicolons
        while let Some(Tok::Semicolon) = self.current_token {
            self.next_token();

            // Check for condition after semicolon
            if let Some(Tok::If) = self.current_token {
                let condition_cmds = self.parse_condition()?;
                unsafe {
                    tailq_concat(commands, condition_cmds);
                }
                unsafe {
                    free_(condition_cmds);
                }
            } else if self.current_token.is_some()
                && !matches!(self.current_token, Some(Tok::Newline))
            {
                let cmd = self.parse_command()?;
                unsafe {
                    if !tailq_empty(&cmd.arguments) {
                        tailq_insert_tail(commands, cmd);
                    } else {
                        cmd_parse_free_command(cmd);
                    }
                }
            }
        }

        Ok(commands)
    }

    fn parse_command(&mut self) -> Result<&'static mut cmd_parse_command, ()> {
        unsafe {
            let mut command = xcalloc1::<cmd_parse_command>();
            (*command).line = (*self.ps.as_ptr())
                .input
                .as_mut()
                .unwrap()
                .line
                .load(Ordering::SeqCst);
            tailq_init(&mut (*command).arguments);

            // Handle optional assignment first
            if let Some(Tok::Equals(equals_ptr)) = self.current_token {
                self.handle_assignment(equals_ptr)?;
                self.next_token();
            }

            // Parse command token
            if let Some(Tok::Token(token_ptr)) = self.current_token {
                self.next_token();
                if let Some(ptr) = token_ptr {
                    let mut arg = xcalloc1::<cmd_parse_argument>();
                    (*arg).type_ = cmd_parse_argument_type::String(ptr.as_ptr());
                    tailq_insert_tail(&mut (*command).arguments, arg);
                }
            }

            // Parse arguments
            while let Some(token) = self.current_token {
                match token {
                    Tok::Token(token_ptr) => {
                        self.next_token();
                        if let Some(ptr) = token_ptr {
                            let mut arg = xcalloc1::<cmd_parse_argument>();
                            (*arg).type_ = cmd_parse_argument_type::String(ptr.as_ptr());
                            tailq_insert_tail(&mut (*command).arguments, arg);
                        }
                    }
                    Tok::Equals(equals_ptr) => {
                        self.next_token();
                        if let Some(ptr) = equals_ptr {
                            let mut arg = xcalloc1::<cmd_parse_argument>();
                            (*arg).type_ = cmd_parse_argument_type::String(ptr.as_ptr());
                            tailq_insert_tail(&mut (*command).arguments, arg);
                        }
                    }
                    Tok::LeftBrace => {
                        self.next_token();
                        let statements = self.parse_argument_statements()?;
                        let mut arg = xcalloc1::<cmd_parse_argument>();
                        (*arg).type_ = cmd_parse_argument_type::Commands(statements);
                        tailq_insert_tail(&mut (*command).arguments, arg);
                    }
                    _ => break,
                }
            }

            Ok(command)
        }
    }

    fn parse_argument_statements(&mut self) -> Result<&'static mut cmd_parse_commands, ()> {
        let statement = self.parse_statement()?;

        if let Some(Tok::RightBrace) = self.current_token {
            self.next_token();
            Ok(statement)
        } else {
            // Parse multiple statements
            let mut statements = statement;
            // Parse additional statements separated by newlines
            while let Some(Tok::Newline) = self.current_token {
                self.next_token();
                if matches!(self.current_token, Some(Tok::RightBrace) | None) {
                    break;
                }

                let stmt = self.parse_statement()?;
                unsafe {
                    if !tailq_empty(stmt) {
                        tailq_concat(statements, stmt);
                    }
                    free_(stmt);
                }
            }

            if let Some(Tok::RightBrace) = self.current_token {
                self.next_token();
            }

            Ok(statements)
        }
    }
}

pub fn parse_lines(
    ps: &mut cmd_parse_state,
) -> Result<Option<&'static mut cmd_parse_commands>, ()> {
    let mut parser = PrattParser::new(NonNull::new(ps).unwrap());
    parser.parse_lines()
}

