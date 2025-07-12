use crate::*;

pub(crate) struct GetOptIter<'a> {
    ostr: &'static str,

    args: std::iter::Peekable<std::slice::Iter<'a, &'a str>>,
    arg_group: Option<std::iter::Peekable<std::str::Chars<'a>>>,
}

impl<'a> GetOptIter<'a> {
    pub fn new(args: &'a [&'a str], ostr: &'static str) -> Self {
        GetOptIter {
            args: args.iter().peekable(),
            ostr,
            arg_group: None,
        }
    }

    pub fn remaining_args(self) -> impl Iterator<Item = &'a str> {
        assert!(self.arg_group.is_none());
        self.args.copied()
    }
}

impl<'a> Iterator for GetOptIter<'a> {
    type Item = Result<(char, Option<&'a str>), &'static str>;

    fn next(&mut self) -> Option<Self::Item> {
        if self
            .arg_group
            .as_mut()
            .is_none_or(|group| group.peek().is_none())
        {
            let arg = self.args.next()?;

            let stripped = arg.strip_prefix("-")?;

            if stripped.is_empty() {
                return Some(Err("- with no args"));
            }

            self.arg_group = Some(stripped.chars().peekable());
        }

        let ch = self.arg_group.as_mut().unwrap().next()?;

        let Some(ch_pos) = self.ostr.find(ch) else {
            return Some(Err("unexpected extra argument"));
        };

        if self.ostr.as_bytes().get(ch_pos + 1).copied() == Some(b':') {
            assert!(self.arg_group.as_mut().unwrap().peek().is_none()); // TODO return err
            TODO need to instead if there's any remaining use that as the argument
            something like self.arg_group.as_mut().unwrap().as_slice()
            but can't use because we've wrapped it in a peek

            if let Some(next_arg) = self.args.next()
                && !next_arg.starts_with('-')
            {
                return Some(Ok((ch, Some(next_arg))));
            } else {
                eprintln!("tmux-rs: option requires an argument -- {ch}");
                return Some(Err("missing required arg"));
            }
        }

        Some(Ok((ch, None)))
    }
}

// note we don't need to support optional arguments
pub(crate) fn getopt_rs<'a>(args: &'a [&'a str], ostr: &'static str) -> GetOptIter<'a> {
    GetOptIter::new(args, ostr)
}
