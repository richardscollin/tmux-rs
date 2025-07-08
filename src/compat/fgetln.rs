// Copyright (c) 2015 Joerg Jung <jung@openbsd.org>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
use std::io::{BufRead, BufReader, Result};

pub fn fgetln_safe<R: BufRead>(reader: &mut R) -> Result<Option<Vec<u8>>> {
    let mut buffer = Vec::new();

    match reader.read_until(b'\n', &mut buffer) {
        Ok(0) => Ok(None), // EOF
        Ok(_) => Ok(Some(buffer)),
        Err(e) => Err(e),
    }
}
