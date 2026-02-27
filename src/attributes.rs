// Copyright (c) 2009 Joshua Elsasser <josh@elsasser.org>
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
use std::borrow::Cow;

use crate::grid_attr;

#[rustfmt::skip]
pub fn attributes_tostring(attr: grid_attr) -> Cow<'static, str> {
    if attr.is_empty() {
        return Cow::Borrowed("none");
    }

    Cow::Owned(format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
        if attr.intersects(grid_attr::GRID_ATTR_CHARSET) { "acs," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_BRIGHT) { "bright," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_DIM ) { "dim," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE) { "underscore," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_BLINK) { "blink," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_REVERSE ) { "reverse," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_HIDDEN) { "hidden," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_ITALICS ) { "italics," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_STRIKETHROUGH) { "strikethrough," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_2) { "double-underscore," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_3) { "curly-underscore," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_4) { "dotted-underscore," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_5) { "dashed-underscore," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_OVERLINE) { "overline," } else { "" },
    ))
}

/// Parse a comma/space/pipe-separated attribute string into grid_attr flags.
pub fn attributes_fromstring(str: &str) -> Result<grid_attr, ()> {
    struct table_entry {
        name: &'static str,
        attr: grid_attr,
    }

    #[rustfmt::skip]
    const TABLE: [table_entry; 15] = [
        table_entry { name: "acs", attr: grid_attr::GRID_ATTR_CHARSET, },
        table_entry { name: "bright", attr: grid_attr::GRID_ATTR_BRIGHT, },
        table_entry { name: "bold", attr: grid_attr::GRID_ATTR_BRIGHT, },
        table_entry { name: "dim", attr: grid_attr::GRID_ATTR_DIM, },
        table_entry { name: "underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE, },
        table_entry { name: "blink", attr: grid_attr::GRID_ATTR_BLINK, },
        table_entry { name: "reverse", attr: grid_attr::GRID_ATTR_REVERSE, },
        table_entry { name: "hidden", attr: grid_attr::GRID_ATTR_HIDDEN, },
        table_entry { name: "italics", attr: grid_attr::GRID_ATTR_ITALICS, },
        table_entry { name: "strikethrough", attr: grid_attr::GRID_ATTR_STRIKETHROUGH, },
        table_entry { name: "double-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_2, },
        table_entry { name: "curly-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_3, },
        table_entry { name: "dotted-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_4, },
        table_entry { name: "dashed-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_5, },
        table_entry { name: "overline", attr: grid_attr::GRID_ATTR_OVERLINE, },
    ];

    let delimiters = &[' ', ',', '|'];

    if str.is_empty() || str.find(delimiters) == Some(0) {
        return Err(());
    }

    if matches!(str.chars().next_back().unwrap(), ' ' | ',' | '|') {
        return Err(());
    }

    if str.eq_ignore_ascii_case("default") || str.eq_ignore_ascii_case("none") {
        return Ok(grid_attr::empty());
    }

    let mut attr = grid_attr::empty();
    for str in str.split(delimiters) {
        let Some(i) = TABLE.iter().position(|t| str.eq_ignore_ascii_case(t.name)) else {
            return Err(());
        };
        attr |= TABLE[i].attr;
    }

    Ok(attr)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg_attr(not(feature = "coverage-tests"), ignore)]
    fn test_tostring() {
        assert_eq!(attributes_tostring(grid_attr::empty()).as_ref(), "none");

        // Single flag: exercises true for one, false for all others
        assert!(attributes_tostring(grid_attr::GRID_ATTR_CHARSET).contains("acs,"));

        // All flags at once - each name must appear in the output
        let all = grid_attr::GRID_ATTR_CHARSET
            | grid_attr::GRID_ATTR_BRIGHT
            | grid_attr::GRID_ATTR_DIM
            | grid_attr::GRID_ATTR_UNDERSCORE
            | grid_attr::GRID_ATTR_BLINK
            | grid_attr::GRID_ATTR_REVERSE
            | grid_attr::GRID_ATTR_HIDDEN
            | grid_attr::GRID_ATTR_ITALICS
            | grid_attr::GRID_ATTR_STRIKETHROUGH
            | grid_attr::GRID_ATTR_UNDERSCORE_2
            | grid_attr::GRID_ATTR_UNDERSCORE_3
            | grid_attr::GRID_ATTR_UNDERSCORE_4
            | grid_attr::GRID_ATTR_UNDERSCORE_5
            | grid_attr::GRID_ATTR_OVERLINE;
        let s = attributes_tostring(all);
        for name in [
            "acs,", "bright,", "dim,", "underscore,", "blink,", "reverse,",
            "hidden,", "italics,", "strikethrough,", "double-underscore,",
            "curly-underscore,", "dotted-underscore,", "dashed-underscore,", "overline,",
        ] {
            assert!(s.contains(name), "{name} not in {s}");
        }
    }

    fn assert_parses_to(input: &str, expected: grid_attr) {
        let result = attributes_fromstring(input);
        assert!(result.is_ok(), "expected Ok for {input:?}, got Err");
        assert!(result.unwrap() == expected, "wrong flags for {input:?}");
    }

    #[test]
    #[cfg_attr(not(feature = "coverage-tests"), ignore)]
    fn test_fromstring() {
        // All table entries via combined parse
        let cases: &[(&str, grid_attr)] = &[
            ("acs", grid_attr::GRID_ATTR_CHARSET),
            ("bright", grid_attr::GRID_ATTR_BRIGHT),
            ("bold", grid_attr::GRID_ATTR_BRIGHT),
            ("dim", grid_attr::GRID_ATTR_DIM),
            ("underscore", grid_attr::GRID_ATTR_UNDERSCORE),
            ("blink", grid_attr::GRID_ATTR_BLINK),
            ("reverse", grid_attr::GRID_ATTR_REVERSE),
            ("hidden", grid_attr::GRID_ATTR_HIDDEN),
            ("italics", grid_attr::GRID_ATTR_ITALICS),
            ("strikethrough", grid_attr::GRID_ATTR_STRIKETHROUGH),
            ("double-underscore", grid_attr::GRID_ATTR_UNDERSCORE_2),
            ("curly-underscore", grid_attr::GRID_ATTR_UNDERSCORE_3),
            ("dotted-underscore", grid_attr::GRID_ATTR_UNDERSCORE_4),
            ("dashed-underscore", grid_attr::GRID_ATTR_UNDERSCORE_5),
            ("overline", grid_attr::GRID_ATTR_OVERLINE),
        ];
        for &(name, attr) in cases {
            assert_parses_to(name, attr);
        }

        // Special names: none/default (case insensitive)
        for name in ["none", "default", "NONE", "Default"] {
            assert_parses_to(name, grid_attr::empty());
        }

        // Combined with all delimiter types
        let expected = grid_attr::GRID_ATTR_BRIGHT | grid_attr::GRID_ATTR_ITALICS;
        for input in ["bright,italics", "bright italics", "bright|italics"] {
            assert_parses_to(input, expected);
        }

        // Error cases: empty, leading delimiter, trailing delimiter, unknown
        for bad in ["", ",bright", " bright", "|bright", "bright,", "bright ", "bright|", "invalid"] {
            assert!(attributes_fromstring(bad).is_err(), "expected Err for {bad:?}");
        }
    }

    #[test]
    #[cfg_attr(not(feature = "coverage-tests"), ignore)]
    fn test_roundtrip() {
        let all = grid_attr::GRID_ATTR_BRIGHT
            | grid_attr::GRID_ATTR_DIM
            | grid_attr::GRID_ATTR_UNDERSCORE
            | grid_attr::GRID_ATTR_BLINK
            | grid_attr::GRID_ATTR_REVERSE
            | grid_attr::GRID_ATTR_HIDDEN
            | grid_attr::GRID_ATTR_ITALICS
            | grid_attr::GRID_ATTR_OVERLINE;
        // tostring produces "bright,dim,...overline," - trim trailing comma, split, re-parse
        let s = attributes_tostring(all);
        let trimmed = s.trim_end_matches(',');
        let parsed = attributes_fromstring(trimmed);
        assert!(parsed.is_ok(), "roundtrip parse failed for {s}");
        assert!(parsed.unwrap() == all, "roundtrip mismatch for {s}");
    }
}
