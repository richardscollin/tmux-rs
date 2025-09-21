// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
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

//! Based on the description by Paul Williams at:
//!
//! <https://vt100.net/emu/dec_ansi_parser>
//!
//! With the following changes:
//!
//! - 7-bit only.
//!
//! - Support for UTF-8.
//!
//! - OSC (but not APC) may be terminated by \007 as well as ST.
//!
//! - A state for APC similar to OSC. Some terminals appear to use this to set
//!   the title.
//!
//! - A state for the screen \033k...\033\\ sequence to rename a window. This is
//!   pretty stupid but not supporting it is more trouble than it is worth.
//!
//! - Special handling for ESC inside a DCS to allow arbitrary byte sequences to
//!   be passed to the underlying terminals.
use crate::compat::b64::{b64_ntop, b64_pton};
use crate::libc::{strchr, strpbrk, strtol};
use crate::*;

// Input parser cell.
#[repr(C)]
struct input_cell {
    cell: grid_cell,
    set: i32,
    g0set: i32, // 1 if ACS
    g1set: i32, // 1 if ACS
}

#[repr(i32)]
#[derive(Eq, PartialEq)]
enum input_param_type {
    INPUT_MISSING,
    INPUT_NUMBER,
    INPUT_STRING,
}
#[repr(C)]
union input_param_union {
    num: i32,
    str: *mut u8,
}

/// Input parser argument.
#[repr(C)]
struct input_param {
    type_: input_param_type,
    union_: input_param_union,
}

const INPUT_BUF_START: usize = 32;
const INPUT_BUF_LIMIT: usize = 1048576;

bitflags::bitflags! {
    #[repr(C)]
    pub struct input_flags : i32 {
        const INPUT_DISCARD = 0x1;
        const INPUT_LAST = 0x2;
    }
}

#[repr(i32)]
#[derive(Eq, PartialEq)]
enum input_end_type {
    INPUT_END_ST,
    INPUT_END_BEL,
}

/// Input parser context.
#[repr(C)]
pub struct input_ctx {
    wp: *mut window_pane,
    event: *mut bufferevent,
    ctx: screen_write_ctx,
    palette: *mut colour_palette,

    cell: input_cell,

    old_cell: input_cell,
    old_cx: u32,
    old_cy: u32,
    old_mode: mode_flag,

    interm_buf: [u8; 4],
    interm_len: usize,

    param_buf: [u8; 64],
    param_len: usize,

    input_buf: *mut u8,
    input_len: usize,
    input_space: usize,
    input_end: input_end_type,

    param_list: [input_param; 24],
    param_list_len: u32,

    utf8data: utf8_data,
    utf8started: i32,

    ch: i32,
    last: utf8_data,

    flags: input_flags,

    state: *const input_state,

    timer: event,

    /// All input received since we were last in the ground state. Sent to control clients on connection.
    since_ground: *mut evbuffer,
}

// Command table entry.
#[repr(C)]
struct input_table_entry {
    ch: i32,
    interm: SyncCharPtr,
    type_: i32,
}

impl input_table_entry {
    const fn new_csi(ch: char, interm: &'static CStr, type_: input_csi_type) -> Self {
        Self {
            ch: ch as i32,
            interm: SyncCharPtr::new(interm),
            type_: type_ as i32,
        }
    }
    const fn new_esc(ch: char, interm: &'static CStr, type_: input_esc_type) -> Self {
        Self {
            ch: ch as i32,
            interm: SyncCharPtr::new(interm),
            type_: type_ as i32,
        }
    }
}

// Escape commands.
#[derive(num_enum::TryFromPrimitive)]
#[repr(i32)]
enum input_esc_type {
    INPUT_ESC_DECALN,
    INPUT_ESC_DECKPAM,
    INPUT_ESC_DECKPNM,
    INPUT_ESC_DECRC,
    INPUT_ESC_DECSC,
    INPUT_ESC_HTS,
    INPUT_ESC_IND,
    INPUT_ESC_NEL,
    INPUT_ESC_RI,
    INPUT_ESC_RIS,
    INPUT_ESC_SCSG0_OFF,
    INPUT_ESC_SCSG0_ON,
    INPUT_ESC_SCSG1_OFF,
    INPUT_ESC_SCSG1_ON,
    INPUT_ESC_ST,
}

/// Escape command table.
static INPUT_ESC_TABLE: [input_table_entry; 15] = [
    input_table_entry::new_esc('0', c"(", input_esc_type::INPUT_ESC_SCSG0_ON),
    input_table_entry::new_esc('0', c")", input_esc_type::INPUT_ESC_SCSG1_ON),
    input_table_entry::new_esc('7', c"", input_esc_type::INPUT_ESC_DECSC),
    input_table_entry::new_esc('8', c"", input_esc_type::INPUT_ESC_DECRC),
    input_table_entry::new_esc('8', c"#", input_esc_type::INPUT_ESC_DECALN),
    input_table_entry::new_esc('=', c"", input_esc_type::INPUT_ESC_DECKPAM),
    input_table_entry::new_esc('>', c"", input_esc_type::INPUT_ESC_DECKPNM),
    input_table_entry::new_esc('B', c"(", input_esc_type::INPUT_ESC_SCSG0_OFF),
    input_table_entry::new_esc('B', c")", input_esc_type::INPUT_ESC_SCSG1_OFF),
    input_table_entry::new_esc('D', c"", input_esc_type::INPUT_ESC_IND),
    input_table_entry::new_esc('E', c"", input_esc_type::INPUT_ESC_NEL),
    input_table_entry::new_esc('H', c"", input_esc_type::INPUT_ESC_HTS),
    input_table_entry::new_esc('M', c"", input_esc_type::INPUT_ESC_RI),
    input_table_entry::new_esc('\\', c"", input_esc_type::INPUT_ESC_ST),
    input_table_entry::new_esc('c', c"", input_esc_type::INPUT_ESC_RIS),
];

/// Control (CSI) commands.
#[derive(num_enum::TryFromPrimitive)]
#[repr(i32)]
enum input_csi_type {
    INPUT_CSI_CBT,
    INPUT_CSI_CNL,
    INPUT_CSI_CPL,
    INPUT_CSI_CUB,
    INPUT_CSI_CUD,
    INPUT_CSI_CUF,
    INPUT_CSI_CUP,
    INPUT_CSI_CUU,
    INPUT_CSI_DA,
    INPUT_CSI_DA_TWO,
    INPUT_CSI_DCH,
    INPUT_CSI_DECSCUSR,
    INPUT_CSI_DECSTBM,
    INPUT_CSI_DL,
    INPUT_CSI_DSR,
    INPUT_CSI_ECH,
    INPUT_CSI_ED,
    INPUT_CSI_EL,
    INPUT_CSI_HPA,
    INPUT_CSI_ICH,
    INPUT_CSI_IL,
    INPUT_CSI_MODOFF,
    INPUT_CSI_MODSET,
    INPUT_CSI_RCP,
    INPUT_CSI_REP,
    INPUT_CSI_RM,
    INPUT_CSI_RM_PRIVATE,
    INPUT_CSI_SCP,
    INPUT_CSI_SD,
    INPUT_CSI_SGR,
    INPUT_CSI_SM,
    INPUT_CSI_SM_PRIVATE,
    INPUT_CSI_SM_GRAPHICS,
    INPUT_CSI_SU,
    INPUT_CSI_TBC,
    INPUT_CSI_VPA,
    INPUT_CSI_WINOPS,
    INPUT_CSI_XDA,
}

/// control (csi) command table.
static INPUT_CSI_TABLE: [input_table_entry; 40] = [
    input_table_entry::new_csi('@', c"", input_csi_type::INPUT_CSI_ICH),
    input_table_entry::new_csi('A', c"", input_csi_type::INPUT_CSI_CUU),
    input_table_entry::new_csi('B', c"", input_csi_type::INPUT_CSI_CUD),
    input_table_entry::new_csi('C', c"", input_csi_type::INPUT_CSI_CUF),
    input_table_entry::new_csi('D', c"", input_csi_type::INPUT_CSI_CUB),
    input_table_entry::new_csi('E', c"", input_csi_type::INPUT_CSI_CNL),
    input_table_entry::new_csi('F', c"", input_csi_type::INPUT_CSI_CPL),
    input_table_entry::new_csi('G', c"", input_csi_type::INPUT_CSI_HPA),
    input_table_entry::new_csi('H', c"", input_csi_type::INPUT_CSI_CUP),
    input_table_entry::new_csi('J', c"", input_csi_type::INPUT_CSI_ED),
    input_table_entry::new_csi('K', c"", input_csi_type::INPUT_CSI_EL),
    input_table_entry::new_csi('L', c"", input_csi_type::INPUT_CSI_IL),
    input_table_entry::new_csi('M', c"", input_csi_type::INPUT_CSI_DL),
    input_table_entry::new_csi('P', c"", input_csi_type::INPUT_CSI_DCH),
    input_table_entry::new_csi('S', c"", input_csi_type::INPUT_CSI_SU),
    input_table_entry::new_csi('S', c"?", input_csi_type::INPUT_CSI_SM_GRAPHICS),
    input_table_entry::new_csi('T', c"", input_csi_type::INPUT_CSI_SD),
    input_table_entry::new_csi('X', c"", input_csi_type::INPUT_CSI_ECH),
    input_table_entry::new_csi('Z', c"", input_csi_type::INPUT_CSI_CBT),
    input_table_entry::new_csi('`', c"", input_csi_type::INPUT_CSI_HPA),
    input_table_entry::new_csi('b', c"", input_csi_type::INPUT_CSI_REP),
    input_table_entry::new_csi('c', c"", input_csi_type::INPUT_CSI_DA),
    input_table_entry::new_csi('c', c">", input_csi_type::INPUT_CSI_DA_TWO),
    input_table_entry::new_csi('d', c"", input_csi_type::INPUT_CSI_VPA),
    input_table_entry::new_csi('f', c"", input_csi_type::INPUT_CSI_CUP),
    input_table_entry::new_csi('g', c"", input_csi_type::INPUT_CSI_TBC),
    input_table_entry::new_csi('h', c"", input_csi_type::INPUT_CSI_SM),
    input_table_entry::new_csi('h', c"?", input_csi_type::INPUT_CSI_SM_PRIVATE),
    input_table_entry::new_csi('l', c"", input_csi_type::INPUT_CSI_RM),
    input_table_entry::new_csi('l', c"?", input_csi_type::INPUT_CSI_RM_PRIVATE),
    input_table_entry::new_csi('m', c"", input_csi_type::INPUT_CSI_SGR),
    input_table_entry::new_csi('m', c">", input_csi_type::INPUT_CSI_MODSET),
    input_table_entry::new_csi('n', c"", input_csi_type::INPUT_CSI_DSR),
    input_table_entry::new_csi('n', c">", input_csi_type::INPUT_CSI_MODOFF),
    input_table_entry::new_csi('q', c" ", input_csi_type::INPUT_CSI_DECSCUSR),
    input_table_entry::new_csi('q', c">", input_csi_type::INPUT_CSI_XDA),
    input_table_entry::new_csi('r', c"", input_csi_type::INPUT_CSI_DECSTBM),
    input_table_entry::new_csi('s', c"", input_csi_type::INPUT_CSI_SCP),
    input_table_entry::new_csi('t', c"", input_csi_type::INPUT_CSI_WINOPS),
    input_table_entry::new_csi('u', c"", input_csi_type::INPUT_CSI_RCP),
];

/// Input transition.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct input_transition {
    first: i32,
    last: i32,

    handler: Option<unsafe fn(*mut input_ctx) -> i32>,
    state: Option<&'static input_state>,
}
impl input_transition {
    const fn new(
        first: i32,
        last: i32,
        handler: Option<unsafe fn(*mut input_ctx) -> i32>,
        state: Option<&'static input_state>,
    ) -> Self {
        Self {
            first,
            last,
            handler,
            state,
        }
    }
}

// Input state.
#[repr(C)]
pub struct input_state {
    name: SyncCharPtr,
    enter: Option<unsafe fn(*mut input_ctx)>,
    exit: Option<unsafe fn(*mut input_ctx)>,
    transitions: Option<&'static [input_transition]>,
}

impl input_state {
    const fn new(
        name: &'static CStr,
        enter: Option<unsafe fn(*mut input_ctx)>,
        exit: Option<unsafe fn(*mut input_ctx)>,
        transitions: Option<&'static [input_transition]>,
    ) -> Self {
        Self {
            name: SyncCharPtr::new(name),
            enter,
            exit,
            transitions,
        }
    }
}

// State transitions available from all states.
// defined as a macro to break type checking cycle
macro_rules! INPUT_STATE_ANYWHERE {
    () => {
        [
            input_transition::new(
                0x18,
                0x18,
                Some(input_c0_dispatch),
                Some(&INPUT_STATE_GROUND),
            ),
            input_transition::new(
                0x1a,
                0x1a,
                Some(input_c0_dispatch),
                Some(&INPUT_STATE_GROUND),
            ),
            input_transition::new(0x1b, 0x1b, None, Some(&INPUT_STATE_ESC_ENTER)),
        ]
    };
}

