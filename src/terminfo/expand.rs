// Copyright 2019 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Parameterized string expansion

use std::array::from_fn;
use std::iter::repeat_n;

#[derive(Clone, Copy, PartialEq)]
enum States {
    Nothing,
    Delay,
    Percent,
    SetVar,
    GetVar,
    PushParam,
    CharConstant,
    CharClose,
    IntConstant(i32),
    FormatPattern(Flags, FormatState),
    SeekIfElse(usize),
    SeekIfElsePercent(usize),
    SeekIfEnd(usize),
    SeekIfEndPercent(usize),
}

#[derive(Copy, PartialEq, Clone)]
enum FormatState {
    Flags,
    Width,
    Precision,
}

/// Types of parameters a capability can use
#[derive(Clone)]
pub enum Parameter {
    Number(i32),
    String(Vec<u8>),
}

impl From<i32> for Parameter {
    fn from(value: i32) -> Parameter {
        Parameter::Number(value)
    }
}

impl From<&[u8]> for Parameter {
    fn from(value: &[u8]) -> Parameter {
        Parameter::String(value.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for Parameter {
    fn from(value: &[u8; N]) -> Parameter {
        Parameter::String(value.to_vec())
    }
}

impl From<&str> for Parameter {
    fn from(value: &str) -> Parameter {
        Parameter::String(value.as_bytes().to_vec())
    }
}

/// Error reported when expanding a string
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Not enough stack elements for operator {0}")]
    StackUnderflow(char),
    #[error("Parameter type not expected by operator {0}")]
    TypeMismatch(char),
    #[error("Unrecognized format option: {0}")]
    UnrecognizedFormatOption(char),
    #[error("Invalid variable name: {0}")]
    InvalidVariableName(char),
    #[error("Invalid parameter index: {0}")]
    InvalidParameterIndex(char),
    #[error("Malformed character constant")]
    MalformedCharacterConstant,
    #[error("Integer constant too large")]
    IntegerConstantOverflow,
    #[error("Integer constant malformed")]
    MalformedIntegerConstant,
    #[error("Overflow in format width")]
    FormatWidthOverflow,
    #[error("Overflow in format precision")]
    FormatPrecisionOverflow,
    #[error("Unexpected type for format")]
    FormatTypeMismatch,
}

/// Context for variable expansion
///
/// To be compatible with ncurses, the `ExpandContext` instance should be the same
/// for the same terminal.
pub struct ExpandContext {
    /// Static variables A-Z
    static_variables: [Parameter; 26],
}

impl ExpandContext {
    /// Return a newly initialized ExpandContext
    pub fn new() -> Self {
        Self {
            static_variables: from_fn(|_| Parameter::from(0)),
        }
    }

    /// Expand a parameterized capability
    ///
    /// # Arguments
    /// * `cap`    - string to expand
    /// * `params` - vector of params for %p1 etc
    pub fn expand(&mut self, cap: &[u8], params: &[Parameter]) -> Result<Vec<u8>, Error> {
        let mut state = States::Nothing;

        // expanded cap will only rarely be larger than the cap itself
        let mut output = Vec::with_capacity(cap.len());

        let mut stack = Vec::new();

        // Dynamic variables a-z
        let mut dynamic_variables: [Parameter; 26] = from_fn(|_| Parameter::from(0));

        // Copy parameters into a local vector for mutability
        let mut mparams = params.to_vec();

        // The increment should only be done once
        let mut incremented = false;

        // Make sure there are at least 9 parameters
        while mparams.len() < 9 {
            mparams.push(Parameter::from(0));
        }

        for &c in cap.iter() {
            let cur = c as char;
            let mut old_state = state;
            match state {
                States::Nothing => {
                    if cur == '%' {
                        state = States::Percent;
                    } else if cur == '$' {
                        state = States::Delay;
                    } else {
                        output.push(c);
                    }
                }
                States::Delay => {
                    old_state = States::Nothing;
                    if cur == '>' {
                        state = States::Nothing;
                    }
                }
                States::Percent => {
                    match cur {
                        '%' => {
                            output.push(c);
                            state = States::Nothing;
                        }
                        'c' => {
                            match stack.pop() {
                                // if c is 0, use 0200 (128) for ncurses compatibility
                                Some(Parameter::Number(0)) => output.push(128u8),
                                // Don't check bounds. ncurses just casts and truncates.
                                Some(Parameter::Number(c)) => output.push(c as u8),
                                Some(_) => return Err(Error::TypeMismatch(cur)),
                                None => return Err(Error::StackUnderflow(cur)),
                            }
                        }
                        'p' => state = States::PushParam,
                        'P' => state = States::SetVar,
                        'g' => state = States::GetVar,
                        '\'' => state = States::CharConstant,
                        '{' => state = States::IntConstant(0),
                        'l' => match stack.pop() {
                            Some(Parameter::String(s)) => {
                                stack.push(Parameter::from(s.len() as i32));
                            }
                            Some(_) => return Err(Error::TypeMismatch(cur)),
                            None => return Err(Error::StackUnderflow(cur)),
                        },
                        '+' | '-' | '*' | '/' | '|' | '&' | '^' | 'm' => {
                            match (stack.pop(), stack.pop()) {
                                (Some(Parameter::Number(y)), Some(Parameter::Number(x))) => {
                                    let result = match cur {
                                        '+' => x + y,
                                        '-' => x - y,
                                        '*' => x * y,
                                        '/' => x / y,
                                        '|' => x | y,
                                        '&' => x & y,
                                        '^' => x ^ y,
                                        'm' => x % y,
                                        _ => unreachable!("logic error"),
                                    };
                                    stack.push(Parameter::from(result));
                                }
                                (Some(_), Some(_)) => return Err(Error::TypeMismatch(cur)),
                                _ => return Err(Error::StackUnderflow(cur)),
                            }
                        }
                        '=' | '>' | '<' | 'A' | 'O' => match (stack.pop(), stack.pop()) {
                            (Some(Parameter::Number(y)), Some(Parameter::Number(x))) => {
                                let result = match cur {
                                    '=' => x == y,
                                    '<' => x < y,
                                    '>' => x > y,
                                    'A' => x > 0 && y > 0,
                                    'O' => x > 0 || y > 0,
                                    _ => unreachable!("logic error"),
                                };
                                stack.push(Parameter::from(i32::from(result)));
                            }
                            (Some(_), Some(_)) => return Err(Error::TypeMismatch(cur)),
                            _ => return Err(Error::StackUnderflow(cur)),
                        },
                        '!' | '~' => match stack.pop() {
                            Some(Parameter::Number(x)) => {
                                stack.push(Parameter::Number(match cur {
                                    '!' if x > 0 => 0,
                                    '!' => 1,
                                    '~' => !x,
                                    _ => unreachable!("logic error"),
                                }));
                            }
                            Some(_) => return Err(Error::TypeMismatch(cur)),
                            None => return Err(Error::StackUnderflow(cur)),
                        },
                        'i' => match (&mparams[0], &mparams[1]) {
                            (&Parameter::Number(x), &Parameter::Number(y)) => {
                                if !incremented {
                                    mparams[0] = Parameter::from(x + 1);
                                    mparams[1] = Parameter::from(y + 1);
                                    incremented = true;
                                }
                            }
                            (_, _) => return Err(Error::TypeMismatch(cur)),
                        },

                        // printf-style support for %doxXs
                        'd' | 'o' | 'x' | 'X' | 's' => {
                            if let Some(arg) = stack.pop() {
                                let flags = Flags::default();
                                let result = format(arg, cur, flags)?;
                                output.extend(result);
                            } else {
                                return Err(Error::StackUnderflow(cur));
                            }
                        }
                        ':' | '#' | ' ' | '.' | '0'..='9' => {
                            let mut flags = Flags::default();
                            let mut fstate = FormatState::Flags;
                            match cur {
                                ':' => (),
                                '#' => flags.alternate = true,
                                ' ' => flags.space = true,
                                '.' => fstate = FormatState::Precision,
                                '0'..='9' => {
                                    flags.width = cur as u16 - '0' as u16;
                                    fstate = FormatState::Width;
                                }
                                _ => unreachable!("logic error"),
                            }
                            state = States::FormatPattern(flags, fstate);
                        }

                        // conditionals
                        '?' | ';' => (),
                        't' => match stack.pop() {
                            Some(Parameter::Number(0)) => state = States::SeekIfElse(0),
                            Some(Parameter::Number(_)) => (),
                            Some(_) => return Err(Error::TypeMismatch(cur)),
                            None => return Err(Error::StackUnderflow(cur)),
                        },
                        'e' => state = States::SeekIfEnd(0),
                        c => return Err(Error::UnrecognizedFormatOption(c)),
                    }
                }
                States::PushParam => {
                    // params are 1-indexed
                    let index = match cur {
                        '1'..='9' => cur as usize - '1' as usize,
                        _ => return Err(Error::InvalidParameterIndex(cur)),
                    };
                    stack.push(mparams[index].clone());
                }
                States::SetVar => {
                    let Some(arg) = stack.pop() else {
                        return Err(Error::StackUnderflow('P'));
                    };
                    match cur {
                        'A'..='Z' => self.static_variables[usize::from((cur as u8) - b'A')] = arg,
                        'a'..='z' => dynamic_variables[usize::from((cur as u8) - b'a')] = arg,
                        _ => return Err(Error::InvalidVariableName(cur)),
                    };
                }
                States::GetVar => {
                    let value = match cur {
                        'A'..='Z' => &self.static_variables[usize::from((cur as u8) - b'A')],
                        'a'..='z' => &dynamic_variables[usize::from((cur as u8) - b'a')],
                        _ => return Err(Error::InvalidVariableName(cur)),
                    };
                    stack.push(value.clone());
                }
                States::CharConstant => {
                    stack.push(Parameter::from(i32::from(c)));
                    state = States::CharClose;
                }
                States::CharClose => {
                    if cur != '\'' {
                        return Err(Error::MalformedCharacterConstant);
                    }
                }
                States::IntConstant(i) => {
                    if cur == '}' {
                        stack.push(Parameter::from(i));
                        state = States::Nothing;
                    } else if let Some(digit) = cur.to_digit(10) {
                        match i
                            .checked_mul(10)
                            .and_then(|i_ten| i_ten.checked_add(digit as i32))
                        {
                            Some(i) => {
                                state = States::IntConstant(i);
                                old_state = States::Nothing;
                            }
                            None => return Err(Error::IntegerConstantOverflow),
                        }
                    } else {
                        return Err(Error::MalformedIntegerConstant);
                    }
                }
                States::FormatPattern(ref mut flags, ref mut fstate) => {
                    old_state = States::Nothing;
                    match (*fstate, cur) {
                        (_, 'd') | (_, 'o') | (_, 'x') | (_, 'X') | (_, 's') => {
                            if let Some(arg) = stack.pop() {
                                let res = format(arg, cur, *flags)?;
                                output.extend(res);
                                // will cause state to go to States::Nothing
                                old_state = States::FormatPattern(*flags, *fstate);
                            } else {
                                return Err(Error::StackUnderflow(cur));
                            }
                        }
                        (FormatState::Flags, '#') => {
                            flags.alternate = true;
                        }
                        (FormatState::Flags, '-') => {
                            flags.left = true;
                        }
                        (FormatState::Flags, '+') => {
                            flags.sign = true;
                        }
                        (FormatState::Flags, ' ') => {
                            flags.space = true;
                        }
                        (FormatState::Flags, '0'..='9') => {
                            flags.width = cur as u16 - '0' as u16;
                            *fstate = FormatState::Width;
                        }
                        (FormatState::Width, '0'..='9') => {
                            flags.width = match flags
                                .width
                                .checked_mul(10)
                                .and_then(|w| w.checked_add(cur as u16 - '0' as u16))
                            {
                                Some(width) => width,
                                None => return Err(Error::FormatWidthOverflow),
                            }
                        }
                        (FormatState::Width, '.') | (FormatState::Flags, '.') => {
                            *fstate = FormatState::Precision;
                        }
                        (FormatState::Precision, '0'..='9') => {
                            flags.precision = match flags
                                .precision
                                .unwrap_or(0)
                                .checked_mul(10)
                                .and_then(|w| w.checked_add(cur as u16 - '0' as u16))
                            {
                                Some(precision) => Some(precision),
                                None => return Err(Error::FormatPrecisionOverflow),
                            }
                        }
                        _ => return Err(Error::UnrecognizedFormatOption(cur)),
                    }
                }
                States::SeekIfElse(level) => {
                    if cur == '%' {
                        state = States::SeekIfElsePercent(level);
                    }
                    old_state = States::Nothing;
                }
                States::SeekIfElsePercent(level) => {
                    if cur == ';' {
                        if level == 0 {
                            state = States::Nothing;
                        } else {
                            state = States::SeekIfElse(level - 1);
                        }
                    } else if cur == 'e' && level == 0 {
                        state = States::Nothing;
                    } else if cur == '?' {
                        state = States::SeekIfElse(level + 1);
                    } else {
                        state = States::SeekIfElse(level);
                    }
                }
                States::SeekIfEnd(level) => {
                    if cur == '%' {
                        state = States::SeekIfEndPercent(level);
                    }
                    old_state = States::Nothing;
                }
                States::SeekIfEndPercent(level) => {
                    if cur == ';' {
                        if level == 0 {
                            state = States::Nothing;
                        } else {
                            state = States::SeekIfEnd(level - 1);
                        }
                    } else if cur == '?' {
                        state = States::SeekIfEnd(level + 1);
                    } else {
                        state = States::SeekIfEnd(level);
                    }
                }
            }
            if state == old_state {
                state = States::Nothing;
            }
        }
        Ok(output)
    }
}

#[derive(Copy, PartialEq, Clone, Default)]
struct Flags {
    width: u16,
    precision: Option<u16>,
    alternate: bool,
    left: bool,
    sign: bool,
    space: bool,
}

fn format(val: Parameter, op: char, flags: Flags) -> Result<Vec<u8>, Error> {
    let mut s = match val {
        Parameter::Number(d) => {
            match op {
                'd' => match flags.precision {
                    Some(precision) => {
                        if flags.sign {
                            format!("{d:+0prec$}", prec = usize::from(precision + 1))
                        } else if d < 0 {
                            format!("{d:0prec$}", prec = usize::from(precision + 1))
                        } else if flags.space {
                            format!(" {d:0prec$}", prec = precision.into())
                        } else {
                            format!("{d:0prec$}", prec = precision.into())
                        }
                    }
                    None => {
                        if flags.sign {
                            format!("{d:+}")
                        } else if d < 0 {
                            format!("{d}")
                        } else if flags.space {
                            format!(" {d}")
                        } else {
                            format!("{d}")
                        }
                    }
                },
                'o' => match flags.precision {
                    Some(precision) => {
                        if flags.alternate {
                            // Leading octal zero counts against precision.
                            format!("0{d:0prec$o}", prec = precision.saturating_sub(1).into())
                        } else {
                            format!("{d:0prec$o}", prec = precision.into())
                        }
                    }
                    None => {
                        if flags.alternate {
                            format!("0{d:o}")
                        } else {
                            format!("{d:o}")
                        }
                    }
                },
                'x' => match flags.precision {
                    Some(precision) => {
                        if flags.alternate && d != 0 {
                            format!("0x{d:0prec$x}", prec = precision.into())
                        } else {
                            format!("{d:0prec$x}", prec = precision.into())
                        }
                    }
                    None => {
                        if flags.alternate && d != 0 {
                            format!("0x{d:x}")
                        } else {
                            format!("{d:x}")
                        }
                    }
                },
                'X' => match flags.precision {
                    Some(precision) => {
                        if flags.alternate && d != 0 {
                            format!("0X{d:0prec$X}", prec = precision.into())
                        } else {
                            format!("{d:0prec$X}", prec = precision.into())
                        }
                    }
                    None => {
                        if flags.alternate && d != 0 {
                            format!("0X{d:X}")
                        } else {
                            format!("{d:X}")
                        }
                    }
                },
                _ => return Err(Error::FormatTypeMismatch),
            }
            .into_bytes()
        }
        Parameter::String(mut s) => match op {
            's' => {
                if let Some(precision) = flags.precision
                    && let precision = usize::from(precision)
                    && precision < s.len()
                {
                    s.truncate(precision);
                }
                s
            }
            _ => return Err(Error::FormatTypeMismatch),
        },
    };
    if usize::from(flags.width) > s.len() {
        let n = usize::from(flags.width) - s.len();
        if flags.left {
            s.extend(repeat_n(b' ', n));
        } else {
            let mut s_ = Vec::with_capacity(usize::from(flags.width));
            s_.extend(repeat_n(b' ', n));
            s_.extend(s);
            s = s_;
        }
    }
    Ok(s)
}

#[cfg(test)]
mod test {
    use super::{Error, ExpandContext, Parameter};

    /// Compare the result of `expand()` to the expected string
    fn assert_str(actual: Result<Vec<u8>, Error>, expected: &str) {
        assert_eq!(str::from_utf8(&actual.unwrap()).unwrap(), expected);
    }

    #[test]
    fn multiple_parameters() {
        let mut expand_context = ExpandContext::new();
        assert_str(
            expand_context.expand(
                b"%p1%p2%p3%p4%p5%p6%p7%p8%p9%d%d%d%d%d%s%s%s%d",
                &[
                    Parameter::from(1),
                    Parameter::from(b"Two"),
                    Parameter::from(b"Three".as_slice()),
                    Parameter::from("Four"),
                    Parameter::from(5),
                    Parameter::from(6),
                    Parameter::from(7),
                    Parameter::from(8),
                    Parameter::from(9),
                ],
            ),
            "98765FourThreeTwo1",
        );
    }

    #[test]
    fn delay_ignored() {
        let mut expand_context = ExpandContext::new();
        assert_str(
            expand_context.expand(b"%p1%d$<5*/>%p1%d", &[Parameter::from(42)]),
            "4242",
        );
    }

    #[test]
    fn percent_escape() {
        let mut expand_context = ExpandContext::new();
        assert_str(
            expand_context.expand(b"%p1%%%%%d", &[Parameter::from(42)]),
            "%%42",
        );
    }

    #[test]
    fn char_output() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(
                b"%p1%c%p2%c%p3%c",
                &[
                    Parameter::from(42),
                    Parameter::from(0),
                    Parameter::from(257)
                ],
            ),
            Ok(vec![42, 128, 1]),
        );
    }

