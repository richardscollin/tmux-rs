#[repr(transparent)]
pub struct BitStr {
    bits: Box<[u8]>,
}

impl BitStr {
    pub fn new(nbits: u32) -> Self {
        Self {
            bits: vec![0; nbits.div_ceil(8) as usize].into_boxed_slice()
        }
    }

    pub fn bit_set(&mut self, i: u32) {
        let byte_index = i / 8;
        let bit_index = i % 8;
        self.bits[byte_index as usize] |= 1 << bit_index;
    }

    #[inline]
    pub fn bit_clear(&mut self, i: u32) {
        let byte_index = i / 8;
        let bit_index = i % 8;
        self.bits[byte_index as usize] &= !(1 << bit_index); 
    }

    pub fn bit_nclear(&mut self, start: u32, stop: u32) {
        // TODO this is written inefficiently, assuming the compiler will optimize it. if it doesn't rewrite it
        for i in start..=stop {
            self.bit_clear(i);
        }
    }

    pub fn bit_test(&self, i: u32) -> bool {
        let byte_index = i / 8;
        let bit_index = i % 8;
        self.bits[byte_index as usize] & (1 << bit_index) != 0
    }
}