pub static INPUT_STATE_GROUND: input_state = input_state::new(
    c"ground",
    Some(input_ground),
    None,
    Some(&INPUT_STATE_GROUND_TABLE),
);

pub static INPUT_STATE_ESC_ENTER: input_state = input_state::new(
    c"esc_enter",
    Some(input_clear),
    None,
    Some(&INPUT_STATE_ESC_ENTER_TABLE),
);

pub static INPUT_STATE_ESC_INTERMEDIATE: input_state = input_state::new(
    c"esc_intermediate",
    None,
    None,
    Some(&INPUT_STATE_ESC_INTERMEDIATE_TABLE),
);

pub static INPUT_STATE_CSI_ENTER: input_state = input_state::new(
    c"csi_enter",
    Some(input_clear),
    None,
    Some(&INPUT_STATE_CSI_ENTER_TABLE),
);

pub static INPUT_STATE_CSI_PARAMETER: input_state = input_state::new(
    c"csi_parameter",
    None,
    None,
    Some(&INPUT_STATE_CSI_PARAMETER_TABLE),
);

pub static INPUT_STATE_CSI_INTERMEDIATE: input_state = input_state::new(
    c"csi_intermediate",
    None,
    None,
    Some(&INPUT_STATE_CSI_INTERMEDIATE_TABLE),
);

pub static INPUT_STATE_CSI_IGNORE: input_state = input_state::new(
    c"csi_ignore",
    None,
    None,
    Some(&INPUT_STATE_CSI_IGNORE_TABLE),
);

pub static INPUT_STATE_DCS_ENTER: input_state = input_state::new(
    c"dcs_enter",
    Some(input_enter_dcs),
    None,
    Some(&INPUT_STATE_DCS_ENTER_TABLE),
);

pub static INPUT_STATE_DCS_PARAMETER: input_state = input_state::new(
    c"dcs_parameter",
    None,
    None,
    Some(&INPUT_STATE_DCS_PARAMETER_TABLE),
);

pub static INPUT_STATE_DCS_INTERMEDIATE: input_state = input_state::new(
    c"dcs_intermediate",
    None,
    None,
    Some(&INPUT_STATE_DCS_INTERMEDIATE_TABLE),
);

pub static INPUT_STATE_DCS_HANDLER: input_state = input_state::new(
    c"dcs_handler",
    None,
    None,
    Some(&INPUT_STATE_DCS_HANDLER_TABLE),
);

pub static INPUT_STATE_DCS_ESCAPE: input_state = input_state::new(
    c"dcs_escape",
    None,
    None,
    Some(&INPUT_STATE_DCS_ESCAPE_TABLE),
);

pub static INPUT_STATE_DCS_IGNORE: input_state = input_state::new(
    c"dcs_ignore",
    None,
    None,
    Some(&INPUT_STATE_DCS_IGNORE_TABLE),
);

pub static INPUT_STATE_OSC_STRING: input_state = input_state::new(
    c"osc_string",
    Some(input_enter_osc),
    Some(input_exit_osc),
    Some(&INPUT_STATE_OSC_STRING_TABLE),
);

pub static INPUT_STATE_APC_STRING: input_state = input_state::new(
    c"apc_string",
    Some(input_enter_apc),
    Some(input_exit_apc),
    Some(&INPUT_STATE_APC_STRING_TABLE),
);

pub static INPUT_STATE_RENAME_STRING: input_state = input_state::new(
    c"rename_string",
    Some(input_enter_rename),
    Some(input_exit_rename),
    Some(&INPUT_STATE_RENAME_STRING_TABLE),
);

pub static INPUT_STATE_CONSUME_ST: input_state = input_state::new(
    c"consume_st",
    Some(input_enter_rename),
    None,
    // rename also waits for ST
    Some(&INPUT_STATE_CONSUME_ST_TABLE),
);

static INPUT_STATE_GROUND_TABLE: [input_transition; 9] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, Some(input_c0_dispatch), None),
        input_transition::new(0x19, 0x19, Some(input_c0_dispatch), None),
        input_transition::new(0x1c, 0x1f, Some(input_c0_dispatch), None),
        input_transition::new(0x20, 0x7e, Some(input_print), None),
        input_transition::new(0x7f, 0x7f, None, None),
        input_transition::new(0x80, 0xff, Some(input_top_bit_set), None),
    ],
);

static INPUT_STATE_ESC_ENTER_TABLE: [input_transition; 22] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, Some(input_c0_dispatch), None),
        input_transition::new(0x19, 0x19, Some(input_c0_dispatch), None),
        input_transition::new(0x1c, 0x1f, Some(input_c0_dispatch), None),
        input_transition::new(
            0x20,
            0x2f,
            Some(input_intermediate),
            Some(&INPUT_STATE_ESC_INTERMEDIATE),
        ),
        input_transition::new(
            0x30,
            0x4f,
            Some(input_esc_dispatch),
            Some(&INPUT_STATE_GROUND),
        ),
        input_transition::new(0x50, 0x50, None, Some(&INPUT_STATE_DCS_ENTER)),
        input_transition::new(
            0x51,
            0x57,
            Some(input_esc_dispatch),
            Some(&INPUT_STATE_GROUND),
        ),
        input_transition::new(0x58, 0x58, None, Some(&INPUT_STATE_CONSUME_ST)),
        input_transition::new(
            0x59,
            0x59,
            Some(input_esc_dispatch),
            Some(&INPUT_STATE_GROUND),
        ),
        input_transition::new(
            0x5a,
            0x5a,
            Some(input_esc_dispatch),
            Some(&INPUT_STATE_GROUND),
        ),
        input_transition::new(0x5b, 0x5b, None, Some(&INPUT_STATE_CSI_ENTER)),
        input_transition::new(
            0x5c,
            0x5c,
            Some(input_esc_dispatch),
            Some(&INPUT_STATE_GROUND),
        ),
        input_transition::new(0x5d, 0x5d, None, Some(&INPUT_STATE_OSC_STRING)),
        input_transition::new(0x5e, 0x5e, None, Some(&INPUT_STATE_CONSUME_ST)),
        input_transition::new(0x5f, 0x5f, None, Some(&INPUT_STATE_APC_STRING)),
        input_transition::new(
            0x60,
            0x6a,
            Some(input_esc_dispatch),
            Some(&INPUT_STATE_GROUND),
        ),
        input_transition::new(0x6b, 0x6b, None, Some(&INPUT_STATE_RENAME_STRING)),
        input_transition::new(
            0x6c,
            0x7e,
            Some(input_esc_dispatch),
            Some(&INPUT_STATE_GROUND),
        ),
        input_transition::new(0x7f, 0xff, None, None),
    ],
);

static INPUT_STATE_ESC_INTERMEDIATE_TABLE: [input_transition; 9] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, Some(input_c0_dispatch), None),
        input_transition::new(0x19, 0x19, Some(input_c0_dispatch), None),
        input_transition::new(0x1c, 0x1f, Some(input_c0_dispatch), None),
        input_transition::new(0x20, 0x2f, Some(input_intermediate), None),
        input_transition::new(
            0x30,
            0x7e,
            Some(input_esc_dispatch),
            Some(&INPUT_STATE_GROUND),
        ),
        input_transition::new(0x7f, 0xff, None, None),
    ],
);

static INPUT_STATE_CSI_ENTER_TABLE: [input_transition; 13] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, Some(input_c0_dispatch), None),
        input_transition::new(0x19, 0x19, Some(input_c0_dispatch), None),
        input_transition::new(0x1c, 0x1f, Some(input_c0_dispatch), None),
        input_transition::new(
            0x20,
            0x2f,
            Some(input_intermediate),
            Some(&INPUT_STATE_CSI_INTERMEDIATE),
        ),
        input_transition::new(
            0x30,
            0x39,
            Some(input_parameter),
            Some(&INPUT_STATE_CSI_PARAMETER),
        ),
        input_transition::new(
            0x3a,
            0x3a,
            Some(input_parameter),
            Some(&INPUT_STATE_CSI_PARAMETER),
        ),
        input_transition::new(
            0x3b,
            0x3b,
            Some(input_parameter),
            Some(&INPUT_STATE_CSI_PARAMETER),
        ),
        input_transition::new(
            0x3c,
            0x3f,
            Some(input_intermediate),
            Some(&INPUT_STATE_CSI_PARAMETER),
        ),
        input_transition::new(
            0x40,
            0x7e,
            Some(input_csi_dispatch),
            Some(&INPUT_STATE_GROUND),
        ),
        input_transition::new(0x7f, 0xff, None, None),
    ],
);

static INPUT_STATE_CSI_PARAMETER_TABLE: [input_transition; 13] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, Some(input_c0_dispatch), None),
        input_transition::new(0x19, 0x19, Some(input_c0_dispatch), None),
        input_transition::new(0x1c, 0x1f, Some(input_c0_dispatch), None),
        input_transition::new(
            0x20,
            0x2f,
            Some(input_intermediate),
            Some(&INPUT_STATE_CSI_INTERMEDIATE),
        ),
        input_transition::new(0x30, 0x39, Some(input_parameter), None),
        input_transition::new(0x3a, 0x3a, Some(input_parameter), None),
        input_transition::new(0x3b, 0x3b, Some(input_parameter), None),
        input_transition::new(0x3c, 0x3f, None, Some(&INPUT_STATE_CSI_IGNORE)),
        input_transition::new(
            0x40,
            0x7e,
            Some(input_csi_dispatch),
            Some(&INPUT_STATE_GROUND),
        ),
        input_transition::new(0x7f, 0xff, None, None),
    ],
);

static INPUT_STATE_CSI_INTERMEDIATE_TABLE: [input_transition; 10] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, Some(input_c0_dispatch), None),
        input_transition::new(0x19, 0x19, Some(input_c0_dispatch), None),
        input_transition::new(0x1c, 0x1f, Some(input_c0_dispatch), None),
        input_transition::new(0x20, 0x2f, Some(input_intermediate), None),
        input_transition::new(0x30, 0x3f, None, Some(&INPUT_STATE_CSI_IGNORE)),
        input_transition::new(
            0x40,
            0x7e,
            Some(input_csi_dispatch),
            Some(&INPUT_STATE_GROUND),
        ),
        input_transition::new(0x7f, 0xff, None, None),
    ],
);

static INPUT_STATE_CSI_IGNORE_TABLE: [input_transition; 9] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, Some(input_c0_dispatch), None),
        input_transition::new(0x19, 0x19, Some(input_c0_dispatch), None),
        input_transition::new(0x1c, 0x1f, Some(input_c0_dispatch), None),
        input_transition::new(0x20, 0x3f, None, None),
        input_transition::new(0x40, 0x7e, None, Some(&INPUT_STATE_GROUND)),
        input_transition::new(0x7f, 0xff, None, None),
    ],
);

static INPUT_STATE_DCS_ENTER_TABLE: [input_transition; 13] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, None, None),
        input_transition::new(0x19, 0x19, None, None),
        input_transition::new(0x1c, 0x1f, None, None),
        input_transition::new(
            0x20,
            0x2f,
            Some(input_intermediate),
            Some(&INPUT_STATE_DCS_INTERMEDIATE),
        ),
        input_transition::new(
            0x30,
            0x39,
            Some(input_parameter),
            Some(&INPUT_STATE_DCS_PARAMETER),
        ),
        input_transition::new(0x3a, 0x3a, None, Some(&INPUT_STATE_DCS_IGNORE)),
        input_transition::new(
            0x3b,
            0x3b,
            Some(input_parameter),
            Some(&INPUT_STATE_DCS_PARAMETER),
        ),
        input_transition::new(
            0x3c,
            0x3f,
            Some(input_intermediate),
            Some(&INPUT_STATE_DCS_PARAMETER),
        ),
        input_transition::new(
            0x40,
            0x7e,
            Some(input_input),
            Some(&INPUT_STATE_DCS_HANDLER),
        ),
        input_transition::new(0x7f, 0xff, None, None),
    ],
);

static INPUT_STATE_DCS_PARAMETER_TABLE: [input_transition; 13] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, None, None),
        input_transition::new(0x19, 0x19, None, None),
        input_transition::new(0x1c, 0x1f, None, None),
        input_transition::new(
            0x20,
            0x2f,
            Some(input_intermediate),
            Some(&INPUT_STATE_DCS_INTERMEDIATE),
        ),
        input_transition::new(0x30, 0x39, Some(input_parameter), None),
        input_transition::new(0x3a, 0x3a, None, Some(&INPUT_STATE_DCS_IGNORE)),
        input_transition::new(0x3b, 0x3b, Some(input_parameter), None),
        input_transition::new(0x3c, 0x3f, None, Some(&INPUT_STATE_DCS_IGNORE)),
        input_transition::new(
            0x40,
            0x7e,
            Some(input_input),
            Some(&INPUT_STATE_DCS_HANDLER),
        ),
        input_transition::new(0x7f, 0xff, None, None),
    ],
);

