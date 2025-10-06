pub type bitstr_t = u8;

pub unsafe fn bit_alloc(nbits: u32) -> *mut u8 {
    unsafe { libc::calloc(nbits.div_ceil(8) as usize, 1).cast() }
}

pub unsafe fn bit_set(bits: *mut u8, i: u32) {
    unsafe {
        let byte_index = i / 8;
        let bit_index = i % 8;
        *bits.add(byte_index as usize) |= 1 << bit_index;
    }
}

#[inline]
pub unsafe fn bit_clear(bits: *mut u8, i: u32) {
    unsafe {
        let byte_index = i / 8;
        let bit_index = i % 8;
        *bits.add(byte_index as usize) &= !(1 << bit_index);
    }
}

/// clear bits start..=stop in bitstring
pub unsafe fn bit_nclear(bits: *mut u8, start: u32, stop: u32) {
    unsafe {
        // TODO this is written inefficiently, assuming the compiler will optimize it. if it doesn't rewrite it
        for i in start..=stop {
            bit_clear(bits, i);
        }
    }
}

pub unsafe fn bit_test(bits: *const u8, i: u32) -> bool {
    unsafe {
        let byte_index = i / 8;
        let bit_index = i % 8;
        (*bits.add(byte_index as usize) & (1 << bit_index)) != 0
    }
}

