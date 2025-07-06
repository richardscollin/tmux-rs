use core::mem::MaybeUninit;

// https://www.rfc-editor.org/rfc/rfc4648

pub unsafe fn b64_ntop(src: *const u8, srclength: usize, target: *mut u8, targsize: usize) -> i32 {
    let src = unsafe { std::slice::from_raw_parts(src, srclength) };
    let dst = unsafe { std::slice::from_raw_parts_mut(target.cast::<MaybeUninit<u8>>(), targsize) };

    match ntop(src, dst) {
        Ok(out) => (out.len() - 1) as i32,
        Err(_) => -1,
    }
}

/// skips all whitespace anywhere.
/// converts characters, four at a time, starting at (or after)
/// src from base - 64 numbers into three 8 bit bytes in the target area.
/// it returns the number of data bytes stored at the target, or -1 on error.
pub unsafe fn b64_pton(src: *const u8, target: *mut u8, targsize: usize) -> i32 {
    let srclength: usize = unsafe { crate::libc::strlen(src) };
    let src = unsafe { std::slice::from_raw_parts(src.cast::<u8>(), srclength) };
    let dst = unsafe { std::slice::from_raw_parts_mut(target.cast::<MaybeUninit<u8>>(), targsize) };

    match pton(src, dst) {
        Ok(out) => out.len() as i32,
        Err(_) => -1,
    }
}

/// minimum ascii value used in encoded format
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const REVERSE: [u8; u8::MAX as usize] = const {
    let mut tmp = [u8::MAX; u8::MAX as usize];

    let mut i: u8 = 0;
    while i < ALPHABET.len() as u8 {
        tmp[ALPHABET[i as usize] as usize] = i;
        i += 1;
    }

    tmp
};

/// decode
fn pton<'out>(src: &'_ [u8], dst: &'out mut [MaybeUninit<u8>]) -> Result<&'out mut [u8], ()> {
    // dst must be at least 3/4 of src, and room for NUL byte
    if src.len().div_ceil(4) * 3 + 1 > dst.len() {
        return Err(());
    }

    let mut i = 0;
    let mut it = src.iter().cloned().filter(|b| !b.is_ascii_whitespace());

    while let Some(ch) = it.next() {
        let chunk: [u8; 4] = [
            ch,
            it.next().ok_or(())?,
            it.next().ok_or(())?,
            it.next().ok_or(())?, // TODO consider special handling for missing =
        ];

        for g in chunk {
            if !matches!(g, b'A'..=b'Z' | b'a'..=b'z' | b'+' | b'/') {
                return Err(());
            }
        }

        let a = REVERSE[chunk[0] as usize];
        let b = REVERSE[chunk[1] as usize];
        let c = REVERSE[chunk[2] as usize];
        let d = REVERSE[chunk[3] as usize];

        //        a                 b                 c                 d
        // X X 0 0 0 0 0 0 | X X 0 0 0 0 0 0 | X X 0 0 0 0 0 0 | X X 0 0 0 0 0 0
        //
        // ( a << 2  ) ( b >> 4 )    (b<<4) (    c >> 2    )     (c<<4)(      d       )
        // 0  0  0  0  0  0  0  0  |  0  0  0  0  0  0  0  0  |  0  0  0  0  0  0  0  0
        //
        dst[i] = MaybeUninit::new(a << 2 | b >> 4);
        dst[i + 1] = MaybeUninit::new(b << 4 | c >> 2);
        dst[i + 2] = MaybeUninit::new(c << 6 | d);
        i += 3;
    }

    dst[i] = MaybeUninit::new(0);
    Ok(unsafe { std::slice::from_raw_parts_mut(dst.as_mut_ptr().cast::<u8>(), i) })
}

/// encode
fn ntop<'out>(src: &'_ [u8], dst: &'out mut [MaybeUninit<u8>]) -> Result<&'out mut [u8], ()> {
    if dst.len() < src.len().div_ceil(3) * 4 + 1 {
        return Err(());
    }

    let mut i = 0;
    let mut it = src.chunks_exact(3);

    macro_rules! enc {
        ($e:expr) => {
            MaybeUninit::new(ALPHABET[($e & 0b00111111) as usize])
        }
    }

    for chunk in &mut it {
        dst[i] = enc!(chunk[0] >> 2);
        dst[i + 1] = enc!(chunk[0] << 4 | chunk[1] >> 4);
        dst[i + 2] = enc!(chunk[1] << 2 | chunk[2] >> 6);
        dst[i + 3] = enc!(chunk[2]);
        i += 4;
    }

    let chunk = it.remainder();
    match chunk.len() {
        0 => (),
        1 => {
            dst[i] = enc!(chunk[0] >> 2);
            dst[i + 1] = enc!(chunk[0] << 4);
            dst[i + 2] = MaybeUninit::new(b'=');
            dst[i + 3] = MaybeUninit::new(b'=');
            i += 4;
        }
        2 => {
            dst[i] = enc!(chunk[0] >> 2);
            dst[i + 1] = enc!(chunk[0] << 4 | chunk[1] >> 4);
            dst[i + 2] = enc!(chunk[1] << 2);
            dst[i + 3] = MaybeUninit::new(b'=');
            i += 4;
        }
        _ => unreachable!(),
    }

    dst[i] = MaybeUninit::new(b'\0');
    Ok(unsafe { std::slice::from_raw_parts_mut(dst.as_mut_ptr().cast::<u8>(), i) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_b64_pton_valid() {
        let input = crate::c!("TWFu");
        let mut output = [0u8; 4];
        let expected = [b'M', b'a', b'n', 0];

        unsafe {
            let result = b64_pton(input, output.as_mut_ptr(), output.len());
            assert_eq!(&output, &expected);
            assert_eq!(result, 3);
        }
    }

    #[test]
    fn test_b64_pton_invalid() {
        let input = crate::c!("****");
        let mut output = [0u8; 3];

        unsafe {
            let result = b64_pton(input, output.as_mut_ptr(), output.len());
            assert_eq!(result, -1);
        }
    }

    #[test]
    fn test_b64_pton_partial() {
        let input = crate::c!("TWE=");
        let mut output = [0u8; 2];
        // TODO currently not supporting missing =, but we could

        unsafe {
            let result = b64_pton(input, output.as_mut_ptr(), output.len());
            assert_eq!(result, -1);
        }
    }
}