static INPUT_STATE_DCS_INTERMEDIATE_TABLE: [input_transition; 10] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, None, None),
        input_transition::new(0x19, 0x19, None, None),
        input_transition::new(0x1c, 0x1f, None, None),
        input_transition::new(0x20, 0x2f, Some(input_intermediate), None),
        input_transition::new(0x30, 0x3f, None, Some(&INPUT_STATE_DCS_IGNORE)),
        input_transition::new(
            0x40,
            0x7e,
            Some(input_input),
            Some(&INPUT_STATE_DCS_HANDLER),
        ),
        input_transition::new(0x7f, 0xff, None, None),
    ],
);

static INPUT_STATE_DCS_HANDLER_TABLE: [input_transition; 3] = [
    // No INPUT_STATE_ANYWHERE
    input_transition::new(0x00, 0x1a, Some(input_input), None),
    input_transition::new(0x1b, 0x1b, None, Some(&INPUT_STATE_DCS_ESCAPE)),
    input_transition::new(0x1c, 0xff, Some(input_input), None),
];

static INPUT_STATE_DCS_ESCAPE_TABLE: [input_transition; 3] = [
    // No INPUT_STATE_ANYWHERE
    input_transition::new(
        0x00,
        0x5b,
        Some(input_input),
        Some(&INPUT_STATE_DCS_HANDLER),
    ),
    input_transition::new(
        0x5c,
        0x5c,
        Some(input_dcs_dispatch),
        Some(&INPUT_STATE_GROUND),
    ),
    input_transition::new(
        0x5d,
        0xff,
        Some(input_input),
        Some(&INPUT_STATE_DCS_HANDLER),
    ),
];

static INPUT_STATE_DCS_IGNORE_TABLE: [input_transition; 7] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, None, None),
        input_transition::new(0x19, 0x19, None, None),
        input_transition::new(0x1c, 0x1f, None, None),
        input_transition::new(0x20, 0xff, None, None),
    ],
);

static INPUT_STATE_OSC_STRING_TABLE: [input_transition; 9] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x06, None, None),
        input_transition::new(0x07, 0x07, Some(input_end_bel), Some(&INPUT_STATE_GROUND)),
        input_transition::new(0x08, 0x17, None, None),
        input_transition::new(0x19, 0x19, None, None),
        input_transition::new(0x1c, 0x1f, None, None),
        input_transition::new(0x20, 0xff, Some(input_input), None),
    ],
);

static INPUT_STATE_APC_STRING_TABLE: [input_transition; 7] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, None, None),
        input_transition::new(0x19, 0x19, None, None),
        input_transition::new(0x1c, 0x1f, None, None),
        input_transition::new(0x20, 0xff, Some(input_input), None),
    ],
);

static INPUT_STATE_RENAME_STRING_TABLE: [input_transition; 7] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, None, None),
        input_transition::new(0x19, 0x19, None, None),
        input_transition::new(0x1c, 0x1f, None, None),
        input_transition::new(0x20, 0xff, Some(input_input), None),
    ],
);

static INPUT_STATE_CONSUME_ST_TABLE: [input_transition; 7] = concat_array(
    INPUT_STATE_ANYWHERE!(),
    [
        input_transition::new(0x00, 0x17, None, None),
        input_transition::new(0x19, 0x19, None, None),
        input_transition::new(0x1c, 0x1f, None, None),
        input_transition::new(0x20, 0xff, None, None),
    ],
);

unsafe fn input_table_compare(
    ictx: *const input_ctx,
    entry: &input_table_entry,
) -> std::cmp::Ordering {
    unsafe {
        (*ictx).ch.cmp(&entry.ch).then_with(|| {
            i32_to_ordering(libc::strcmp(
                (&raw const (*ictx).interm_buf).cast(),
                entry.interm.as_ptr().cast(),
            ))
        })
    }
}

/// Timer
///
/// if this expires then have been waiting for a terminator for too long, so reset to ground.
unsafe extern "C-unwind" fn input_timer_callback(_fd: i32, _events: i16, ictx: NonNull<input_ctx>) {
    unsafe {
        log_debug!(
            "{}: {} expired",
            "input_timer_callback",
            _s((*(*ictx.as_ptr()).state).name.as_ptr())
        );
        input_reset(ictx.as_ptr(), 0);
    }
}

/// Start the timer.
unsafe fn input_start_timer(ictx: *mut input_ctx) {
    unsafe {
        let tv: timeval = timeval {
            tv_sec: 5,
            tv_usec: 0,
        };

        event_del(&raw mut (*ictx).timer);
        event_add(&raw mut (*ictx).timer, &raw const tv);
    }
}

/// Reset cell state to default.
unsafe fn input_reset_cell(ictx: *mut input_ctx) {
    unsafe {
        memcpy__(&raw mut (*ictx).cell.cell, &raw const GRID_DEFAULT_CELL);
        (*ictx).cell.set = 0;
        (*ictx).cell.g0set = 0;
        (*ictx).cell.g1set = 0;

        memcpy__(&raw mut (*ictx).old_cell, &raw const (*ictx).cell);
        (*ictx).old_cx = 0;
        (*ictx).old_cy = 0;
    }
}

/// Save screen state.
unsafe fn input_save_state(ictx: *mut input_ctx) {
    unsafe {
        let sctx: *mut screen_write_ctx = &raw mut (*ictx).ctx;
        let s: *mut screen = (*sctx).s;

        memcpy__(&raw mut (*ictx).old_cell, &raw const (*ictx).cell);
        (*ictx).old_cx = (*s).cx;
        (*ictx).old_cy = (*s).cy;
        (*ictx).old_mode = (*s).mode;
    }
}

/// Restore screen state.
unsafe fn input_restore_state(ictx: *mut input_ctx) {
    unsafe {
        let sctx: *mut screen_write_ctx = &raw mut (*ictx).ctx;

        memcpy__(&raw mut (*ictx).cell, &raw const (*ictx).old_cell);
        if (*ictx).old_mode.intersects(mode_flag::MODE_ORIGIN) {
            screen_write_mode_set(sctx, mode_flag::MODE_ORIGIN);
        } else {
            screen_write_mode_clear(sctx, mode_flag::MODE_ORIGIN);
        }
        screen_write_cursormove(sctx, (*ictx).old_cx as i32, (*ictx).old_cy as i32, 0);
    }
}

/// Initialise input parser.
pub unsafe fn input_init(
    wp: *mut window_pane,
    bev: *mut bufferevent,
    palette: *mut colour_palette,
) -> *mut input_ctx {
    unsafe {
        let ictx = Box::into_raw(Box::new(input_ctx {
            wp,
            event: bev,
            palette,
            input_space: INPUT_BUF_START,
            input_buf: xmalloc(INPUT_BUF_START).as_ptr().cast(),
            since_ground: evbuffer_new(),
            ..zeroed()
        }));

        if (*ictx).since_ground.is_null() {
            fatalx("out of memory");
        }

        evtimer_set(
            &raw mut (*ictx).timer,
            input_timer_callback,
            NonNull::new(ictx).unwrap(),
        );

        input_reset(ictx, 0);
        ictx
    }
}

/// Destroy input parser.
pub unsafe fn input_free(ictx: *mut input_ctx) {
    unsafe {
        for i in 0..(*ictx).param_list_len {
            if (*ictx).param_list[i as usize].type_ == input_param_type::INPUT_STRING {
                free_((*ictx).param_list[i as usize].union_.str);
            }
        }

        event_del(&raw mut (*ictx).timer);

        free_((*ictx).input_buf);
        evbuffer_free((*ictx).since_ground);

        free_(ictx);
    }
}

// Reset input state and clear screen.

pub unsafe fn input_reset(ictx: *mut input_ctx, clear: i32) {
    unsafe {
        let sctx = &raw mut (*ictx).ctx;
        let wp = (*ictx).wp;

        input_reset_cell(ictx);

        if clear != 0 && !wp.is_null() {
            if tailq_empty(&raw const (*wp).modes) {
                screen_write_start_pane(sctx, wp, &raw mut (*wp).base);
            } else {
                screen_write_start(sctx, &raw mut (*wp).base);
            }
            screen_write_reset(sctx);
            screen_write_stop(sctx);
        }

        input_clear(ictx);

        (*ictx).state = &raw const INPUT_STATE_GROUND;
        (*ictx).flags = input_flags::empty();
    }
}

/// Return pending data.
pub unsafe fn input_pending(ictx: *mut input_ctx) -> *mut evbuffer {
    unsafe { (*ictx).since_ground }
}

pub unsafe fn input_set_state(ictx: *mut input_ctx, itr: *const input_transition) {
    unsafe {
        if let Some(exit) = (*(*ictx).state).exit {
            exit(ictx);
        }

        (*ictx).state = (*itr).state.map(|e| &raw const *e).unwrap_or_default();

        if let Some(enter) = (*(*ictx).state).enter {
            enter(ictx);
        }
    }
}

/// Parse data.
fn input_parse(ictx: *mut input_ctx, buf: *mut u8, len: usize) {
    unsafe {
        let sctx = &raw mut (*ictx).ctx;
        let mut state: *const input_state = null();
        let mut itr: Option<&input_transition> = None;
        let mut off = 0usize;

        // Parse the input.
        while off < len {
            (*ictx).ch = *buf.add(off) as i32;
            off += 1;

            // Find the transition.
            if (*ictx).state != state
                || itr.is_none()
                || (*ictx).ch < itr.as_ref().unwrap().first
                || (*ictx).ch > itr.as_ref().unwrap().last
            {
                itr = (*(*ictx).state)
                    .transitions
                    .as_ref()
                    .unwrap()
                    .iter()
                    .find(|&itr| (*ictx).ch >= itr.first && (*ictx).ch <= itr.last);

                if itr.is_none() {
                    fatalx("no transition from state");
                } /* No transition? Eh? */
            }
            state = (*ictx).state;

            // Any state except print stops the current collection. This is
            // an optimization to avoid checking if the attributes have
            // changed for every character. It will stop unnecessarily for
            // sequences that don't make a terminal change, but they should
            // be the minority.
            if itr.as_ref().unwrap().handler != Some(input_print) {
                screen_write_collect_end(sctx);
            }

            // Execute the handler, if any. Don't switch state if it
            // returns non-zero.
            if let Some(handler) = itr.as_ref().unwrap().handler
                && handler(ictx) != 0
            {
                continue;
            }

            // And switch state, if necessary.
            if itr.as_ref().unwrap().state.is_some() {
                input_set_state(ictx, itr.map(|e| &raw const *e).unwrap_or_default());
            }

            // If not in ground state, save input.
            if (*ictx).state != &raw const INPUT_STATE_GROUND {
                evbuffer_add((*ictx).since_ground, (&raw const (*ictx).ch).cast(), 1);
            }
        }
    }
}

/// Parse input from pane.
pub unsafe fn input_parse_pane(wp: *mut window_pane) {
    unsafe {
        let mut new_size: usize = 0;
        let new_data = window_pane_get_new_data(wp, &raw mut (*wp).offset, &raw mut new_size);
        input_parse_buffer(wp, new_data.cast(), new_size);
        window_pane_update_used_data(wp, &raw mut (*wp).offset, new_size);
    }
}

/// Parse given input.
pub unsafe fn input_parse_buffer(wp: *mut window_pane, buf: *mut u8, len: usize) {
    unsafe {
        let ictx = (*wp).ictx;
        let sctx = &raw mut (*ictx).ctx;
        if len == 0 {
            return;
        }

        window_update_activity(NonNull::new((*wp).window).unwrap());
        (*wp).flags |= window_pane_flags::PANE_CHANGED;

        // Flag new input while in a mode.
        if !tailq_empty(&raw const (*wp).modes) {
            (*wp).flags |= window_pane_flags::PANE_UNSEENCHANGES;
        }

        // NULL wp if there is a mode set as don't want to update the tty.
        if tailq_empty(&raw mut (*wp).modes) {
            screen_write_start_pane(sctx, wp, &raw mut (*wp).base);
        } else {
            screen_write_start(sctx, &raw mut (*wp).base);
        }

        // log_debug!("{}: %%{} {}, {} bytes: %.*s", "input_parse_buffer", (*wp).id, (*ictx).state).name, len, (int)len, buf);

        input_parse(ictx, buf, len);
        screen_write_stop(sctx);
    }
}

/// Parse given input for screen.
pub unsafe fn input_parse_screen(
    ictx: *mut input_ctx,
    s: *mut screen,
    cb: screen_write_init_ctx_cb,
    arg: *mut c_void,
    buf: *mut u8,
    len: usize,
) {
    unsafe {
        let sctx: *mut screen_write_ctx = &raw mut (*ictx).ctx;

        if len == 0 {
            return;
        }

        screen_write_start_callback(sctx, s, cb, arg);
        input_parse(ictx, buf, len);
        screen_write_stop(sctx);
    }
}