    #[test]
    fn type_mismatch_expected_number() {
        let mut expand_context = ExpandContext::new();
        for op in "c!~+-*/|&^m=><AOit".chars() {
            let cap = format!("%p1%p2%{op}");
            assert_eq!(
                expand_context.expand(
                    cap.as_bytes(),
                    &[Parameter::from(42), Parameter::from("word")]
                ),
                Err(Error::TypeMismatch(op)),
                "Failed for %{op}"
            );
        }
    }

    #[test]
    fn type_mismatch_expected_string() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%p1%l", &[Parameter::from(42)]),
            Err(Error::TypeMismatch('l'))
        );
    }

    #[test]
    fn stack_underflow_unary() {
        let mut expand_context = ExpandContext::new();
        for op in "cl!~doxXst".chars() {
            let cap = format!("%{op}");
            assert_eq!(
                expand_context.expand(cap.as_bytes(), &[]),
                Err(Error::StackUnderflow(op)),
                "Failed for %{op}"
            );
        }
    }

    #[test]
    fn stack_underflow_format() {
        let mut expand_context = ExpandContext::new();
        for op in "doxXs".chars() {
            let cap = format!("%:{op}");
            assert_eq!(
                expand_context.expand(cap.as_bytes(), &[]),
                Err(Error::StackUnderflow(op)),
                "Failed for %{op}"
            );
        }
    }

    #[test]
    fn stack_underflow_binary() {
        let mut expand_context = ExpandContext::new();
        for op in "+-*/|&^m=><AO".chars() {
            let cap = format!("%p1%{op}");
            assert_eq!(
                expand_context.expand(cap.as_bytes(), &[Parameter::from(42)]),
                Err(Error::StackUnderflow(op)),
                "Failed for %{op}"
            );
        }
    }

    #[test]
    fn stack_underflow_variable() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%P1", &[]),
            Err(Error::StackUnderflow('P'))
        );
    }

    #[test]
    fn variable_persistence() {
        let mut expand_context = ExpandContext::new();
        assert_str(
            expand_context.expand(
                b"%p1%PA%p2%PZ%p3%Pa%p4%Pz%gA%d%gZ%d%ga%d%gz%d",
                &[
                    Parameter::from(1),
                    Parameter::from(2),
                    Parameter::from(3),
                    Parameter::from(4),
                ],
            ),
            "1234",
        );
        assert_str(expand_context.expand(b"%gA%d%gZ%d%ga%d%gz%d", &[]), "1200");
    }

    #[test]
    fn variable_bad_name() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%p1%P7", &[Parameter::from(42)]),
            Err(Error::InvalidVariableName('7'))
        );
        assert_eq!(
            expand_context.expand(b"%g8", &[]),
            Err(Error::InvalidVariableName('8'))
        );
    }

    #[test]
    fn constants() {
        let mut expand_context = ExpandContext::new();
        assert_str(expand_context.expand(b"%{456}%d %'A'%d", &[]), "456 65");
    }

    #[test]
    fn bad_char_constant() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%'ab'", &[]),
            Err(Error::MalformedCharacterConstant)
        );
    }

    #[test]
    fn bad_integer_constant() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%{2b}", &[]),
            Err(Error::MalformedIntegerConstant)
        );
    }

    #[test]
    fn integer_constant_overflow() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%{2147483648}", &[]),
            Err(Error::IntegerConstantOverflow)
        );
    }

    #[test]
    fn string_length() {
        let mut expand_context = ExpandContext::new();
        assert_str(
            expand_context.expand(b"%p1%l%d", &[Parameter::from("Hello, World!")]),
            "13",
        );
    }

    #[test]
    fn numeric_binary_operations() {
        let tests = [
            (12, '+', 29, "41"),
            (35, '-', 7, "28"),
            (3, '*', 16, "48"),
            (70, '/', 3, "23"),
            (3, '|', 5, "7"),
            (15, '&', 35, "3"),
            (15, '^', 35, "44"),
            (101, 'm', 7, "3"),
            (5, '=', 7, "0"),
            (15, '=', 15, "1"),
            (17, '<', 8, "0"),
            (17, '<', 50, "1"),
            (17, '>', 8, "1"),
            (17, '>', 50, "0"),
            (0, 'A', 0, "0"),
            (15, 'A', 0, "0"),
            (0, 'A', 9, "0"),
            (15, 'A', 32, "1"),
            (0, 'O', 0, "0"),
            (15, 'O', 0, "1"),
            (0, 'O', 9, "1"),
            (15, 'O', 32, "1"),
        ];
        let mut expand_context = ExpandContext::new();
        for (operand1, operation, operand2, expect) in tests {
            let cap = format!("%p1%p2%{operation}%d");
            assert_str(
                expand_context.expand(
                    cap.as_bytes(),
                    &[Parameter::from(operand1), Parameter::from(operand2)],
                ),
                expect,
            );
        }
    }

    #[test]
    fn negation() {
        let mut expand_context = ExpandContext::new();
        assert_str(
            expand_context.expand(
                b"%p1%!%d %p2%!%d %p1%~%d %p2%~%d",
                &[Parameter::from(0), Parameter::from(15)],
            ),
            "1 0 -1 -16",
        );
    }

    #[test]
    fn increment() {
        let mut expand_context = ExpandContext::new();
        assert_str(
            expand_context.expand(
                b"%i%p1%d_%p2%d_%p3%d_%i%p1%d_%p2%d_%p3%d",
                &[
                    Parameter::from(10),
                    Parameter::from(15),
                    Parameter::from(20),
                ],
            ),
            "11_16_20_11_16_20",
        );
    }

    #[test]
    fn conditional_if_then() {
        let mut expand_context = ExpandContext::new();
        let cap = b"%p1%p2%?%<%tless%;";
        assert_str(
            expand_context.expand(cap, &[Parameter::from(1), Parameter::from(2)]),
            "less",
        );
        assert_str(
            expand_context.expand(cap, &[Parameter::from(2), Parameter::from(1)]),
            "",
        );
    }

    #[test]
    fn conditional_if_then_else() {
        let mut expand_context = ExpandContext::new();
        let cap = b"%p1%p2%?%<%tless%emore%;";
        assert_str(
            expand_context.expand(cap, &[Parameter::from(1), Parameter::from(2)]),
            "less",
        );
        assert_str(
            expand_context.expand(cap, &[Parameter::from(2), Parameter::from(1)]),
            "more",
        );
    }

    #[test]
    fn conditional_nested() {
        let mut expand_context = ExpandContext::new();
        let cap = b"%?%p1%t+%?%p2%t+%e-%;%e-%?%p2%t+%e-%;%;";
        assert_str(
            expand_context.expand(cap, &[Parameter::from(0), Parameter::from(0)]),
            "--",
        );
        assert_str(
            expand_context.expand(cap, &[Parameter::from(0), Parameter::from(1)]),
            "-+",
        );
        assert_str(
            expand_context.expand(cap, &[Parameter::from(1), Parameter::from(0)]),
            "+-",
        );
        assert_str(
            expand_context.expand(cap, &[Parameter::from(1), Parameter::from(1)]),
            "++",
        );
    }

    #[test]
    fn format_flags() {
        let tests = [
            (63, "%x", "3f"),
            (63, "%#x", "0x3f"),
            (63, "%6x", "    3f"),
            (63, "%:-6x", "3f    "),
            (63, "%:+d", "+63"),
            (63, "%: d", " 63"),
            (63, "%p1%:-+ #10.5x", "0x0003f   "),
        ];
        let mut expand_context = ExpandContext::new();
        for (param1, format, expected) in tests {
            let cap = format!("%p1{format}");
            assert_str(
                expand_context.expand(cap.as_bytes(), &[Parameter::from(param1)]),
                expected,
            );
        }
    }

    #[test]
    fn format_bad_flag() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%p1%:^x", &[Parameter::from(63)]),
            Err(Error::UnrecognizedFormatOption('^'))
        );
    }

    #[test]
    fn format_decimal() {
        let tests = [
            (42, "%d", "42"),
            (-42, "%d", "-42"),
            (42, "%:+d", "+42"),
            (-42, "%:+d", "-42"),
            (42, "% d", " 42"),
            (-42, "% d", "-42"),
            (42, "%.5d", "00042"),
            (-42, "%.5d", "-00042"),
            (42, "%:+.5d", "+00042"),
            (-42, "%:+.5d", "-00042"),
            (42, "% .5d", " 00042"),
            (-42, "% .5d", "-00042"),
        ];
        let mut expand_context = ExpandContext::new();
        for (param1, format, expected) in tests {
            let cap = format!("%p1{format}");
            assert_str(
                expand_context.expand(cap.as_bytes(), &[Parameter::from(param1)]),
                expected,
            );
        }
    }

    #[test]
    fn format_octal() {
        let tests = [
            (42, "%o", "52"),
            (42, "%#o", "052"),
            (42, "%.5o", "00052"),
            (42, "%#.5o", "00052"),
        ];
        let mut expand_context = ExpandContext::new();
        for (param1, format, expected) in tests {
            let cap = format!("%p1{format}");
            assert_str(
                expand_context.expand(cap.as_bytes(), &[Parameter::from(param1)]),
                expected,
            );
        }
    }

    #[test]
    fn format_hexadecimal() {
        let tests = [
            (42, "%x", "2a"),
            (42, "%#x", "0x2a"),
            (0, "%#x", "0"),
            (42, "%.5x", "0002a"),
            (42, "%#.5x", "0x0002a"),
            (0, "%#.5x", "00000"),
            (42, "%X", "2A"),
            (42, "%#X", "0X2A"),
            (0, "%#X", "0"),
            (42, "%.5X", "0002A"),
            (42, "%#.5X", "0X0002A"),
            (0, "%#.5X", "00000"),
        ];
        let mut expand_context = ExpandContext::new();
        for (param1, format, expected) in tests {
            let cap = format!("%p1{format}");
            assert_str(
                expand_context.expand(cap.as_bytes(), &[Parameter::from(param1)]),
                expected,
            );
        }
    }

    #[test]
    fn format_string() {
        let tests = [
            ("One", "%s", "One"),
            ("One", "%5s", "  One"),
            ("One", "%5.2s", "   On"),
            ("One", "%:-5.4s", "One  "),
        ];
        let mut expand_context = ExpandContext::new();
        for (param1, format, expected) in tests {
            let cap = format!("%p1{format}");
            assert_str(
                expand_context.expand(cap.as_bytes(), &[Parameter::from(param1)]),
                expected,
            );
        }
    }

    #[test]
    fn format_width_overflow() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%{1}%65536d", &[]),
            Err(Error::FormatWidthOverflow)
        );
    }

    #[test]
    fn format_precision_overflow() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%{1}%.65536d", &[]),
            Err(Error::FormatPrecisionOverflow)
        );
    }

    #[test]
    fn format_type_mismatch() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%p1%s", &[Parameter::from(63)]),
            Err(Error::FormatTypeMismatch)
        );
        assert_eq!(
            expand_context.expand(b"%p1%3d", &[Parameter::from("one")]),
            Err(Error::FormatTypeMismatch)
        );
    }

    #[test]
    fn unrecornized_format_option() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%Y", &[]),
            Err(Error::UnrecognizedFormatOption('Y'))
        );
    }

    #[test]
    fn bad_parameter_index() {
        let mut expand_context = ExpandContext::new();
        assert_eq!(
            expand_context.expand(b"%p0", &[]),
            Err(Error::InvalidParameterIndex('0'))
        );
    }
}
