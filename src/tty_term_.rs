// Copyright (c) 2008 Nicholas Marriott <niu8ail.com>
//
// Permission to use, copy, modify, and distriu8his software for any
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
use crate::compat::{strnvis, strunvis};
use crate::libc::{fnmatch, memset, strchr, strcmp, strcspn, strncmp};
use crate::*;
use crate::options_::*;

pub static mut TTY_TERMS: tty_terms = list_head_initializer();

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum tty_code_type {
    None = 0,
    String,
    Number,
    Flag,
}

#[repr(C)]
pub union tty_code_union {
    string: *mut u8,
    number: i32,
    flag: i32,
}

#[repr(C)]
pub struct tty_code {
    pub type_: tty_code_type,
    pub value: tty_code_union,
}

#[repr(C)]
pub struct tty_term_code_entry {
    pub type_: tty_code_type,
    pub name: SyncCharPtr,
}

impl tty_term_code_entry {
    const fn new(type_: tty_code_type, name: &'static CStr) -> Self {
        Self {
            type_,
            name: SyncCharPtr::new(name),
        }
    }
}

#[rustfmt::skip]
static TTY_TERM_CODES: [tty_term_code_entry; 232] = const {
    let mut tmp: [tty_term_code_entry; 232] = unsafe { zeroed() };

    tmp[tty_code_code::TTYC_ACSC as usize] = tty_term_code_entry::new(tty_code_type::String, c"acsc");
    tmp[tty_code_code::TTYC_AM as usize] = tty_term_code_entry::new(tty_code_type::Flag, c"am");
    tmp[tty_code_code::TTYC_AX as usize] = tty_term_code_entry::new(tty_code_type::Flag, c"AX");
    tmp[tty_code_code::TTYC_BCE as usize] = tty_term_code_entry::new(tty_code_type::Flag, c"bce");
    tmp[tty_code_code::TTYC_BEL as usize] = tty_term_code_entry::new(tty_code_type::String, c"bel");
    tmp[tty_code_code::TTYC_BIDI as usize] = tty_term_code_entry::new(tty_code_type::String, c"Bidi");
    tmp[tty_code_code::TTYC_BLINK as usize] = tty_term_code_entry::new(tty_code_type::String, c"blink");
    tmp[tty_code_code::TTYC_BOLD as usize] = tty_term_code_entry::new(tty_code_type::String, c"bold");
    tmp[tty_code_code::TTYC_CIVIS as usize] = tty_term_code_entry::new(tty_code_type::String, c"civis");
    tmp[tty_code_code::TTYC_CLEAR as usize] = tty_term_code_entry::new(tty_code_type::String, c"clear");
    tmp[tty_code_code::TTYC_CLMG as usize] = tty_term_code_entry::new(tty_code_type::String, c"Clmg");
    tmp[tty_code_code::TTYC_CMG as usize] = tty_term_code_entry::new(tty_code_type::String, c"Cmg");
    tmp[tty_code_code::TTYC_CNORM as usize] = tty_term_code_entry::new(tty_code_type::String, c"cnorm");
    tmp[tty_code_code::TTYC_COLORS as usize] = tty_term_code_entry::new(tty_code_type::Number, c"colors");
    tmp[tty_code_code::TTYC_CR as usize] = tty_term_code_entry::new(tty_code_type::String, c"Cr");
    tmp[tty_code_code::TTYC_CSR as usize] = tty_term_code_entry::new(tty_code_type::String, c"csr");
    tmp[tty_code_code::TTYC_CS as usize] = tty_term_code_entry::new(tty_code_type::String, c"Cs");
    tmp[tty_code_code::TTYC_CUB1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"cub1");
    tmp[tty_code_code::TTYC_CUB as usize] = tty_term_code_entry::new(tty_code_type::String, c"cub");
    tmp[tty_code_code::TTYC_CUD1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"cud1");
    tmp[tty_code_code::TTYC_CUD as usize] = tty_term_code_entry::new(tty_code_type::String, c"cud");
    tmp[tty_code_code::TTYC_CUF1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"cuf1");
    tmp[tty_code_code::TTYC_CUF as usize] = tty_term_code_entry::new(tty_code_type::String, c"cuf");
    tmp[tty_code_code::TTYC_CUP as usize] = tty_term_code_entry::new(tty_code_type::String, c"cup");
    tmp[tty_code_code::TTYC_CUU1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"cuu1");
    tmp[tty_code_code::TTYC_CUU as usize] = tty_term_code_entry::new(tty_code_type::String, c"cuu");
    tmp[tty_code_code::TTYC_CVVIS as usize] = tty_term_code_entry::new(tty_code_type::String, c"cvvis");
    tmp[tty_code_code::TTYC_DCH1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"dch1");
    tmp[tty_code_code::TTYC_DCH as usize] = tty_term_code_entry::new(tty_code_type::String, c"dch");
    tmp[tty_code_code::TTYC_DIM as usize] = tty_term_code_entry::new(tty_code_type::String, c"dim");
    tmp[tty_code_code::TTYC_DL1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"dl1");
    tmp[tty_code_code::TTYC_DL as usize] = tty_term_code_entry::new(tty_code_type::String, c"dl");
    tmp[tty_code_code::TTYC_DSEKS as usize] = tty_term_code_entry::new(tty_code_type::String, c"Dseks");
    tmp[tty_code_code::TTYC_DSFCS as usize] = tty_term_code_entry::new(tty_code_type::String, c"Dsfcs");
    tmp[tty_code_code::TTYC_DSBP as usize] = tty_term_code_entry::new(tty_code_type::String, c"Dsbp");
    tmp[tty_code_code::TTYC_DSMG as usize] = tty_term_code_entry::new(tty_code_type::String, c"Dsmg");
    tmp[tty_code_code::TTYC_E3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"E3");
    tmp[tty_code_code::TTYC_ECH as usize] = tty_term_code_entry::new(tty_code_type::String, c"ech");
    tmp[tty_code_code::TTYC_ED as usize] = tty_term_code_entry::new(tty_code_type::String, c"ed");
    tmp[tty_code_code::TTYC_EL1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"el1");
    tmp[tty_code_code::TTYC_EL as usize] = tty_term_code_entry::new(tty_code_type::String, c"el");
    tmp[tty_code_code::TTYC_ENACS as usize] = tty_term_code_entry::new(tty_code_type::String, c"enacs");
    tmp[tty_code_code::TTYC_ENBP as usize] = tty_term_code_entry::new(tty_code_type::String, c"Enbp");
    tmp[tty_code_code::TTYC_ENEKS as usize] = tty_term_code_entry::new(tty_code_type::String, c"Eneks");
    tmp[tty_code_code::TTYC_ENFCS as usize] = tty_term_code_entry::new(tty_code_type::String, c"Enfcs");
    tmp[tty_code_code::TTYC_ENMG as usize] = tty_term_code_entry::new(tty_code_type::String, c"Enmg");
    tmp[tty_code_code::TTYC_FSL as usize] = tty_term_code_entry::new(tty_code_type::String, c"fsl");
    tmp[tty_code_code::TTYC_HLS as usize] = tty_term_code_entry::new(tty_code_type::String, c"Hls");
    tmp[tty_code_code::TTYC_HOME as usize] = tty_term_code_entry::new(tty_code_type::String, c"home");
    tmp[tty_code_code::TTYC_HPA as usize] = tty_term_code_entry::new(tty_code_type::String, c"hpa");
    tmp[tty_code_code::TTYC_ICH1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"ich1");
    tmp[tty_code_code::TTYC_ICH as usize] = tty_term_code_entry::new(tty_code_type::String, c"ich");
    tmp[tty_code_code::TTYC_IL1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"il1");
    tmp[tty_code_code::TTYC_IL as usize] = tty_term_code_entry::new(tty_code_type::String, c"il");
    tmp[tty_code_code::TTYC_INDN as usize] = tty_term_code_entry::new(tty_code_type::String, c"indn");
    tmp[tty_code_code::TTYC_INVIS as usize] = tty_term_code_entry::new(tty_code_type::String, c"invis");
    tmp[tty_code_code::TTYC_KCBT as usize] = tty_term_code_entry::new(tty_code_type::String, c"kcbt");
    tmp[tty_code_code::TTYC_KCUB1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kcub1");
    tmp[tty_code_code::TTYC_KCUD1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kcud1");
    tmp[tty_code_code::TTYC_KCUF1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kcuf1");
    tmp[tty_code_code::TTYC_KCUU1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kcuu1");
    tmp[tty_code_code::TTYC_KDC2 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDC");
    tmp[tty_code_code::TTYC_KDC3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDC3");
    tmp[tty_code_code::TTYC_KDC4 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDC4");
    tmp[tty_code_code::TTYC_KDC5 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDC5");
    tmp[tty_code_code::TTYC_KDC6 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDC6");
    tmp[tty_code_code::TTYC_KDC7 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDC7");
    tmp[tty_code_code::TTYC_KDCH1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kdch1");
    tmp[tty_code_code::TTYC_KDN2 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDN"); // not kDN2
    tmp[tty_code_code::TTYC_KDN3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDN3");
    tmp[tty_code_code::TTYC_KDN4 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDN4");
    tmp[tty_code_code::TTYC_KDN5 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDN5");
    tmp[tty_code_code::TTYC_KDN6 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDN6");
    tmp[tty_code_code::TTYC_KDN7 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kDN7");
    tmp[tty_code_code::TTYC_KEND2 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kEND");
    tmp[tty_code_code::TTYC_KEND3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kEND3");
    tmp[tty_code_code::TTYC_KEND4 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kEND4");
    tmp[tty_code_code::TTYC_KEND5 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kEND5");
    tmp[tty_code_code::TTYC_KEND6 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kEND6");
    tmp[tty_code_code::TTYC_KEND7 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kEND7");
    tmp[tty_code_code::TTYC_KEND as usize] = tty_term_code_entry::new(tty_code_type::String, c"kend");
    tmp[tty_code_code::TTYC_KF10 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf10");
    tmp[tty_code_code::TTYC_KF11 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf11");
    tmp[tty_code_code::TTYC_KF12 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf12");
    tmp[tty_code_code::TTYC_KF13 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf13");
    tmp[tty_code_code::TTYC_KF14 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf14");
    tmp[tty_code_code::TTYC_KF15 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf15");
    tmp[tty_code_code::TTYC_KF16 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf16");
    tmp[tty_code_code::TTYC_KF17 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf17");
    tmp[tty_code_code::TTYC_KF18 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf18");
    tmp[tty_code_code::TTYC_KF19 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf19");
    tmp[tty_code_code::TTYC_KF1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf1");
    tmp[tty_code_code::TTYC_KF20 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf20");
    tmp[tty_code_code::TTYC_KF21 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf21");
    tmp[tty_code_code::TTYC_KF22 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf22");
    tmp[tty_code_code::TTYC_KF23 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf23");
    tmp[tty_code_code::TTYC_KF24 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf24");
    tmp[tty_code_code::TTYC_KF25 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf25");
    tmp[tty_code_code::TTYC_KF26 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf26");
    tmp[tty_code_code::TTYC_KF27 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf27");
    tmp[tty_code_code::TTYC_KF28 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf28");
    tmp[tty_code_code::TTYC_KF29 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf29");
    tmp[tty_code_code::TTYC_KF2 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf2");
    tmp[tty_code_code::TTYC_KF30 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf30");
    tmp[tty_code_code::TTYC_KF31 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf31");
    tmp[tty_code_code::TTYC_KF32 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf32");
    tmp[tty_code_code::TTYC_KF33 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf33");
    tmp[tty_code_code::TTYC_KF34 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf34");
    tmp[tty_code_code::TTYC_KF35 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf35");
    tmp[tty_code_code::TTYC_KF36 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf36");
    tmp[tty_code_code::TTYC_KF37 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf37");
    tmp[tty_code_code::TTYC_KF38 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf38");
    tmp[tty_code_code::TTYC_KF39 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf39");
    tmp[tty_code_code::TTYC_KF3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf3");
    tmp[tty_code_code::TTYC_KF40 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf40");
    tmp[tty_code_code::TTYC_KF41 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf41");
    tmp[tty_code_code::TTYC_KF42 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf42");
    tmp[tty_code_code::TTYC_KF43 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf43");
    tmp[tty_code_code::TTYC_KF44 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf44");
    tmp[tty_code_code::TTYC_KF45 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf45");
    tmp[tty_code_code::TTYC_KF46 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf46");
    tmp[tty_code_code::TTYC_KF47 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf47");
    tmp[tty_code_code::TTYC_KF48 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf48");
    tmp[tty_code_code::TTYC_KF49 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf49");
    tmp[tty_code_code::TTYC_KF4 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf4");
    tmp[tty_code_code::TTYC_KF50 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf50");
    tmp[tty_code_code::TTYC_KF51 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf51");
    tmp[tty_code_code::TTYC_KF52 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf52");
    tmp[tty_code_code::TTYC_KF53 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf53");
    tmp[tty_code_code::TTYC_KF54 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf54");
    tmp[tty_code_code::TTYC_KF55 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf55");
    tmp[tty_code_code::TTYC_KF56 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf56");
    tmp[tty_code_code::TTYC_KF57 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf57");
    tmp[tty_code_code::TTYC_KF58 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf58");
    tmp[tty_code_code::TTYC_KF59 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf59");
    tmp[tty_code_code::TTYC_KF5 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf5");
    tmp[tty_code_code::TTYC_KF60 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf60");
    tmp[tty_code_code::TTYC_KF61 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf61");
    tmp[tty_code_code::TTYC_KF62 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf62");
    tmp[tty_code_code::TTYC_KF63 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf63");
    tmp[tty_code_code::TTYC_KF6 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf6");
    tmp[tty_code_code::TTYC_KF7 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf7");
    tmp[tty_code_code::TTYC_KF8 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf8");
    tmp[tty_code_code::TTYC_KF9 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kf9");
    tmp[tty_code_code::TTYC_KHOM2 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kHOM");
    tmp[tty_code_code::TTYC_KHOM3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kHOM3");
    tmp[tty_code_code::TTYC_KHOM4 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kHOM4");
    tmp[tty_code_code::TTYC_KHOM5 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kHOM5");
    tmp[tty_code_code::TTYC_KHOM6 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kHOM6");
    tmp[tty_code_code::TTYC_KHOM7 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kHOM7");
    tmp[tty_code_code::TTYC_KHOME as usize] = tty_term_code_entry::new(tty_code_type::String, c"khome");
    tmp[tty_code_code::TTYC_KIC2 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kIC");
    tmp[tty_code_code::TTYC_KIC3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kIC3");
    tmp[tty_code_code::TTYC_KIC4 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kIC4");
    tmp[tty_code_code::TTYC_KIC5 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kIC5");
    tmp[tty_code_code::TTYC_KIC6 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kIC6");
    tmp[tty_code_code::TTYC_KIC7 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kIC7");
    tmp[tty_code_code::TTYC_KICH1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kich1");
    tmp[tty_code_code::TTYC_KIND as usize] = tty_term_code_entry::new(tty_code_type::String, c"kind");
    tmp[tty_code_code::TTYC_KLFT2 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kLFT");
    tmp[tty_code_code::TTYC_KLFT3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kLFT3");
    tmp[tty_code_code::TTYC_KLFT4 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kLFT4");
    tmp[tty_code_code::TTYC_KLFT5 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kLFT5");
    tmp[tty_code_code::TTYC_KLFT6 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kLFT6");
    tmp[tty_code_code::TTYC_KLFT7 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kLFT7");
    tmp[tty_code_code::TTYC_KMOUS as usize] = tty_term_code_entry::new(tty_code_type::String, c"kmous");
    tmp[tty_code_code::TTYC_KNP as usize] = tty_term_code_entry::new(tty_code_type::String, c"knp");
    tmp[tty_code_code::TTYC_KNXT2 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kNXT");
    tmp[tty_code_code::TTYC_KNXT3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kNXT3");
    tmp[tty_code_code::TTYC_KNXT4 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kNXT4");
    tmp[tty_code_code::TTYC_KNXT5 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kNXT5");
    tmp[tty_code_code::TTYC_KNXT6 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kNXT6");
    tmp[tty_code_code::TTYC_KNXT7 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kNXT7");
    tmp[tty_code_code::TTYC_KPP as usize] = tty_term_code_entry::new(tty_code_type::String, c"kpp");
    tmp[tty_code_code::TTYC_KPRV2 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kPRV");
    tmp[tty_code_code::TTYC_KPRV3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kPRV3");
    tmp[tty_code_code::TTYC_KPRV4 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kPRV4");
    tmp[tty_code_code::TTYC_KPRV5 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kPRV5");
    tmp[tty_code_code::TTYC_KPRV6 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kPRV6");
    tmp[tty_code_code::TTYC_KPRV7 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kPRV7");
    tmp[tty_code_code::TTYC_KRIT2 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kRIT");
    tmp[tty_code_code::TTYC_KRIT3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kRIT3");
    tmp[tty_code_code::TTYC_KRIT4 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kRIT4");
    tmp[tty_code_code::TTYC_KRIT5 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kRIT5");
    tmp[tty_code_code::TTYC_KRIT6 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kRIT6");
    tmp[tty_code_code::TTYC_KRIT7 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kRIT7");
    tmp[tty_code_code::TTYC_KRI as usize] = tty_term_code_entry::new(tty_code_type::String, c"kri");
    tmp[tty_code_code::TTYC_KUP2 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kUP"); // not kUP2
    tmp[tty_code_code::TTYC_KUP3 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kUP3");
    tmp[tty_code_code::TTYC_KUP4 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kUP4");
    tmp[tty_code_code::TTYC_KUP5 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kUP5");
    tmp[tty_code_code::TTYC_KUP6 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kUP6");
    tmp[tty_code_code::TTYC_KUP7 as usize] = tty_term_code_entry::new(tty_code_type::String, c"kUP7");
    tmp[tty_code_code::TTYC_MS as usize] = tty_term_code_entry::new(tty_code_type::String, c"Ms");
    tmp[tty_code_code::TTYC_NOBR as usize] = tty_term_code_entry::new(tty_code_type::String, c"Nobr");
    tmp[tty_code_code::TTYC_OL as usize] = tty_term_code_entry::new(tty_code_type::String, c"ol");
    tmp[tty_code_code::TTYC_OP as usize] = tty_term_code_entry::new(tty_code_type::String, c"op");
    tmp[tty_code_code::TTYC_RECT as usize] = tty_term_code_entry::new(tty_code_type::String, c"Rect");
    tmp[tty_code_code::TTYC_REV as usize] = tty_term_code_entry::new(tty_code_type::String, c"rev");
    tmp[tty_code_code::TTYC_RGB as usize] = tty_term_code_entry::new(tty_code_type::Flag, c"RGB");
    tmp[tty_code_code::TTYC_RIN as usize] = tty_term_code_entry::new(tty_code_type::String, c"rin");
    tmp[tty_code_code::TTYC_RI as usize] = tty_term_code_entry::new(tty_code_type::String, c"ri");
    tmp[tty_code_code::TTYC_RMACS as usize] = tty_term_code_entry::new(tty_code_type::String, c"rmacs");
    tmp[tty_code_code::TTYC_RMCUP as usize] = tty_term_code_entry::new(tty_code_type::String, c"rmcup");
    tmp[tty_code_code::TTYC_RMKX as usize] = tty_term_code_entry::new(tty_code_type::String, c"rmkx");
    tmp[tty_code_code::TTYC_SETAB as usize] = tty_term_code_entry::new(tty_code_type::String, c"setab");
    tmp[tty_code_code::TTYC_SETAF as usize] = tty_term_code_entry::new(tty_code_type::String, c"setaf");
    tmp[tty_code_code::TTYC_SETAL as usize] = tty_term_code_entry::new(tty_code_type::String, c"setal");
    tmp[tty_code_code::TTYC_SETRGBB as usize] = tty_term_code_entry::new(tty_code_type::String, c"setrgbb");
    tmp[tty_code_code::TTYC_SETRGBF as usize] = tty_term_code_entry::new(tty_code_type::String, c"setrgbf");
    tmp[tty_code_code::TTYC_SETULC as usize] = tty_term_code_entry::new(tty_code_type::String, c"Setulc");
    tmp[tty_code_code::TTYC_SETULC1 as usize] = tty_term_code_entry::new(tty_code_type::String, c"Setulc1");
    tmp[tty_code_code::TTYC_SE as usize] = tty_term_code_entry::new(tty_code_type::String, c"Se");
    tmp[tty_code_code::TTYC_SXL as usize] = tty_term_code_entry::new(tty_code_type::Flag, c"Sxl");
    tmp[tty_code_code::TTYC_SGR0 as usize] = tty_term_code_entry::new(tty_code_type::String, c"sgr0");
    tmp[tty_code_code::TTYC_SITM as usize] = tty_term_code_entry::new(tty_code_type::String, c"sitm");
    tmp[tty_code_code::TTYC_SMACS as usize] = tty_term_code_entry::new(tty_code_type::String, c"smacs");
    tmp[tty_code_code::TTYC_SMCUP as usize] = tty_term_code_entry::new(tty_code_type::String, c"smcup");
    tmp[tty_code_code::TTYC_SMKX as usize] = tty_term_code_entry::new(tty_code_type::String, c"smkx");
    tmp[tty_code_code::TTYC_SMOL as usize] = tty_term_code_entry::new(tty_code_type::String, c"Smol");
    tmp[tty_code_code::TTYC_SMSO as usize] = tty_term_code_entry::new(tty_code_type::String, c"smso");
    tmp[tty_code_code::TTYC_SMULX as usize] = tty_term_code_entry::new(tty_code_type::String, c"Smulx");
    tmp[tty_code_code::TTYC_SMUL as usize] = tty_term_code_entry::new(tty_code_type::String, c"smul");
    tmp[tty_code_code::TTYC_SMXX as usize] = tty_term_code_entry::new(tty_code_type::String, c"smxx");
    tmp[tty_code_code::TTYC_SS as usize] = tty_term_code_entry::new(tty_code_type::String, c"Ss");
    tmp[tty_code_code::TTYC_SWD as usize] = tty_term_code_entry::new(tty_code_type::String, c"Swd");
    tmp[tty_code_code::TTYC_SYNC as usize] = tty_term_code_entry::new(tty_code_type::String, c"Sync");
    tmp[tty_code_code::TTYC_TC as usize] = tty_term_code_entry::new(tty_code_type::Flag, c"Tc");
    tmp[tty_code_code::TTYC_TSL as usize] = tty_term_code_entry::new(tty_code_type::String, c"tsl");
    tmp[tty_code_code::TTYC_U8 as usize] = tty_term_code_entry::new(tty_code_type::Number, c"U8");
    tmp[tty_code_code::TTYC_VPA as usize] = tty_term_code_entry::new(tty_code_type::String, c"vpa");
    tmp[tty_code_code::TTYC_XT as usize] = tty_term_code_entry::new(tty_code_type::Flag, c"XT");

    tmp
};

pub const unsafe fn tty_term_ncodes() -> u32 {
    TTY_TERM_CODES.len() as u32
}

pub unsafe fn tty_term_strip(s: *const u8) -> *mut u8 {
    let sizeof_buf: usize = 8192;
    static mut BUF: [u8; 8192] = [0; 8192];

    unsafe {
        // const char *ptr;
        // static char buf[8192];
        // size_t len;

        // Ignore strings with no padding.
        if strchr(s, b'$' as i32).is_null() {
            return xstrdup(s).as_ptr();
        }

        let mut len = 0;
        let mut ptr = s;
        while *ptr != b'\0' {
            if *ptr == b'$' && *(ptr.add(1)) == b'<' {
                while *ptr != b'\0' && *ptr != b'>' {
                    ptr = ptr.add(1);
                }
                if *ptr == b'>' {
                    ptr = ptr.add(1);
                }
                if *ptr == b'\0' {
                    break;
                }
            }

            BUF[len] = *ptr;
            len += 1;
            if len == (sizeof_buf) - 1 {
                break;
            }
            ptr = ptr.add(1);
        }
        BUF[len] = b'\0';

        xstrdup(&raw mut BUF as *mut u8).as_ptr()
    }
}

pub unsafe fn tty_term_override_next(s: &str, offset: *mut usize) -> *mut u8 {
    const SIZEOF_VALUE: usize = 8192;
    static mut VALUE: [u8; 8192] = [0; 8192];

    unsafe {
        let remaining = s.as_bytes().get(*offset..).unwrap_or(&[]);

        if remaining.is_empty() {
            return null_mut();
        }

        let mut n = 0;
        let mut i = 0;

        while i < remaining.len() && n < SIZEOF_VALUE - 1 {
            if remaining[i] == b':' && remaining.get(i + 1) == Some(&b':') {
                VALUE[n] = b':';
                n += 1;
                i += 2;
            } else if remaining[i] == b':' {
                break;
            } else {
                VALUE[n] = remaining[i];
                n += 1;
                i += 1;
            }
        }

        if n >= SIZEOF_VALUE - 1 {
            return null_mut();
        }

        *offset += if i < remaining.len() { i + 1 } else { i };
        VALUE[n] = b'\0';

        &raw mut VALUE as *mut u8
    }
}

pub unsafe fn tty_term_apply(term: *mut tty_term, capabilities: &str, quiet: i32) {
    unsafe {
        let mut code: *mut tty_code;
        let mut offset = 0usize;
        let mut cp;
        let mut value;
        let mut s;

        let name = (*term).name;

        while {
            s = tty_term_override_next(capabilities, &raw mut offset);
            !s.is_null()
        } {
            if *s == b'\0' {
                continue;
            }
            value = null_mut();

            let mut remove = 0;
            cp = strchr(s, b'=' as i32);
            if !cp.is_null() {
                *cp = b'\0';
                cp = cp.add(1);
                value = xstrdup(cp).as_ptr();
                if strunvis(value, cp) == -1 {
                    free_(value);
                    value = xstrdup(cp).as_ptr();
                }
            } else if *s.add(strlen(s) - 1) == b'@' {
                *s.add(strlen(s) - 1) = b'\0';
                remove = 1;
            } else {
                value = xstrdup_(c"").as_ptr();
            }

            if quiet == 0 {
                if remove != 0 {
                    log_debug!("{} override: {}@", _s(name), _s(s));
                } else if *value == b'\0' {
                    log_debug!("{} override: {}", _s(name), _s(s));
                } else {
                    log_debug!("{} override: {}={}", _s(name), _s(s), _s(value));
                }
            }

            for i in 0..tty_term_ncodes() {
                let ent = &raw const TTY_TERM_CODES[i as usize];
                if strcmp(s, (*ent).name.as_ptr()) != 0 {
                    continue;
                }
                code = (*term).codes.add(i as usize);

                if remove != 0 {
                    (*code).type_ = tty_code_type::None;
                    continue;
                }
                match (*ent).type_ {
                    tty_code_type::None => (),
                    tty_code_type::String => {
                        if (*code).type_ == tty_code_type::String {
                            free_((*code).value.string);
                        }
                        (*code).value.string = xstrdup(value).as_ptr();
                        (*code).type_ = (*ent).type_;
                    }
                    tty_code_type::Number => {
                        let Ok(n) = strtonum(value, 0, i32::MAX) else {
                            break;
                        };
                        (*code).value.number = n;
                        (*code).type_ = (*ent).type_;
                    }
                    tty_code_type::Flag => {
                        (*code).value.flag = 1;
                        (*code).type_ = (*ent).type_;
                    }
                }
            }

            free_(value);
        }
    }
}

pub unsafe fn tty_term_apply_overrides(term: *mut tty_term) {
    let mut ov: *mut options_value;
    let mut s: *const u8;
    let mut acs: *const u8;
    let mut offset: usize;
    let mut first: *mut u8;

    unsafe {
        // Update capabilities from the option.
        let o = options_get_only(GLOBAL_OPTIONS, "terminal-overrides");
        let mut a = options_array_first(o);
        while !a.is_null() {
            ov = options_array_item_value(a);
            s = (*ov).string;

            offset = 0;
            first = tty_term_override_next(cstr_to_str(s), &raw mut offset);
            if !first.is_null() && fnmatch(first, (*term).name, 0) == 0 {
                tty_term_apply(term, cstr_to_str(s.add(offset)), 0);
            }
            a = options_array_next(a);
        }

        // Log the SIXEL flag.
        log_debug!(
            "SIXEL flag is {}",
            ((*term).flags & term_flags::TERM_SIXEL).bits()
        );

        // Update the RGB flag if the terminal has RGB colours.
        if tty_term_has(term, tty_code_code::TTYC_SETRGBF)
            && tty_term_has(term, tty_code_code::TTYC_SETRGBB)
        {
            (*term).flags |= term_flags::TERM_RGBCOLOURS;
        } else {
            (*term).flags &= !term_flags::TERM_RGBCOLOURS;
        }
        log_debug!(
            "RGBCOLOURS flag is {}",
            ((*term).flags & term_flags::TERM_RGBCOLOURS).bits()
        );

        // Set or clear the DECSLRM flag if the terminal has the margin
        // capabilities.
        if tty_term_has(term, tty_code_code::TTYC_CMG)
            && tty_term_has(term, tty_code_code::TTYC_CLMG)
        {
            (*term).flags |= term_flags::TERM_DECSLRM;
        } else {
            (*term).flags &= !term_flags::TERM_DECSLRM;
        }
        log_debug!(
            "DECSLRM flag is {}",
            ((*term).flags & term_flags::TERM_DECSLRM).bits()
        );

        // Set or clear the DECFRA flag if the terminal has the rectangle
        // capability.
        if tty_term_has(term, tty_code_code::TTYC_RECT) {
            (*term).flags |= term_flags::TERM_DECFRA;
        } else {
            (*term).flags &= !term_flags::TERM_DECFRA;
        }
        log_debug!(
            "DECFRA flag is {}",
            ((*term).flags & term_flags::TERM_DECFRA).bits()
        );

        // Terminals without am (auto right margin) wrap at at $COLUMNS - 1
        // rather than $COLUMNS (the cursor can never be beyond $COLUMNS - 1).
        //
        // Terminals without xenl (eat newline glitch) ignore a newline beyond
        // the right edge of the terminal, but tmux doesn't care about this -
        // it always uses absolute only moves the cursor with a newline when
        // also sending a linefeed.
        //
        // This is irritating, most notably because it is painful to write to
        // the very bottom-right of the screen without scrolling.
        //
        // Flag the terminal here and apply some workarounds in other places to
        // do the best possible.
        if tty_term_flag(term, tty_code_code::TTYC_AM) == 0 {
            (*term).flags |= term_flags::TERM_NOAM;
        } else {
            (*term).flags &= !term_flags::TERM_NOAM;
        }
        log_debug!(
            "NOAM flag is {}",
            ((*term).flags & term_flags::TERM_NOAM).bits()
        );

        // Generate ACS table. If none is present, use nearest ASCII.
        memset(
            &raw mut (*term).acs as *mut c_void,
            0,
            size_of::<[[i8; 2]; 256]>(),
        );
        if tty_term_has(term, tty_code_code::TTYC_ACSC) {
            acs = tty_term_string(term, tty_code_code::TTYC_ACSC);
        } else {
            acs = c!("a#j+k+l+m+n+o-p-q-r-s-t+u+v+w+x|y<z>~.");
        }
        while *acs != b'\0' && *acs.add(1) != b'\0' {
            (*term).acs[*acs as usize][0] = *acs.add(1);
            acs = acs.add(2);
        }
    }
}

pub unsafe fn tty_term_create(
    tty: *mut tty,
    name: *mut u8,
    caps: *mut *mut u8,
    ncaps: u32,
    feat: *mut i32,
    cause: *mut *mut u8,
) -> *mut tty_term {
    unsafe {
        log_debug!("adding term {}", _s(name));
        let term = xcalloc1::<tty_term>() as *mut tty_term;
        (*term).tty = tty;
        (*term).name = xstrdup(name).as_ptr();
        (*term).codes = xcalloc_(tty_term_ncodes() as usize).as_ptr();
        list_insert_head(&raw mut TTY_TERMS, term);
        'error: {
            // Fill in codes.
            for i in 0..ncaps as usize {
                let namelen = strcspn(*caps.add(i), c!("="));
                if namelen == 0 {
                    continue;
                }
                let value = (*caps.add(i)).add(namelen + 1);

                for (j, ent) in TTY_TERM_CODES.iter().enumerate() {
                    if strncmp(ent.name.as_ptr(), *caps.add(i), namelen) != 0 {
                        continue;
                    }
                    if *ent.name.as_ptr().add(namelen) != b'\0' {
                        continue;
                    }

                    let code = (*term).codes.add(j);
                    (*code).type_ = tty_code_type::None;
                    match ent.type_ {
                        tty_code_type::None => (),
                        tty_code_type::String => {
                            (*code).type_ = tty_code_type::String;
                            (*code).value.string = tty_term_strip(value);
                        }
                        tty_code_type::Number => match strtonum(value, 0, i32::MAX) {
                            Ok(n) => {
                                (*code).type_ = tty_code_type::Number;
                                (*code).value.number = n;
                            }
                            Err(errstr) => {
                                log_debug!(
                                    "{}: {}",
                                    _s(ent.name.as_ptr()),
                                    errstr.to_string_lossy()
                                );
                            }
                        },
                        tty_code_type::Flag => {
                            (*code).type_ = tty_code_type::Flag;
                            (*code).value.flag = (*value == b'1') as i32;
                        }
                    }
                }
            }

            // Apply terminal features.
            let o = options_get_only(GLOBAL_OPTIONS, "terminal-features");
            let mut a = options_array_first(o);
            while !a.is_null() {
                let ov = options_array_item_value(a);
                let s = (*ov).string;

                let mut offset = 0;
                let first = tty_term_override_next(cstr_to_str(s), &raw mut offset);
                if !first.is_null() && fnmatch(first, (*term).name, 0) == 0 {
                    tty_add_features(feat, cstr_to_str(s.add(offset)), c!(":"));
                }
                a = options_array_next(a);
            }

            // Delete curses data.
            // #if !defined(NCURSES_VERSION_MAJOR) || NCURSES_VERSION_MAJOR > 5 || (NCURSES_VERSION_MAJOR == 5 && NCURSES_VERSION_MINOR > 6)
            del_curterm(cur_term);
            // #endif

            // Apply overrides so any capabilities used for features are changed.
            tty_term_apply_overrides(term);

            // These are always required.
            if !tty_term_has(term, tty_code_code::TTYC_CLEAR) {
                *cause = format_nul!("terminal does not support clear");
                break 'error;
            }
            if !tty_term_has(term, tty_code_code::TTYC_CUP) {
                *cause = format_nul!("terminal does not support cup");
                break 'error;
            }

            // If TERM has XT or clear starts with CSI then it is safe to assume
            // the terminal is derived from the VT100. This controls whether device
            // attributes requests are sent to get more information.
            //
            // This is a bit of a hack but there aren't that many alternatives.
            // Worst case tmux will just fall back to using whatever terminfo(5)
            // says without trying to correct anything that is missing.
            //
            // Also add few features that VT100-like terminals should either
            // support or safely ignore.
            let s = tty_term_string(term, tty_code_code::TTYC_CLEAR);
            if tty_term_flag(term, tty_code_code::TTYC_XT) != 0 || strncmp(s, c!("\x1b["), 2) == 0 {
                (*term).flags |= term_flags::TERM_VT100LIKE;
                tty_add_features(feat, "bpaste,focus,title", c!(","));
            }

            // Add RGB feature if terminal has RGB colours.
            if (tty_term_flag(term, tty_code_code::TTYC_TC) != 0
                || tty_term_has(term, tty_code_code::TTYC_RGB))
                && (!tty_term_has(term, tty_code_code::TTYC_SETRGBF)
                    || !tty_term_has(term, tty_code_code::TTYC_SETRGBB))
            {
                tty_add_features(feat, "RGB", c!(","));
            }

            // Apply the features and overrides again.
            if tty_apply_features(term, *feat) {
                tty_term_apply_overrides(term);
            }

            // Log the capabilities.
            for i in 0..tty_term_ncodes() {
                log_debug!(
                    "{}{}",
                    _s(name),
                    _s(tty_term_describe(term, tty_code_code::try_from(i).unwrap()))
                );
            }

            return term;
        }

        // error:
        tty_term_free(term);
        null_mut()
    }
}

pub unsafe fn tty_term_free(term: *mut tty_term) {
    unsafe {
        log_debug!("removing term {}", _s((*term).name));

        for i in 0..tty_term_ncodes() as usize {
            if (*(*term).codes.add(i)).type_ == tty_code_type::String {
                free_((*(*term).codes.add(i)).value.string);
            }
        }
        free_((*term).codes);

        list_remove(term);
        free_((*term).name);
        free_(term);
    }
}

pub unsafe fn tty_term_read_list(
    name: *const u8,
    fd: i32,
    caps: *mut *mut *mut u8,
    ncaps: *mut u32,
    cause: *mut *mut u8,
) -> i32 {
    unsafe {
        let mut error = 0;
        let mut tmp: [u8; 11] = [0; 11];
        let sizeof_tmp = 11;

        if setupterm(name.cast(), fd, &raw mut error) != OK {
            match error {
                1 => *cause = format_nul!("can't use hardcopy terminal: {}", _s(name)),
                0 => *cause = format_nul!("missing or unsuitable terminal: {}", _s(name)),
                -1 => *cause = format_nul!("can't find terminfo database"),
                _ => *cause = format_nul!("unknown error"),
            }
            return -1;
        }

        *ncaps = 0;
        *caps = null_mut();

        let mut s = null();
        for ent in &TTY_TERM_CODES {
            match ent.type_ {
                tty_code_type::None => (),
                tty_code_type::String => {
                    s = tigetstr(ent.name.as_ptr());
                    if s.is_null() || s == (-1i32 as *const u8) {
                        continue;
                    }
                }
                tty_code_type::Number => {
                    let n = tigetnum(ent.name.as_ptr());
                    if n == -1 || n == -2 {
                        continue;
                    }
                    _ = xsnprintf_!(&raw mut tmp as *mut u8, sizeof_tmp, "{}", n);
                    s = &raw mut tmp as *const u8;
                }
                tty_code_type::Flag => {
                    let n = tigetflag(ent.name.as_ptr());
                    if n == -1 {
                        continue;
                    }
                    if n != 0 {
                        s = c!("1");
                    } else {
                        s = c!("0");
                    }
                }
            }
            *caps = xreallocarray((*caps).cast(), (*ncaps) as usize + 1, size_of::<*mut u8>())
                .as_ptr()
                .cast();
            *(*caps).add(*ncaps as usize) = format_nul!("{}={}", _s(ent.name.as_ptr()), _s(s));
            (*ncaps) += 1;
        }

        // #if !defined(NCURSES_VERSION_MAJOR) || NCURSES_VERSION_MAJOR > 5 || (NCURSES_VERSION_MAJOR == 5 && NCURSES_VERSION_MINOR > 6)
        del_curterm(cur_term);
        // #endif
        0
    }
}

pub unsafe fn tty_term_free_list(caps: *mut *mut u8, ncaps: u32) {
    unsafe {
        for i in 0..ncaps {
            free_(*caps.add(i as usize));
        }
        free_(caps);
    }
}

pub unsafe fn tty_term_has(term: *mut tty_term, code: tty_code_code) -> bool {
    unsafe { (*(*term).codes.add(code as usize)).type_ != tty_code_type::None }
}

pub unsafe fn tty_term_string(term: *mut tty_term, code: tty_code_code) -> *const u8 {
    unsafe {
        if !tty_term_has(term, code) {
            return c!("");
        }
        if (*(*term).codes.add(code as usize)).type_ != tty_code_type::String {
            fatalx_!("not a string: {}", code as u32);
        }
        (*(*term).codes.add(code as usize)).value.string
    }
}

pub unsafe fn tty_term_string_i(term: *mut tty_term, code: tty_code_code, a: i32) -> *const u8 {
    unsafe {
        let x = tty_term_string(term, code);

        // #if defined(HAVE_TIPARM_S)
        // s = tiparm_s(1, 0, x, a);
        // #elif defined(HAVE_TIPARM)
        // s = tiparm(x, a);
        // #else
        let s = tparm(x, a, 0, 0, 0, 0, 0, 0, 0, 0);
        // #endif
        if s.is_null() {
            log_debug!(
                "could not expand {}",
                _s(TTY_TERM_CODES[code as usize].name.as_ptr())
            );
            return c!("c");
        }
        s
    }
}

pub unsafe fn tty_term_string_ii(
    term: *mut tty_term,
    code: tty_code_code,
    a: i32,
    b: i32,
) -> *const u8 {
    unsafe {
        let x = tty_term_string(term, code);

        // TODO
        // #if defined(HAVE_TIPARM_S)
        // s = tiparm_s(2, 0, x, a, b);
        // #elif defined(HAVE_TIPARM)
        // s = tiparm(x, a, b);
        // #else
        let s = tparm(x, a, b, 0, 0, 0, 0, 0, 0, 0);
        // #endif
        if s.is_null() {
            log_debug!(
                "could not expand {}",
                _s(TTY_TERM_CODES[code as usize].name)
            );
            return c!("");
        }

        s
    }
}

pub unsafe fn tty_term_string_iii(
    term: *mut tty_term,
    code: tty_code_code,
    a: i32,
    b: i32,
    c: i32,
) -> *const u8 {
    unsafe {
        let x = tty_term_string(term, code);

        // TODO
        // #if defined(HAVE_TIPARM_S)
        // s = tiparm_s(3, 0, x, a, b, c);
        // #elif defined(HAVE_TIPARM)
        // s = tiparm(x, a, b, c);
        // #else
        let s = tparm(x, a, b, c, 0, 0, 0, 0, 0, 0);
        // #endif
        if s.is_null() {
            log_debug!(
                "could not expand {}",
                _s(TTY_TERM_CODES[code as usize].name)
            );
            return c!("");
        }
        s
    }
}

pub unsafe fn tty_term_string_s(
    term: *mut tty_term,
    code: tty_code_code,
    a: *const u8,
) -> *const u8 {
    unsafe {
        let x = tty_term_string(term, code);

        // TODO
        // #if defined(HAVE_TIPARM_S)
        // s = tiparm_s(1, 1, x, a);
        // #elif defined(HAVE_TIPARM)
        // s = tiparm(x, a);
        // #else
        let s: *mut u8 = tparm(x.cast(), a as c_long, 0, 0, 0, 0, 0, 0, 0, 0);
        // #endif
        if s.is_null() {
            log_debug!(
                "could not expand {}",
                _s(TTY_TERM_CODES[code as usize].name)
            );
            return c!("");
        }

        s
    }
}

pub unsafe fn tty_term_string_ss(
    term: *mut tty_term,
    code: tty_code_code,
    a: *const u8,
    b: *const u8,
) -> *const u8 {
    unsafe {
        let x = tty_term_string(term, code);
        // *s;

        // #if defined(HAVE_TIPARM_S)
        // let s = tiparm_s(2, 3, x, a, b);
        // #elif defined(HAVE_TIPARM)
        // let s = tiparm(x, a, b);
        // #else
        // TODO
        let s = tparm(x.cast(), a as c_long, b as c_long, 0, 0, 0, 0, 0, 0, 0).cast::<u8>();
        // #endif
        if s.is_null() {
            log_debug!(
                "could not expand {}",
                _s(TTY_TERM_CODES[code as usize].name)
            );
            return c!("");
        }

        s
    }
}

pub unsafe fn tty_term_number(term: *mut tty_term, code: tty_code_code) -> i32 {
    unsafe {
        if !tty_term_has(term, code) {
            return 0;
        }
        if (*(*term).codes.add(code as usize)).type_ != tty_code_type::Number {
            fatalx_!("not a number: {}", code as u32);
        }
        (*(*term).codes.add(code as usize)).value.number
    }
}

pub unsafe fn tty_term_flag(term: *mut tty_term, code: tty_code_code) -> i32 {
    unsafe {
        if !tty_term_has(term, code) {
            return 0;
        }
        if (*(*term).codes.add(code as usize)).type_ != tty_code_type::Flag {
            fatalx_!("not a flag: {}", code as u32);
        }
        (*(*term).codes.add(code as usize)).value.flag
    }
}

pub unsafe fn tty_term_describe(term: *mut tty_term, code: tty_code_code) -> *const u8 {
    let sizeof_s = 256;
    static mut S: [u8; 256] = [0; 256];

    unsafe {
        let sizeof_out = 128;
        let mut out: [u8; 128] = [0; 128];

        match (*(*term).codes.add(code as usize)).type_ {
            tty_code_type::None => {
                _ = xsnprintf_!(
                    &raw mut S as _,
                    sizeof_s,
                    "{:4}: {}: [missing]",
                    code as u32,
                    _s(TTY_TERM_CODES[code as usize].name),
                );
            }
            tty_code_type::String => {
                strnvis(
                    &raw mut out as *mut u8,
                    (*(*term).codes.add(code as usize)).value.string,
                    sizeof_out,
                    vis_flags::VIS_OCTAL
                        | vis_flags::VIS_CSTYLE
                        | vis_flags::VIS_TAB
                        | vis_flags::VIS_NL,
                );
                _ = xsnprintf_!(
                    &raw mut S as _,
                    sizeof_s,
                    "{:4}: {}: (string) {}",
                    code as u32,
                    _s(TTY_TERM_CODES[code as usize].name),
                    _s(out.as_ptr()),
                );
            }
            tty_code_type::Number => {
                _ = xsnprintf_!(
                    &raw mut S as _,
                    sizeof_s,
                    "{:4}: {}: (number) {}",
                    code as u32,
                    _s(TTY_TERM_CODES[code as usize].name),
                    (*(*term).codes.add(code as usize)).value.number,
                );
            }
            tty_code_type::Flag => {
                _ = xsnprintf_!(
                    &raw mut S as _,
                    sizeof_s,
                    "{:4}: {}: (flag) {}",
                    code as u32,
                    _s(TTY_TERM_CODES[code as usize].name),
                    (*(*term).codes.add(code as usize)).value.flag != 0
                );
            }
        }

        &raw const S as *const u8
    }
}