/// Split the parameter list (if any).
unsafe fn input_split(ictx: *mut input_ctx) -> i32 {
    unsafe {
        // const char *errstr;
        // char *ptr, *out;
        // struct input_param *ip;
        // u_int i;

        for i in 0..(*ictx).param_list_len {
            if (*ictx).param_list[i as usize].type_ == input_param_type::INPUT_STRING {
                free_((*ictx).param_list[i as usize].union_.str);
            }
        }
        (*ictx).param_list_len = 0;

        if (*ictx).param_len == 0 {
            return 0;
        }

        let mut ip = &raw mut (*ictx).param_list[0];

        let mut out;

        let mut ptr: *mut u8 = (&raw mut (*ictx).param_buf).cast();
        while {
            out = strsep(&raw mut ptr, c!(";"));
            !out.is_null()
        } {
            if *out == b'\0' {
                (*ip).type_ = input_param_type::INPUT_MISSING;
            } else if !libc::strchr(out, b':' as i32).is_null() {
                (*ip).type_ = input_param_type::INPUT_STRING;
                (*ip).union_.str = xstrdup(out).as_ptr();
            } else {
                (*ip).type_ = input_param_type::INPUT_NUMBER;
                (*ip).union_.num = match strtonum(out, 0, i32::MAX) {
                    Ok(n) => n,
                    Err(_errstr) => return -1,
                };
            }
            (*ictx).param_list_len += 1;
            ip = &raw mut (*ictx).param_list[(*ictx).param_list_len as usize];
            if (*ictx).param_list_len == (*ictx).param_list.len() as u32 {
                return -1;
            }
        }

        for i in 0..(*ictx).param_list_len {
            ip = &raw mut (*ictx).param_list[i as usize];
            match (*ip).type_ {
                input_param_type::INPUT_MISSING => {
                    log_debug!("parameter {}: missing", i);
                }
                input_param_type::INPUT_STRING => {
                    log_debug!("parameter {}: string {}", i, _s((*ip).union_.str));
                }
                input_param_type::INPUT_NUMBER => {
                    log_debug!("parameter {}: number {}", i, (*ip).union_.num);
                }
            }
        }

        0
    }
}

/// Get an argument or return default value.
pub unsafe fn input_get(ictx: *mut input_ctx, validx: u32, minval: i32, defval: i32) -> i32 {
    unsafe {
        if validx >= (*ictx).param_list_len {
            return defval;
        }
        let ip = &raw mut (*ictx).param_list[validx as usize];
        if (*ip).type_ == input_param_type::INPUT_MISSING {
            return defval;
        }
        if (*ip).type_ == input_param_type::INPUT_STRING {
            return -1;
        }
        (*ip).union_.num.max(minval)
    }
}

macro_rules! input_reply {
    ($ictx:expr, $($args:expr),* $(,)?) => {
        crate::input::input_reply_($ictx, format_args!($($args),*))
    };
}
/// Reply to terminal query.
unsafe fn input_reply_(ictx: *mut input_ctx, args: std::fmt::Arguments) {
    unsafe {
        let bev = (*ictx).event;
        if bev.is_null() {
            return;
        }

        let reply = CString::new(args.to_string()).unwrap();
        log_debug!("input_reply: {}", _s(reply.as_ptr()));
        bufferevent_write(bev, reply.as_ptr().cast(), strlen(reply.as_ptr().cast()));
    }
}

/// Clear saved state.
unsafe fn input_clear(ictx: *mut input_ctx) {
    unsafe {
        event_del(&raw mut (*ictx).timer);

        (*ictx).interm_buf[0] = b'\0';
        (*ictx).interm_len = 0;

        (*ictx).param_buf[0] = b'\0';
        (*ictx).param_len = 0;

        *(*ictx).input_buf = b'\0';
        (*ictx).input_len = 0;

        (*ictx).input_end = input_end_type::INPUT_END_ST;

        (*ictx).flags &= !input_flags::INPUT_DISCARD;
    }
}

/// Reset for ground state.
unsafe fn input_ground(ictx: *mut input_ctx) {
    unsafe {
        event_del(&raw mut (*ictx).timer);
        evbuffer_drain((*ictx).since_ground, EVBUFFER_LENGTH((*ictx).since_ground));

        if (*ictx).input_space > INPUT_BUF_START {
            (*ictx).input_space = INPUT_BUF_START;
            (*ictx).input_buf = xrealloc_((*ictx).input_buf, INPUT_BUF_START).as_ptr();
        }
    }
}

/// Output this character to the screen.
unsafe fn input_print(ictx: *mut input_ctx) -> i32 {
    unsafe {
        let sctx = &raw mut (*ictx).ctx;

        (*ictx).utf8started = 0; /* can't be valid UTF-8 */

        let set = if (*ictx).cell.set == 0 {
            (*ictx).cell.g0set
        } else {
            (*ictx).cell.g1set
        };
        if set == 1 {
            (*ictx).cell.cell.attr |= grid_attr::GRID_ATTR_CHARSET;
        } else {
            (*ictx).cell.cell.attr &= !grid_attr::GRID_ATTR_CHARSET;
        }

        utf8_set(&raw mut (*ictx).cell.cell.data, (*ictx).ch as u8);
        screen_write_collect_add(sctx, &(*ictx).cell.cell);

        utf8_copy(&raw mut (*ictx).last, &raw mut (*ictx).cell.cell.data);
        (*ictx).flags |= input_flags::INPUT_LAST;

        (*ictx).cell.cell.attr &= !grid_attr::GRID_ATTR_CHARSET;
    }

    0
}

/// Collect intermediate string.
unsafe fn input_intermediate(ictx: *mut input_ctx) -> i32 {
    let sizeof_interm_buf = 4;
    unsafe {
        if (*ictx).interm_len == sizeof_interm_buf - 1 {
            (*ictx).flags |= input_flags::INPUT_DISCARD;
        } else {
            (*ictx).interm_buf[(*ictx).interm_len] = (*ictx).ch as u8;
            (*ictx).interm_len += 1;
            (*ictx).interm_buf[(*ictx).interm_len] = b'\0';
        }
    }
    0
}

/// Collect parameter string.
unsafe fn input_parameter(ictx: *mut input_ctx) -> i32 {
    let sizeof_param_buf = 64;
    unsafe {
        if (*ictx).param_len == sizeof_param_buf - 1 {
            (*ictx).flags |= input_flags::INPUT_DISCARD;
        } else {
            (*ictx).param_buf[(*ictx).param_len] = (*ictx).ch as u8;
            (*ictx).param_len += 1;
            (*ictx).param_buf[(*ictx).param_len] = b'\0';
        }
    }

    0
}

/// Collect input string.
unsafe fn input_input(ictx: *mut input_ctx) -> i32 {
    unsafe {
        let mut available: usize = (*ictx).input_space;
        while (*ictx).input_len + 1 >= available {
            available *= 2;
            if available > INPUT_BUF_LIMIT {
                (*ictx).flags |= input_flags::INPUT_DISCARD;
                return 0;
            }
            (*ictx).input_buf = xrealloc_((*ictx).input_buf, available).as_ptr();
            (*ictx).input_space = available;
        }
        *(*ictx).input_buf.add((*ictx).input_len) = (*ictx).ch as u8;
        (*ictx).input_len += 1;
        *(*ictx).input_buf.add((*ictx).input_len) = b'\0';

        0
    }
}

/// Execute C0 control sequence.
unsafe fn input_c0_dispatch(ictx: *mut input_ctx) -> i32 {
    let func = "input_c0_dispatch";
    unsafe {
        let sctx = &raw mut (*ictx).ctx;
        let wp = (*ictx).wp;
        let s = (*sctx).s;

        (*ictx).utf8started = 0; /* can't be valid UTF-8 */

        log_debug!("{}: '{}'", "input_c0_dispatch", (*ictx).ch as u8 as char);

        const NUL: u8 = 0o00;
        const BEL: u8 = 0o07;
        const BS: u8 = 0o10;
        const HT: u8 = 0o11;

        const LF: u8 = 0o12;
        const VT: u8 = 0o13;
        const FF: u8 = 0o14;

        const CR: u8 = 0o15;
        const SO: u8 = 0o16;
        const SI: u8 = 0o17;

        match (*ictx).ch as u8 {
            NUL | BEL => {
                if !wp.is_null() {
                    alerts_queue(NonNull::new((*wp).window).unwrap(), window_flag::BELL);
                }
            }
            BS => screen_write_backspace(sctx),
            HT => {
                while (*s).cx < screen_size_x(s) - 1 {
                    // Don't tab beyond the end of the line.
                    // Find the next tab point, or use the last column if none.
                    (*s).cx += 1;
                    if bit_test((*s).tabs, (*s).cx) {
                        break;
                    }
                }
            }

            LF | VT | FF => {
                screen_write_linefeed(sctx, false, (*ictx).cell.cell.bg as u32);
                if (*s).mode.intersects(mode_flag::MODE_CRLF) {
                    screen_write_carriagereturn(sctx);
                }
            }
            CR => screen_write_carriagereturn(sctx),
            SO => (*ictx).cell.set = 1,
            SI => (*ictx).cell.set = 0,
            _ => log_debug!("{}: unknown '{}'", func, (*ictx).ch),
        }

        (*ictx).flags &= !input_flags::INPUT_LAST;
        0
    }
}

/// Execute escape sequence.
unsafe fn input_esc_dispatch(ictx: *mut input_ctx) -> i32 {
    let __func__ = "input_esc_dispatch";
    unsafe {
        let sctx = &raw mut (*ictx).ctx;
        let s = (*sctx).s;

        if (*ictx).flags.intersects(input_flags::INPUT_DISCARD) {
            return 0;
        }
        log_debug!(
            "{}: '{}', {}",
            __func__,
            (*ictx).ch as u8 as char,
            _s((*ictx).interm_buf.as_ptr().cast::<u8>())
        );

        let Ok(entry) =
            INPUT_ESC_TABLE.binary_search_by(|e| input_table_compare(ictx, e).reverse())
        else {
            log_debug!("{}: unknown '{}'", __func__, (*ictx).ch);
            return 0;
        };

        match input_esc_type::try_from((INPUT_ESC_TABLE[entry]).type_) {
            Ok(input_esc_type::INPUT_ESC_RIS) => {
                colour_palette_clear(NonNull::new((*ictx).palette).map(|e| &mut *e.as_ptr()));
                input_reset_cell(ictx);
                screen_write_reset(sctx);
                screen_write_fullredraw(sctx);
            }
            Ok(input_esc_type::INPUT_ESC_IND) => {
                screen_write_linefeed(sctx, false, (*ictx).cell.cell.bg as u32);
            }
            Ok(input_esc_type::INPUT_ESC_NEL) => {
                screen_write_carriagereturn(sctx);
                screen_write_linefeed(sctx, false, (*ictx).cell.cell.bg as u32);
            }
            Ok(input_esc_type::INPUT_ESC_HTS) => {
                if (*s).cx < screen_size_x(s) {
                    bit_set((*s).tabs, (*s).cx);
                }
            }
            Ok(input_esc_type::INPUT_ESC_RI) => {
                screen_write_reverseindex(sctx, (*ictx).cell.cell.bg as u32);
            }
            Ok(input_esc_type::INPUT_ESC_DECKPAM) => {
                screen_write_mode_set(sctx, mode_flag::MODE_KKEYPAD);
            }
            Ok(input_esc_type::INPUT_ESC_DECKPNM) => {
                screen_write_mode_clear(sctx, mode_flag::MODE_KKEYPAD);
            }
            Ok(input_esc_type::INPUT_ESC_DECSC) => input_save_state(ictx),
            Ok(input_esc_type::INPUT_ESC_DECRC) => input_restore_state(ictx),
            Ok(input_esc_type::INPUT_ESC_DECALN) => screen_write_alignmenttest(sctx),
            Ok(input_esc_type::INPUT_ESC_SCSG0_ON) => (*ictx).cell.g0set = 1,
            Ok(input_esc_type::INPUT_ESC_SCSG0_OFF) => (*ictx).cell.g0set = 0,
            Ok(input_esc_type::INPUT_ESC_SCSG1_ON) => (*ictx).cell.g1set = 1,
            Ok(input_esc_type::INPUT_ESC_SCSG1_OFF) => (*ictx).cell.g1set = 0,
            Ok(input_esc_type::INPUT_ESC_ST) => (),
            // ST terminates OSC but the state transition already did it.
            Err(_) => (),
        }

        (*ictx).flags &= !input_flags::INPUT_LAST;
        0
    }
}

/// Execute control sequence.
unsafe fn input_csi_dispatch(ictx: *mut input_ctx) -> i32 {
    let __func__ = "input_csi_dispatch";
    unsafe {
        let sctx = &raw mut (*ictx).ctx;
        let s = (*sctx).s;
        // int i, n, m, ek;
        let mut cx: u32;
        let bg: u32 = (*ictx).cell.cell.bg as u32;

        if (*ictx).flags.intersects(input_flags::INPUT_DISCARD) {
            return 0;
        }

        log_debug!(
            "{}: '{}' \"{}\" \"{}\"",
            __func__,
            (*ictx).ch as u8 as char,
            _s((&raw const (*ictx).interm_buf).cast::<u8>()),
            _s((&raw const (*ictx).param_buf).cast::<u8>())
        );

        if input_split(ictx) != 0 {
            return 0;
        }

        let Ok(entry) =
            INPUT_CSI_TABLE.binary_search_by(|e| input_table_compare(ictx.cast(), e).reverse())
        else {
            log_debug!("{}: unknown '{}'", __func__, (*ictx).ch);
            return 0;
        };

        match input_csi_type::try_from(INPUT_CSI_TABLE[entry].type_) {
            Ok(input_csi_type::INPUT_CSI_CBT) => {
                // Find the previous tab point, n times.
                cx = (*s).cx;
                if cx > screen_size_x(s) - 1 {
                    cx = screen_size_x(s) - 1;
                }
                let mut n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    while cx > 0 && n > 0 {
                        n -= 1;
                        loop {
                            cx -= 1;
                            if cx == 0 || bit_test((*s).tabs, cx) {
                                break;
                            }
                        }
                    }
                    (*s).cx = cx;
                }
            }
            Ok(input_csi_type::INPUT_CSI_CUB) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_cursorleft(sctx, n as u32);
                }
            }
            Ok(input_csi_type::INPUT_CSI_CUD) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_cursordown(sctx, n as u32);
                }
            }
            Ok(input_csi_type::INPUT_CSI_CUF) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_cursorright(sctx, n as u32);
                }
            }
            Ok(input_csi_type::INPUT_CSI_CUP) => {
                let n = input_get(ictx, 0, 1, 1);
                let m = input_get(ictx, 1, 1, 1);
                if n != -1 && m != -1 {
                    screen_write_cursormove(sctx, m - 1, n - 1, 1);
                }
            }
            Ok(input_csi_type::INPUT_CSI_MODSET) => {
                let n = input_get(ictx, 0, 0, 0);
                if n == 4 {
                    let m = input_get(ictx, 1, 0, 0);

                    // Set the extended key reporting mode as per the client
                    // request, unless "extended-keys" is set to "off".
                    let ek = options_get_number_(GLOBAL_OPTIONS, "extended-keys");
                    if ek != 0 {
                        screen_write_mode_clear(sctx, EXTENDED_KEY_MODES);
                        if m == 2 {
                            screen_write_mode_set(sctx, mode_flag::MODE_KEYS_EXTENDED_2);
                        } else if m == 1 || ek == 2 {
                            screen_write_mode_set(sctx, mode_flag::MODE_KEYS_EXTENDED);
                        }
                    }
                }
            }
            Ok(input_csi_type::INPUT_CSI_MODOFF) => {
                let n = input_get(ictx, 0, 0, 0);
                if n == 4 {
                    // Clear the extended key reporting mode as per the client
                    // request, unless "extended-keys always" forces into mode 1.
                    screen_write_mode_clear(
                        sctx,
                        mode_flag::MODE_KEYS_EXTENDED | mode_flag::MODE_KEYS_EXTENDED_2,
                    );
                    if options_get_number_(GLOBAL_OPTIONS, "extended-keys") == 2 {
                        screen_write_mode_set(sctx, mode_flag::MODE_KEYS_EXTENDED);
                    }
                }
            }
            Ok(input_csi_type::INPUT_CSI_WINOPS) => input_csi_dispatch_winops(ictx),
            Ok(input_csi_type::INPUT_CSI_CUU) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_cursorup(sctx, n as u32);
                }
            }
            Ok(input_csi_type::INPUT_CSI_CNL) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_carriagereturn(sctx);
                    screen_write_cursordown(sctx, n as u32);
                }
            }
            Ok(input_csi_type::INPUT_CSI_CPL) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_carriagereturn(sctx);
                    screen_write_cursorup(sctx, n as u32);
                }
            }
            Ok(input_csi_type::INPUT_CSI_DA) => match input_get(ictx, 0, 0, 0) {
                -1 => (),
                0 => {
                    #[cfg(feature = "sixel")]
                    {
                        input_reply!(ictx, "\x1b[?1;2;4c");
                    }
                    #[cfg(not(feature = "sixel"))]
                    {
                        input_reply!(ictx, "\x1b[?1;2c");
                    }
                }
                _ => log_debug!("{}: unknown '{}'", __func__, (*ictx).ch),
            },
            Ok(input_csi_type::INPUT_CSI_DA_TWO) => match input_get(ictx, 0, 0, 0) {
                -1 => (),
                0 => input_reply!(ictx, "\x1b[>84;0;0c"),
                _ => log_debug!("{}: unknown '{}'", __func__, (*ictx).ch as u8 as char),
            },
            Ok(input_csi_type::INPUT_CSI_ECH) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_clearcharacter(sctx, n as u32, bg);
                }
            }
            Ok(input_csi_type::INPUT_CSI_DCH) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_deletecharacter(sctx, n as u32, bg);
                }
            }
            Ok(input_csi_type::INPUT_CSI_DECSTBM) => {
                let n = input_get(ictx, 0, 1, 1);
                let m = input_get(ictx, 1, 1, screen_size_y(s) as i32);
                if n != -1 && m != -1 {
                    screen_write_scrollregion(sctx, (n - 1) as u32, (m - 1) as u32);
                }
            }
            Ok(input_csi_type::INPUT_CSI_DL) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_deleteline(sctx, n as u32, bg);
                }
            }
            Ok(input_csi_type::INPUT_CSI_DSR) => match input_get(ictx, 0, 0, 0) {
                -1 => (),
                5 => input_reply!(ictx, "\x1b[0n"),
                6 => input_reply!(ictx, "\x1b[{};{}R", (*s).cy + 1, (*s).cx + 1),
                _ => log_debug!("{}: unknown '{}'", __func__, (*ictx).ch as u8 as char),
            },
            Ok(input_csi_type::INPUT_CSI_ED) => {
                match input_get(ictx, 0, 0, 0) {
                    -1 => (),
                    0 => screen_write_clearendofscreen(sctx, bg),
                    1 => screen_write_clearstartofscreen(sctx, bg),
                    2 => screen_write_clearscreen(sctx, bg),
                    3 => {
                        if input_get(ictx, 1, 0, 0) == 0 {
                            // Linux console extension to clear history
                            // (for example before locking the screen).
                            screen_write_clearhistory(sctx);
                        }
                    }
                    _ => log_debug!("{}: unknown '{}'", __func__, (*ictx).ch as u8 as char),
                }
            }
            Ok(input_csi_type::INPUT_CSI_EL) => match input_get(ictx, 0, 0, 0) {
                -1 => (),
                0 => screen_write_clearendofline(sctx, bg),
                1 => screen_write_clearstartofline(sctx, bg),
                2 => screen_write_clearline(sctx, bg),
                _ => log_debug!("{}: unknown '{}'", __func__, (*ictx).ch as u8 as char),
            },
            Ok(input_csi_type::INPUT_CSI_HPA) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_cursormove(sctx, n - 1, -1, 1);
                }
            }
            Ok(input_csi_type::INPUT_CSI_ICH) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_insertcharacter(sctx, n as u32, bg);
                }
            }
            Ok(input_csi_type::INPUT_CSI_IL) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_insertline(sctx, n as u32, bg);
                }
            }
            Ok(input_csi_type::INPUT_CSI_REP) => {
                let mut n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    let m = screen_size_x(s) - (*s).cx;
                    if n as u32 > m {
                        n = m as i32;
                    }

                    if (*ictx).flags.intersects(input_flags::INPUT_LAST) {
                        utf8_copy(&raw mut (*ictx).cell.cell.data, &raw const (*ictx).last);
                        for _ in 0..n {
                            screen_write_collect_add(sctx, &raw const (*ictx).cell.cell);
                        }
                    }
                }
            }
            Ok(input_csi_type::INPUT_CSI_RCP) => input_restore_state(ictx),
            Ok(input_csi_type::INPUT_CSI_RM) => input_csi_dispatch_rm(ictx),
            Ok(input_csi_type::INPUT_CSI_RM_PRIVATE) => input_csi_dispatch_rm_private(ictx),
            Ok(input_csi_type::INPUT_CSI_SCP) => input_save_state(ictx),
            Ok(input_csi_type::INPUT_CSI_SGR) => input_csi_dispatch_sgr(ictx),
            Ok(input_csi_type::INPUT_CSI_SM) => input_csi_dispatch_sm(ictx),
            Ok(input_csi_type::INPUT_CSI_SM_PRIVATE) => input_csi_dispatch_sm_private(ictx),
            Ok(input_csi_type::INPUT_CSI_SM_GRAPHICS) => input_csi_dispatch_sm_graphics(ictx),
            Ok(input_csi_type::INPUT_CSI_SU) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_scrollup(sctx, n as u32, bg);
                }
            }
            Ok(input_csi_type::INPUT_CSI_SD) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_scrolldown(sctx, n as u32, bg);
                }
            }
            Ok(input_csi_type::INPUT_CSI_TBC) => match input_get(ictx, 0, 0, 0) {
                -1 => (),
                0 => {
                    if (*s).cx < screen_size_x(s) {
                        bit_clear((*s).tabs, (*s).cx);
                    }
                }
                3 => bit_nclear((*s).tabs, 0, screen_size_x(s) - 1),
                _ => log_debug!("{}: unknown '{}'", __func__, (*ictx).ch as u8 as char),
            },
            Ok(input_csi_type::INPUT_CSI_VPA) => {
                let n = input_get(ictx, 0, 1, 1);
                if n != -1 {
                    screen_write_cursormove(sctx, -1, n - 1, 1);
                }
            }
            Ok(input_csi_type::INPUT_CSI_DECSCUSR) => {
                let n = input_get(ictx, 0, 0, 0);
                if n != -1 {
                    screen_set_cursor_style(n as u32, &raw mut (*s).cstyle, &raw mut (*s).mode);
                }
            }
            Ok(input_csi_type::INPUT_CSI_XDA) => {
                if input_get(ictx, 0, 0, 0) == 0 {
                    input_reply!(ictx, "\x1bP>|tmux {}\x1b\\", getversion());
                }
            }
            Err(_) => (),
        }

        (*ictx).flags &= !input_flags::INPUT_LAST;
        0
    }
}

/// Handle CSI RM.
unsafe fn input_csi_dispatch_rm(ictx: *mut input_ctx) {
    unsafe {
        let sctx = &raw mut (*ictx).ctx;

        for i in 0..(*ictx).param_list_len {
            match input_get(ictx, i, 0, -1) {
                -1 => (),
                4 => screen_write_mode_clear(sctx, mode_flag::MODE_INSERT), // IRM
                34 => screen_write_mode_set(sctx, mode_flag::MODE_CURSOR_VERY_VISIBLE),
                _ => log_debug!(
                    "input_csi_dispatch_rm: unknown '{}'",
                    (*ictx).ch as u8 as char
                ),
            }
        }
    }
}

/// Handle CSI private RM.
unsafe fn input_csi_dispatch_rm_private(ictx: *mut input_ctx) {
    unsafe {
        let sctx = &raw mut (*ictx).ctx;
        let gc = &raw mut (*ictx).cell.cell;

        for i in 0..(*ictx).param_list_len {
            match input_get(ictx, i, 0, -1) {
                -1 => (),

                1 => screen_write_mode_clear(sctx, mode_flag::MODE_KCURSOR), // DECCKM
                3 => {
                    // DECCOLM
                    screen_write_cursormove(sctx, 0, 0, 1);
                    screen_write_clearscreen(sctx, (*gc).bg as u32);
                }
                6 => {
                    // DECOM
                    screen_write_mode_clear(sctx, mode_flag::MODE_ORIGIN);
                    screen_write_cursormove(sctx, 0, 0, 1);
                }
                7 => screen_write_mode_clear(sctx, mode_flag::MODE_WRAP), // DECAWM
                12 => {
                    screen_write_mode_clear(sctx, mode_flag::MODE_CURSOR_BLINKING);
                    screen_write_mode_set(sctx, mode_flag::MODE_CURSOR_BLINKING_SET);
                }
                25 => screen_write_mode_clear(sctx, mode_flag::MODE_CURSOR), // TCEM
                1000..=1003 => screen_write_mode_clear(sctx, ALL_MOUSE_MODES),
                1004 => screen_write_mode_clear(sctx, mode_flag::MODE_FOCUSON),
                1005 => screen_write_mode_clear(sctx, mode_flag::MODE_MOUSE_UTF8),
                1006 => screen_write_mode_clear(sctx, mode_flag::MODE_MOUSE_SGR),
                47 | 1047 => screen_write_alternateoff(sctx, gc, 0),
                1049 => screen_write_alternateoff(sctx, gc, 1),
                2004 => screen_write_mode_clear(sctx, mode_flag::MODE_BRACKETPASTE),
                _ => log_debug!(
                    "{}: unknown '{}'",
                    "input_csi_dispatch_rm_private",
                    (*ictx).ch as u8 as char
                ),
            }
        }
    }
}

/// Handle CSI SM.
unsafe fn input_csi_dispatch_sm(ictx: *mut input_ctx) {
    unsafe {
        let sctx = &raw mut (*ictx).ctx;

        for i in 0..(*ictx).param_list_len {
            match input_get(ictx, i, 0, -1) {
                -1 => (),
                4 => screen_write_mode_set(sctx, mode_flag::MODE_INSERT), // IRM
                34 => screen_write_mode_clear(sctx, mode_flag::MODE_CURSOR_VERY_VISIBLE),
                _ => log_debug!(
                    "{}: unknown '{}'",
                    "input_csi_dispatch_sm",
                    (*ictx).ch as u8 as char
                ),
            }
        }
    }
}

/// Handle CSI private SM.
unsafe fn input_csi_dispatch_sm_private(ictx: *mut input_ctx) {
    unsafe {
        let sctx = &raw mut (*ictx).ctx;
        let gc = &raw mut (*ictx).cell.cell;

        for i in 0..(*ictx).param_list_len {
            match input_get(ictx, i, 0, -1) {
                -1 => (),
                1 => screen_write_mode_set(sctx, mode_flag::MODE_KCURSOR), // DECCKM
                3 => {
                    // DECCOLM
                    screen_write_cursormove(sctx, 0, 0, 1);
                    screen_write_clearscreen(sctx, (*ictx).cell.cell.bg as u32);
                }
                6 => {
                    // DECOM
                    screen_write_mode_set(sctx, mode_flag::MODE_ORIGIN);
                    screen_write_cursormove(sctx, 0, 0, 1);
                }
                7 => screen_write_mode_set(sctx, mode_flag::MODE_WRAP), // DECAWM
                12 => {
                    screen_write_mode_set(sctx, mode_flag::MODE_CURSOR_BLINKING);
                    screen_write_mode_set(sctx, mode_flag::MODE_CURSOR_BLINKING_SET);
                }
                25 => screen_write_mode_set(sctx, mode_flag::MODE_CURSOR), // TCEM
                1000 => {
                    screen_write_mode_clear(sctx, ALL_MOUSE_MODES);
                    screen_write_mode_set(sctx, mode_flag::MODE_MOUSE_STANDARD);
                }
                1002 => {
                    screen_write_mode_clear(sctx, ALL_MOUSE_MODES);
                    screen_write_mode_set(sctx, mode_flag::MODE_MOUSE_BUTTON);
                }
                1003 => {
                    screen_write_mode_clear(sctx, ALL_MOUSE_MODES);
                    screen_write_mode_set(sctx, mode_flag::MODE_MOUSE_ALL);
                }
                1004 => screen_write_mode_set(sctx, mode_flag::MODE_FOCUSON),
                1005 => screen_write_mode_set(sctx, mode_flag::MODE_MOUSE_UTF8),
                1006 => screen_write_mode_set(sctx, mode_flag::MODE_MOUSE_SGR),
                47 | 1047 => screen_write_alternateon(sctx, gc, 0),
                1049 => screen_write_alternateon(sctx, gc, 1),
                2004 => screen_write_mode_set(sctx, mode_flag::MODE_BRACKETPASTE),
                _ => log_debug!(
                    "{}: unknown '{}'",
                    "input_csi_dispatch_sm_private",
                    (*ictx).ch as u8 as char
                ),
            }
        }
    }
}

/// Handle CSI graphics SM.
unsafe fn input_csi_dispatch_sm_graphics(ictx: *mut input_ctx) {
    #[cfg(feature = "sixel")]
    unsafe {
        use crate::image_sixel::SIXEL_COLOUR_REGISTERS;

        if (*ictx).param_list_len > 3 {
            return;
        }
        let n = input_get(ictx, 0, 0, 0);
        let m = input_get(ictx, 1, 0, 0);
        let o = input_get(ictx, 2, 0, 0);

        if n == 1 && (m == 1 || m == 2 || m == 4) {
            input_reply!(ictx, "\x1b[?{n};0;{SIXEL_COLOUR_REGISTERS}S");
        } else {
            input_reply!(ictx, "\x1b[?{n};3;{o}S");
        }
    }
}

/// Handle CSI window operations.
unsafe fn input_csi_dispatch_winops(ictx: *mut input_ctx) {
    unsafe {
        let sctx = &(*ictx).ctx;
        let s = sctx.s;
        let wp = (*ictx).wp;
        let mut w: *mut window = null_mut();
        let x: u32 = screen_size_x(s);
        let y: u32 = screen_size_y(s);

        if !wp.is_null() {
            w = (*wp).window;
        }

        let mut n: i32;
        let mut m: i32 = 0;
        while {
            n = input_get(ictx, m as u32, 0, -1);
            n != -1
        } {
            match n {
                1 | 2 | 5 | 6 | 7 | 11 | 13 | 20 | 21 | 24 => (),
                3 | 4 | 8 => {
                    m += 1;
                    if input_get(ictx, m as u32, 0, -1) == -1 {
                        return;
                    }
                    // FALLTHROUGH
                    m += 1;
                    if input_get(ictx, m as u32, 0, -1) == -1 {
                        return;
                    }
                }
                9 | 10 => {
                    m += 1;
                    if input_get(ictx, m as u32, 0, -1) == -1 {
                        return;
                    }
                }
                14 => {
                    if !w.is_null() {
                        input_reply!(ictx, "\x1b[4;{};{}t", y * (*w).ypixel, x * (*w).xpixel);
                    }
                }
                15 => {
                    if !w.is_null() {
                        input_reply!(ictx, "\x1b[5;{};{}t", y * (*w).ypixel, x * (*w).xpixel,);
                    }
                }
                16 => {
                    if !w.is_null() {
                        input_reply!(ictx, "\x1b[6;{};{}t", (*w).ypixel, (*w).xpixel);
                    }
                }
                18 => input_reply!(ictx, "\x1b[8;{};{}t", y, x),
                19 => input_reply!(ictx, "\x1b[9;{};{}t", y, x),
                22 => {
                    m += 1;
                    match input_get(ictx, m as u32, 0, -1) {
                        -1 => return,
                        0 | 2 => screen_push_title(sctx.s),
                        _ => (),
                    }
                }
                23 => {
                    m += 1;
                    match input_get(ictx, m as u32, 0, -1) {
                        -1 => return,
                        0 | 2 => {
                            screen_pop_title(sctx.s);
                            if !wp.is_null() {
                                notify_pane("pane-title-changed", wp);
                                server_redraw_window_borders(w);
                                server_status_window(w);
                            }
                        }
                        _ => (),
                    }
                }
                _ => log_debug!(
                    "{}: unknown '{}'",
                    "input_csi_dispatch_winops",
                    (*ictx).ch as u8 as char
                ),
            }
            m += 1;
        }
    }
}

/// Helper for 256 colour SGR.
unsafe fn input_csi_dispatch_sgr_256_do(ictx: *mut input_ctx, fgbg: i32, c: i32) -> i32 {
    unsafe {
        let gc = &raw mut (*ictx).cell.cell;

        if c == -1 || c > 255 {
            match fgbg {
                38 => (*gc).fg = 8,
                48 => (*gc).bg = 8,
                _ => (),
            }
        } else {
            match fgbg {
                38 => (*gc).fg = c | COLOUR_FLAG_256,
                48 => (*gc).bg = c | COLOUR_FLAG_256,
                58 => (*gc).us = c | COLOUR_FLAG_256,
                _ => (),
            }
        }

        1
    }
}

/// Handle CSI SGR for 256 colours.
unsafe fn input_csi_dispatch_sgr_256(ictx: *mut input_ctx, fgbg: i32, i: *mut u32) {
    unsafe {
        let c = input_get(ictx, (*i) + 1, 0, -1);
        if input_csi_dispatch_sgr_256_do(ictx, fgbg, c) != 0 {
            (*i) += 1;
        }
    }
}

/// Helper for RGB colour SGR.
unsafe fn input_csi_dispatch_sgr_rgb_do(
    ictx: *mut input_ctx,
    fgbg: i32,
    r: i32,
    g: i32,
    b: i32,
) -> i32 {
    unsafe {
        let gc = &raw mut (*ictx).cell.cell;

        if r == -1 || r > 255 || g == -1 || g > 255 || b == -1 || b > 255 {
            return 0;
        }

        match fgbg {
            38 => (*gc).fg = colour_join_rgb(r as u8, g as u8, b as u8),
            48 => (*gc).bg = colour_join_rgb(r as u8, g as u8, b as u8),
            58 => (*gc).us = colour_join_rgb(r as u8, g as u8, b as u8),
            _ => (),
        }

        1
    }
}

/// Handle CSI SGR for RGB colours.
unsafe fn input_csi_dispatch_sgr_rgb(ictx: *mut input_ctx, fgbg: i32, i: *mut u32) {
    unsafe {
        let r = input_get(ictx, (*i) + 1, 0, -1);
        let g = input_get(ictx, (*i) + 2, 0, -1);
        let b = input_get(ictx, (*i) + 3, 0, -1);
        if input_csi_dispatch_sgr_rgb_do(ictx, fgbg, r, g, b) != 0 {
            (*i) += 3;
        }
    }
}

/// Handle CSI SGR with a ISO parameter.
unsafe fn input_csi_dispatch_sgr_colon(ictx: *mut input_ctx, mut i: u32) {
    let __func__ = "input_csi_dispatch_sgr_colon";
    unsafe {
        let gc = &raw mut (*ictx).cell.cell;
        let s = (*ictx).param_list[i as usize].union_.str;

        let mut n = 0;
        let mut p: [i32; 8] = [-1; 8];

        let mut ptr = xstrdup(s).as_ptr();
        let copy = ptr;
        let mut out: *mut u8;
        while {
            out = strsep(&raw mut ptr, c!(":"));
            !out.is_null()
        } {
            if *out != b'\0' {
                match strtonum(out, 0, i32::MAX) {
                    Ok(x) => {
                        p[n] = x;
                        n += 1;
                    }
                    Err(_) => {
                        free_(copy);
                        return;
                    }
                }

                if n == p.len() {
                    free_(copy);
                    return;
                }
            } else {
                n += 1;
                if n == p.len() {
                    free_(copy);
                    return;
                }
            }
            log_debug!("{}: {} = {}", __func__, n - 1, p[n - 1]);
        }
        free_(copy);

        if n == 0 {
            return;
        }
        if p[0] == 4 {
            if n != 2 {
                return;
            }
            match p[1] {
                0 => (*gc).attr &= !GRID_ATTR_ALL_UNDERSCORE,
                1 => {
                    (*gc).attr &= !GRID_ATTR_ALL_UNDERSCORE;
                    (*gc).attr |= grid_attr::GRID_ATTR_UNDERSCORE;
                }
                2 => {
                    (*gc).attr &= !GRID_ATTR_ALL_UNDERSCORE;
                    (*gc).attr |= grid_attr::GRID_ATTR_UNDERSCORE_2;
                }
                3 => {
                    (*gc).attr &= !GRID_ATTR_ALL_UNDERSCORE;
                    (*gc).attr |= grid_attr::GRID_ATTR_UNDERSCORE_3;
                }
                4 => {
                    (*gc).attr &= !GRID_ATTR_ALL_UNDERSCORE;
                    (*gc).attr |= grid_attr::GRID_ATTR_UNDERSCORE_4;
                }
                5 => {
                    (*gc).attr &= !GRID_ATTR_ALL_UNDERSCORE;
                    (*gc).attr |= grid_attr::GRID_ATTR_UNDERSCORE_5;
                }
                _ => (),
            }
            return;
        }
        if n < 2 || (p[0] != 38 && p[0] != 48 && p[0] != 58) {
            return;
        }
        match p[1] {
            2 => {
                if n >= 3 {
                    if n == 5 {
                        i = 2;
                    } else {
                        i = 3;
                    }
                    if n >= i as usize + 3 {
                        input_csi_dispatch_sgr_rgb_do(
                            ictx,
                            p[0],
                            p[i as usize],
                            p[i as usize + 1],
                            p[i as usize + 2],
                        );
                    }
                }
            }
            5 => {
                if n >= 3 {
                    input_csi_dispatch_sgr_256_do(ictx, p[0], p[2]);
                }
            }
            _ => (),
        }
    }
}

/// Handle CSI SGR.
unsafe fn input_csi_dispatch_sgr(ictx: *mut input_ctx) {
    unsafe {
        let gc = &raw mut (*ictx).cell.cell;

        if (*ictx).param_list_len == 0 {
            memcpy__(gc, &raw const GRID_DEFAULT_CELL);
            return;
        }

        let mut i: u32 = 0;
        while i < (*ictx).param_list_len {
            if (*ictx).param_list[i as usize].type_ == input_param_type::INPUT_STRING {
                input_csi_dispatch_sgr_colon(ictx, i);
                i += 1;
                continue;
            }
            let n = input_get(ictx, i, 0, 0);
            if n == -1 {
                i += 1;
                continue;
            }

            if n == 38 || n == 48 || n == 58 {
                i += 1;
                match input_get(ictx, i, 0, -1) {
                    2 => input_csi_dispatch_sgr_rgb(ictx, n, &raw mut i),
                    5 => input_csi_dispatch_sgr_256(ictx, n, &raw mut i),
                    _ => (),
                }
                i += 1;
                continue;
            }

            match n {
                0 => {
                    let link = (*gc).link;
                    memcpy__(gc, &raw const GRID_DEFAULT_CELL);
                    (*gc).link = link;
                }
                1 => (*gc).attr |= grid_attr::GRID_ATTR_BRIGHT,
                2 => (*gc).attr |= grid_attr::GRID_ATTR_DIM,
                3 => (*gc).attr |= grid_attr::GRID_ATTR_ITALICS,
                4 => {
                    (*gc).attr &= !GRID_ATTR_ALL_UNDERSCORE;
                    (*gc).attr |= grid_attr::GRID_ATTR_UNDERSCORE;
                }
                5 | 6 => (*gc).attr |= grid_attr::GRID_ATTR_BLINK,
                7 => (*gc).attr |= grid_attr::GRID_ATTR_REVERSE,
                8 => (*gc).attr |= grid_attr::GRID_ATTR_HIDDEN,
                9 => (*gc).attr |= grid_attr::GRID_ATTR_STRIKETHROUGH,
                21 => {
                    (*gc).attr &= !GRID_ATTR_ALL_UNDERSCORE;
                    (*gc).attr |= grid_attr::GRID_ATTR_UNDERSCORE_2;
                }
                22 => (*gc).attr &= !(grid_attr::GRID_ATTR_BRIGHT | grid_attr::GRID_ATTR_DIM),
                23 => (*gc).attr &= !grid_attr::GRID_ATTR_ITALICS,
                24 => (*gc).attr &= !GRID_ATTR_ALL_UNDERSCORE,
                25 => (*gc).attr &= !grid_attr::GRID_ATTR_BLINK,
                27 => (*gc).attr &= !grid_attr::GRID_ATTR_REVERSE,
                28 => (*gc).attr &= !grid_attr::GRID_ATTR_HIDDEN,
                29 => (*gc).attr &= !grid_attr::GRID_ATTR_STRIKETHROUGH,
                30..=37 => (*gc).fg = n - 30,
                39 => (*gc).fg = 8,
                40..=47 => (*gc).bg = n - 40,
                49 => (*gc).bg = 8,
                53 => (*gc).attr |= grid_attr::GRID_ATTR_OVERLINE,
                55 => (*gc).attr &= !grid_attr::GRID_ATTR_OVERLINE,
                59 => (*gc).us = 8,
                90..=97 => (*gc).fg = n,
                100..=107 => (*gc).bg = n - 10,
                _ => (),
            }
            i += 1;
        }
    }
}

/// End of input with BEL.
unsafe fn input_end_bel(ictx: *mut input_ctx) -> i32 {
    log_debug!("input_end_bel");

    unsafe {
        (*ictx).input_end = input_end_type::INPUT_END_BEL;
    }

    0
}

/// DCS string started.
unsafe fn input_enter_dcs(ictx: *mut input_ctx) {
    unsafe {
        log_debug!("input_enter_dcs");

        input_clear(ictx);
        input_start_timer(ictx);
        (*ictx).flags &= !input_flags::INPUT_LAST;
    }
}

/// DCS terminator (ST) received.
unsafe fn input_dcs_dispatch(ictx: *mut input_ctx) -> i32 {
    unsafe {
        let func = "input_dcs_dispatch";

        let wp = (*ictx).wp;
        let sctx = &raw mut (*ictx).ctx;
        let buf = (*ictx).input_buf;
        let len = (*ictx).input_len;

        let prefix = c"tmux;";
        let prefixlen: u32 = 5;

        if wp.is_null() {
            return 0;
        }

        if (*ictx).flags.intersects(input_flags::INPUT_DISCARD) {
            log_debug!("{}: {} bytes (discard)", func, len);
            return 0;
        }

        #[cfg(feature = "sixel")]
        {
            use crate::image_sixel::sixel_parse;
            use crate::screen_write::screen_write_sixelimage;

            let w = (*wp).window;
            if *buf == b'q'
                && let Some(si) = NonNull::new(sixel_parse(buf, len, (*w).xpixel, (*w).ypixel))
            {
                screen_write_sixelimage(sctx, si.as_ptr(), (*ictx).cell.cell.bg as _);
            }
        }

        let allow_passthrough = options_get_number_((*wp).options, "allow-passthrough");
        if allow_passthrough == 0 {
            return 0;
        }
        log_debug!("{}: \"{}\"", func, _s(buf.cast::<u8>()));

        if len >= prefixlen as usize
            && libc::strncmp(buf.cast(), prefix.as_ptr().cast(), prefixlen as usize) == 0
        {
            screen_write_rawstring(
                sctx,
                buf.add(prefixlen as usize),
                len as u32 - prefixlen,
                (allow_passthrough == 2) as i32,
            );
        }

        0
    }
}

/// OSC string started.
unsafe fn input_enter_osc(ictx: *mut input_ctx) {
    unsafe {
        log_debug!("input_enter_osc");

        input_clear(ictx);
        input_start_timer(ictx);
        (*ictx).flags &= !input_flags::INPUT_LAST;
    }
}

/// OSC terminator (ST) received.
unsafe fn input_exit_osc(ictx: *mut input_ctx) {
    unsafe {
        let sctx = &raw mut (*ictx).ctx;
        let wp = (*ictx).wp;
        let mut p = (*ictx).input_buf;

        if (*ictx).flags.intersects(input_flags::INPUT_DISCARD) {
            return;
        }
        if (*ictx).input_len < 1 || *p < b'0' || *p > b'9' {
            return;
        }

        log_debug!(
            "{}: \"{}\" (end {})",
            "input_exit_osc",
            _s(p.cast::<u8>()),
            if (*ictx).input_end == input_end_type::INPUT_END_ST {
                "ST"
            } else {
                "BEL"
            }
        );

        let mut option = 0;
        while *p >= b'0' && *p <= b'9' {
            option = option * 10 + *p - b'0';
            p = p.add(1);
        }
        if *p != b';' && *p != b'\0' {
            return;
        }
        if *p == b';' {
            p = p.add(1);
        }

        match option {
            0 | 2 => {
                if !wp.is_null()
                    && options_get_number_((*wp).options, "allow-set-title") != 0
                    && screen_set_title((*sctx).s, p.cast()) != 0
                {
                    notify_pane("pane-title-changed", wp);
                    server_redraw_window_borders((*wp).window);
                    server_status_window((*wp).window);
                }
            }
            4 => input_osc_4(ictx, p.cast()),
            7 => {
                if utf8_isvalid(p.cast()) {
                    screen_set_path((*sctx).s, p.cast());
                    if !wp.is_null() {
                        server_redraw_window_borders((*wp).window);
                        server_status_window((*wp).window);
                    }
                }
            }
            8 => input_osc_8(ictx, p.cast()),
            10 => input_osc_10(ictx, p.cast()),
            11 => input_osc_11(ictx, p.cast()),
            12 => input_osc_12(ictx, p.cast()),
            52 => input_osc_52(ictx, p.cast()),
            104 => input_osc_104(ictx, p.cast()),
            110 => input_osc_110(ictx, p.cast()),
            111 => input_osc_111(ictx, p.cast()),
            112 => input_osc_112(ictx, p.cast()),
            133 => input_osc_133(ictx, p.cast()),
            _ => log_debug!("{}: unknown '{}'", "input_exit_osc", option),
        };
    }
}

/// APC string started.
unsafe fn input_enter_apc(ictx: *mut input_ctx) {
    unsafe {
        log_debug!("input_enter_apc");

        input_clear(ictx);
        input_start_timer(ictx);
        (*ictx).flags &= !input_flags::INPUT_LAST;
    }
}

/// APC terminator (ST) received.
unsafe fn input_exit_apc(ictx: *mut input_ctx) {
    unsafe {
        let sctx = &raw mut (*ictx).ctx;
        let wp = (*ictx).wp;

        if (*ictx).flags.intersects(input_flags::INPUT_DISCARD) {
            return;
        }
        log_debug!("input_exit_apc: \"{}\"", _s((*ictx).input_buf.cast::<u8>()));

        if screen_set_title((*sctx).s, (*ictx).input_buf.cast()) != 0 && !wp.is_null() {
            notify_pane("pane-title-changed", wp);
            server_redraw_window_borders((*wp).window);
            server_status_window((*wp).window);
        }
    }
}

/// Rename string started.
unsafe fn input_enter_rename(ictx: *mut input_ctx) {
    unsafe {
        log_debug!("input_enter_rename");

        input_clear(ictx);
        input_start_timer(ictx);
        (*ictx).flags &= !input_flags::INPUT_LAST;
    }
}

/// Rename terminator (ST) received.
unsafe fn input_exit_rename(ictx: *mut input_ctx) {
    unsafe {
        let wp = (*ictx).wp;

        if wp.is_null() {
            return;
        }
        if (*ictx).flags.intersects(input_flags::INPUT_DISCARD) {
            return;
        }
        if options_get_number_((*(*ictx).wp).options, "allow-rename") == 0 {
            return;
        }
        log_debug!(
            "{}: \"{}\"",
            "input_exit_rename",
            _s((*ictx).input_buf.cast::<u8>())
        );

        if !utf8_isvalid((*ictx).input_buf.cast()) {
            return;
        }
        let w = (*wp).window;

        if (*ictx).input_len == 0 {
            if let Some(o) = NonNull::new(options_get_only((*w).options, c!("automatic-rename"))) {
                _ = options_remove_or_default(o.as_ptr(), -1);
            }
            if options_get_number_((*w).options, "automatic-rename") == 0 {
                window_set_name(w, c!(""));
            }
        } else {
            options_set_number((*w).options, c!("automatic-rename"), 0);
            window_set_name(w, (*ictx).input_buf.cast());
        }
        server_redraw_window_borders(w);
        server_status_window(w);
    }
}

/// Open UTF-8 character.
unsafe fn input_top_bit_set(ictx: *mut input_ctx) -> i32 {
    let __func__ = "input_top_bit_set";
    unsafe {
        let sctx = &raw mut (*ictx).ctx;
        let ud = &raw mut (*ictx).utf8data;

        (*ictx).flags &= !input_flags::INPUT_LAST;

        if (*ictx).utf8started == 0 {
            if utf8_open(ud, (*ictx).ch as u8) != utf8_state::UTF8_MORE {
                return 0;
            }
            (*ictx).utf8started = 1;
            return 0;
        }

        match utf8_append(ud, (*ictx).ch as u8) {
            utf8_state::UTF8_MORE => return 0,
            utf8_state::UTF8_ERROR => {
                (*ictx).utf8started = 0;
                return 0;
            }
            utf8_state::UTF8_DONE => (),
        }
        (*ictx).utf8started = 0;

        // log_debug!("{} {} '%*s' (width {})", __func__, (*ud).size, (int)(*ud).size, (*ud).data, (*ud).width);

        utf8_copy(&raw mut (*ictx).cell.cell.data, ud);
        screen_write_collect_add(sctx, &raw mut (*ictx).cell.cell);

        utf8_copy(&raw mut (*ictx).last, &raw mut (*ictx).cell.cell.data);
        (*ictx).flags |= input_flags::INPUT_LAST;

        0
    }
}

/// Reply to a colour request.
unsafe fn input_osc_colour_reply(ictx: *mut input_ctx, n: u32, mut c: i32) {
    unsafe {
        if c != -1 {
            c = colour_force_rgb(c);
        }
        if c == -1 {
            return;
        }
        let (r, g, b) = colour_split_rgb(c);

        let end = if (*ictx).input_end == input_end_type::INPUT_END_BEL {
            c!("\x07")
        } else {
            c!("\x1b\\")
        };

        input_reply!(
            ictx,
            "\x1b]{};rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}{}",
            n,
            r,
            r,
            g,
            g,
            b,
            b,
            _s(end),
        );
    }
}

/// Handle the OSC 4 sequence for setting (multiple) palette entries.
unsafe fn input_osc_4(ictx: *mut input_ctx, p: *mut u8) {
    unsafe {
        // char *copy, *s, *next = NULL;
        // long idx;
        // int c, bad = 0, redraw = 0;
        let mut c;
        let mut next = null_mut();
        let mut bad = false;
        let mut redraw = false;

        let mut s: *mut u8 = xstrdup(p).as_ptr();
        let copy = s;
        while !s.is_null() && *s != b'\0' {
            let idx = strtol(s, &raw mut next, 10);

            let tmp = *next;
            next = next.add(1);
            if tmp != b';' {
                bad = true;
                break;
            }
            if !(0..256).contains(&idx) {
                bad = true;
                break;
            }

            s = strsep(&raw mut next, c!(";"));
            if streq_(s, "?") {
                c = colour_palette_get(ptr_to_ref((*ictx).palette), idx as i32);
                if c != -1 {
                    input_osc_colour_reply(ictx, 4, c);
                }
                continue;
            }
            c = colour_parse_x11(s);
            if c == -1 {
                s = next;
                continue;
            }
            if colour_palette_set(ptr_to_mut_ref((*ictx).palette), idx as i32, c) != 0 {
                redraw = true;
            }
            s = next;
        }
        if bad {
            log_debug!("bad OSC 4: {}", _s(p));
        }
        if redraw {
            screen_write_fullredraw(&raw mut (*ictx).ctx);
        }
        free_(copy);
    }
}

/// Handle the OSC 8 sequence for embedding hyperlinks.
unsafe fn input_osc_8(ictx: *mut input_ctx, p: *mut u8) {
    unsafe {
        let hl: *mut hyperlinks = (*(*ictx).ctx.s).hyperlinks;
        let gc = &raw mut (*ictx).cell.cell;

        let mut id: *mut u8 = null_mut();

        'bad: {
            let mut start = p;
            let mut end: *mut u8;
            while {
                end = strpbrk(start, c!(":;"));
                !end.is_null()
            } {
                if end.offset_from_unsigned(start) >= 4 && libc::strncmp(start, c!("id="), 3) == 0 {
                    if !id.is_null() {
                        break 'bad;
                    }
                    id = xstrndup(start.add(3), end.offset_from_unsigned(start) - 3).as_ptr();
                }

                // The first ; is the end of parameters and start of the URI.
                if *end == b';' {
                    break;
                }
                start = end.add(1);
            }
            if end.is_null() || *end != b';' {
                break 'bad;
            }
            let uri = end.add(1);
            if *uri == b'\0' {
                (*gc).link = 0;
                free_(id);
                return;
            }
            (*gc).link = hyperlinks_put(hl, uri, id);
            if id.is_null() {
                log_debug!("hyperlink (anonymous) {} = {}", _s(uri), (*gc).link);
            } else {
                log_debug!("hyperlink (id={}) {} = {}", _s(id), _s(uri), (*gc).link);
            }
            free_(id);
            return;
        }
        // bad:
        log_debug!("bad OSC 8 {}", _s(p.cast::<u8>()));
        free_(id);
    }
}

/// Get a client with a foreground for the pane.
/// There isn't much to choose between them so just use the first.
unsafe fn input_get_fg_client(wp: *mut window_pane) -> i32 {
    unsafe {
        let w = (*wp).window;
        for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*loop_).flags.intersects(CLIENT_UNATTACHEDFLAGS) {
                continue;
            }
            if (*loop_).session.is_null() || session_has((*loop_).session, w) == 0 {
                continue;
            }
            if (*loop_).tty.fg == -1 {
                continue;
            }
            return (*loop_).tty.fg;
        }

        -1
    }
}

/// Get a client with a background for the pane.
unsafe fn input_get_bg_client(wp: *mut window_pane) -> i32 {
    unsafe {
        let w = (*wp).window;

        for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if (*loop_).flags.intersects(CLIENT_UNATTACHEDFLAGS) {
                continue;
            }
            if (*loop_).session.is_null() || session_has((*loop_).session, w) == 0 {
                continue;
            }
            if (*loop_).tty.bg == -1 {
                continue;
            }
            return (*loop_).tty.bg;
        }
        -1
    }
}

// If any control mode client exists that has provided a bg color, return it.
// Otherwise, return -1.

unsafe fn input_get_bg_control_client(wp: *mut window_pane) -> i32 {
    unsafe {
        if (*wp).control_bg == -1 {
            return -1;
        }

        if tailq_foreach(&raw mut CLIENTS)
            .any(|c| (*c.as_ptr()).flags.intersects(client_flag::CONTROL))
        {
            return (*wp).control_bg;
        }
    }

    -1
}

// If any control mode client exists that has provided a fg color, return it.
// Otherwise, return -1.

unsafe fn input_get_fg_control_client(wp: *mut window_pane) -> i32 {
    unsafe {
        if (*wp).control_fg == -1 {
            return -1;
        }

        if tailq_foreach(&raw mut CLIENTS)
            .any(|c| (*c.as_ptr()).flags.intersects(client_flag::CONTROL))
        {
            return (*wp).control_fg;
        }
    }
    -1
}

// Handle the OSC 10 sequence for setting and querying foreground colour.

unsafe fn input_osc_10(ictx: *mut input_ctx, p: *mut u8) {
    unsafe {
        let wp = (*ictx).wp;
        let mut defaults: grid_cell = zeroed();
        let mut c;

        if streq_(p, "?") {
            if wp.is_null() {
                return;
            }
            c = input_get_fg_control_client(wp);
            if c == -1 {
                tty_default_colours(&raw mut defaults, wp);
                if COLOUR_DEFAULT(defaults.fg) {
                    c = input_get_fg_client(wp);
                } else {
                    c = defaults.fg;
                }
            }
            input_osc_colour_reply(ictx, 10, c);
            return;
        }

        c = colour_parse_x11(p);
        if c == -1 {
            log_debug!("bad OSC 10: {}", _s(p));
            return;
        }
        if !(*ictx).palette.is_null() {
            (*(*ictx).palette).fg = c;
            if wp.is_null() {
                (*wp).flags |= window_pane_flags::PANE_STYLECHANGED;
            }
            screen_write_fullredraw(&raw mut (*ictx).ctx);
        }
    }
}

// Handle the OSC 110 sequence for resetting foreground colour.

unsafe fn input_osc_110(ictx: *mut input_ctx, p: *const u8) {
    unsafe {
        let wp = (*ictx).wp;

        if *p != b'\0' {
            return;
        }

        if !(*ictx).palette.is_null() {
            (*(*ictx).palette).fg = 8;
            if !wp.is_null() {
                (*wp).flags |= window_pane_flags::PANE_STYLECHANGED;
            }
            screen_write_fullredraw(&raw mut (*ictx).ctx);
        }
    }
}

/// Handle the OSC 11 sequence for setting and querying background colour.
unsafe fn input_osc_11(ictx: *mut input_ctx, p: *const u8) {
    unsafe {
        let wp = (*ictx).wp;
        let mut defaults: grid_cell = zeroed();

        let mut c;

        if streq_(p, "?") {
            if wp.is_null() {
                return;
            }
            c = input_get_bg_control_client(wp);
            if c == -1 {
                tty_default_colours(&raw mut defaults, wp);
                if COLOUR_DEFAULT(defaults.bg) {
                    c = input_get_bg_client(wp);
                } else {
                    c = defaults.bg;
                }
            }
            input_osc_colour_reply(ictx, 11, c);
            return;
        }

        c = colour_parse_x11(p);
        if c == -1 {
            log_debug!("bad OSC 11: {}", _s(p));
            return;
        }
        if !(*ictx).palette.is_null() {
            (*(*ictx).palette).bg = c;
            if !wp.is_null() {
                (*wp).flags |= window_pane_flags::PANE_STYLECHANGED;
            }
            screen_write_fullredraw(&raw mut (*ictx).ctx);
        }
    }
}

/// Handle the OSC 111 sequence for resetting background colour.
unsafe fn input_osc_111(ictx: *mut input_ctx, p: *mut u8) {
    unsafe {
        let wp = (*ictx).wp;

        if *p != b'\0' {
            return;
        }
        if !(*ictx).palette.is_null() {
            (*(*ictx).palette).bg = 8;
            if !wp.is_null() {
                (*wp).flags |= window_pane_flags::PANE_STYLECHANGED;
            }
            screen_write_fullredraw(&raw mut (*ictx).ctx);
        }
    }
}

/// Handle the OSC 12 sequence for setting and querying cursor colour.
unsafe fn input_osc_12(ictx: *mut input_ctx, p: *const u8) {
    unsafe {
        let wp = (*ictx).wp;
        let mut c;

        if streq_(p, "?") {
            if !wp.is_null() {
                c = (*(*ictx).ctx.s).ccolour;
                if c == -1 {
                    c = (*(*ictx).ctx.s).default_ccolour;
                }
                input_osc_colour_reply(ictx, 12, c);
            }
            return;
        }

        c = colour_parse_x11(p);
        if c == -1 {
            log_debug!("bad OSC 12: {}", _s(p));
            return;
        }
        screen_set_cursor_colour((*ictx).ctx.s, c);
    }
}

/// Handle the OSC 112 sequence for resetting cursor colour.
unsafe fn input_osc_112(ictx: *mut input_ctx, p: *const u8) {
    unsafe {
        if *p == b'\0' {
            // no arguments allowed
            screen_set_cursor_colour((*ictx).ctx.s, -1);
        }
    }
}

/// Handle the OSC 133 sequence.
unsafe fn input_osc_133(ictx: *mut input_ctx, p: *const u8) {
    unsafe {
        let gd = (*(*ictx).ctx.s).grid;
        let line = (*(*ictx).ctx.s).cy + (*gd).hsize;

        if line > (*gd).hsize + (*gd).sy - 1 {
            return;
        }
        let gl = grid_get_line(gd, line);

        match *p {
            b'A' => (*gl).flags |= grid_line_flag::START_PROMPT,
            b'C' => (*gl).flags |= grid_line_flag::START_OUTPUT,
            _ => (),
        }
    }
}

/// Handle the OSC 52 sequence for setting the clipboard.
unsafe fn input_osc_52(ictx: *mut input_ctx, p: *const u8) {
    let __func__ = "input_osc_52";

    unsafe {
        let wp = (*ictx).wp;
        let mut buf: *const u8 = null_mut();
        let mut len: usize = 0;

        let mut ctx: screen_write_ctx = zeroed();
        let allow: *const u8 = c!("cpqs01234567");
        let mut flags: [u8; 13] = [0; 13];
        let mut j = 0;

        if wp.is_null() {
            return;
        }
        let state: i32 = options_get_number_(GLOBAL_OPTIONS, "set-clipboard") as i32;
        if state != 2 {
            return;
        }

        let mut end = strchr(p, b';' as i32);
        if end.is_null() {
            return;
        }
        end = end.add(1);
        if *end == b'\0' {
            return;
        }
        log_debug!("{}: {}", __func__, _s(end));

        let mut i = 0;
        while p.add(i) != end {
            if !strchr(allow, *p.add(i) as i32).is_null()
                && strchr((&raw mut flags) as *const u8, *p.add(i) as i32).is_null()
            {
                flags[j] = *p.add(i);
                j += 1;
            }
            i += 1;
        }
        // log_debug("%s: %.*s %s", __func__, (int)(end - p - 1), p, flags);

        if streq_(end, "?") {
            let pb = paste_get_top(null_mut());
            if !pb.is_null() {
                buf = paste_buffer_data(pb, &raw mut len);
            }
            if (*ictx).input_end == input_end_type::INPUT_END_BEL {
                input_reply_clipboard((*ictx).event, buf, len, c!("\x07"));
            } else {
                input_reply_clipboard((*ictx).event, buf, len, c!("\x1b\\"));
            }
            return;
        }

        len = (strlen(end) / 4) * 3;
        if len == 0 {
            return;
        }

        let out: *mut u8 = xmalloc(len).as_ptr().cast();
        let outlen = b64_pton(end, out, len);
        if outlen == -1 {
            free_(out);
            return;
        }

        screen_write_start_pane(&raw mut ctx, wp, null_mut());
        screen_write_setselection(
            &raw mut ctx,
            (&raw const flags) as *const u8,
            out,
            outlen as u32,
        );
        screen_write_stop(&raw mut ctx);
        notify_pane("pane-set-clipboard", wp);

        paste_add(null(), out.cast(), outlen as usize);
    }
}

/// Handle the OSC 104 sequence for unsetting (multiple) palette entries.
unsafe fn input_osc_104(ictx: *mut input_ctx, p: *const u8) {
    unsafe {
        let mut bad = false;
        let mut redraw = false;

        if *p == b'\0' {
            colour_palette_clear(NonNull::new((*ictx).palette).map(|e| &mut *e.as_ptr()));
            screen_write_fullredraw(&raw mut (*ictx).ctx);
            return;
        }

        let copy: *mut u8 = xstrdup(p).as_ptr();
        let mut s: *mut u8 = copy;
        while *s != b'\0' {
            let idx = strtol(s, &raw mut s, 10);
            if *s != b'\0' && *s != b';' {
                bad = true;
                break;
            }
            if !(0..256).contains(&idx) {
                bad = true;
                break;
            }
            if colour_palette_set(ptr_to_mut_ref((*ictx).palette), idx as i32, -1) != 0 {
                redraw = true;
            }
            if *s == b';' {
                s = s.add(1);
            }
        }
        if bad {
            log_debug!("bad OSC 104: {}", _s(p));
        }
        if redraw {
            screen_write_fullredraw(&raw mut (*ictx).ctx);
        }
        free_(copy);
    }
}

pub unsafe fn input_reply_clipboard(
    bev: *mut bufferevent,
    buf: *const u8,
    len: usize,
    end: *const u8,
) {
    unsafe {
        let mut out: *mut u8 = null_mut();
        let mut outlen: i32 = 0;

        if !buf.is_null() && len != 0 {
            if len >= (i32::MAX as usize * 3 / 4) - 1 {
                return;
            }
            outlen = 4 * ((len as i32 + 2) / 3) + 1;
            out = xmalloc(outlen as usize).as_ptr().cast();

            outlen = b64_ntop(buf.cast(), len, out, outlen as usize);
            if outlen == -1 {
                free_(out);
                return;
            }
        }

        bufferevent_write(bev, c!("\x1b]52;;").cast(), 6);
        if outlen != 0 {
            bufferevent_write(bev, out.cast(), outlen as usize);
        }
        bufferevent_write(bev, end.cast(), strlen(end));
        free_(out);
    }
}
