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

// Special key codes.
#[repr(u64)]
pub(crate) enum keyc {
    // Focus events.
    KEYC_FOCUS_IN = KEYC_BASE,
    KEYC_FOCUS_OUT,

    // "Any" key, used if not found in key table.
    KEYC_ANY,

    // Paste brackets.
    KEYC_PASTE_START,
    KEYC_PASTE_END,

    // Mouse keys.
    KEYC_MOUSE,       // unclassified mouse event
    KEYC_DRAGGING,    // dragging in progress
    KEYC_DOUBLECLICK, // double click complete

    KEYC_MOUSEMOVE_PANE,
    KEYC_MOUSEMOVE_STATUS,
    KEYC_MOUSEMOVE_STATUS_LEFT,
    KEYC_MOUSEMOVE_STATUS_RIGHT,
    KEYC_MOUSEMOVE_STATUS_DEFAULT,
    KEYC_MOUSEMOVE_BORDER,

    KEYC_MOUSEDOWN1_PANE,
    KEYC_MOUSEDOWN1_STATUS,
    KEYC_MOUSEDOWN1_STATUS_LEFT,
    KEYC_MOUSEDOWN1_STATUS_RIGHT,
    KEYC_MOUSEDOWN1_STATUS_DEFAULT,
    KEYC_MOUSEDOWN1_BORDER,

    KEYC_MOUSEDOWN2_PANE,
    KEYC_MOUSEDOWN2_STATUS,
    KEYC_MOUSEDOWN2_STATUS_LEFT,
    KEYC_MOUSEDOWN2_STATUS_RIGHT,
    KEYC_MOUSEDOWN2_STATUS_DEFAULT,
    KEYC_MOUSEDOWN2_BORDER,

    KEYC_MOUSEDOWN3_PANE,
    KEYC_MOUSEDOWN3_STATUS,
    KEYC_MOUSEDOWN3_STATUS_LEFT,
    KEYC_MOUSEDOWN3_STATUS_RIGHT,
    KEYC_MOUSEDOWN3_STATUS_DEFAULT,
    KEYC_MOUSEDOWN3_BORDER,

    KEYC_MOUSEDOWN6_PANE,
    KEYC_MOUSEDOWN6_STATUS,
    KEYC_MOUSEDOWN6_STATUS_LEFT,
    KEYC_MOUSEDOWN6_STATUS_RIGHT,
    KEYC_MOUSEDOWN6_STATUS_DEFAULT,
    KEYC_MOUSEDOWN6_BORDER,

    KEYC_MOUSEDOWN7_PANE,
    KEYC_MOUSEDOWN7_STATUS,
    KEYC_MOUSEDOWN7_STATUS_LEFT,
    KEYC_MOUSEDOWN7_STATUS_RIGHT,
    KEYC_MOUSEDOWN7_STATUS_DEFAULT,
    KEYC_MOUSEDOWN7_BORDER,

    KEYC_MOUSEDOWN8_PANE,
    KEYC_MOUSEDOWN8_STATUS,
    KEYC_MOUSEDOWN8_STATUS_LEFT,
    KEYC_MOUSEDOWN8_STATUS_RIGHT,
    KEYC_MOUSEDOWN8_STATUS_DEFAULT,
    KEYC_MOUSEDOWN8_BORDER,

    KEYC_MOUSEDOWN9_PANE,
    KEYC_MOUSEDOWN9_STATUS,
    KEYC_MOUSEDOWN9_STATUS_LEFT,
    KEYC_MOUSEDOWN9_STATUS_RIGHT,
    KEYC_MOUSEDOWN9_STATUS_DEFAULT,
    KEYC_MOUSEDOWN9_BORDER,

    KEYC_MOUSEDOWN10_PANE,
    KEYC_MOUSEDOWN10_STATUS,
    KEYC_MOUSEDOWN10_STATUS_LEFT,
    KEYC_MOUSEDOWN10_STATUS_RIGHT,
    KEYC_MOUSEDOWN10_STATUS_DEFAULT,
    KEYC_MOUSEDOWN10_BORDER,

    KEYC_MOUSEDOWN11_PANE,
    KEYC_MOUSEDOWN11_STATUS,
    KEYC_MOUSEDOWN11_STATUS_LEFT,
    KEYC_MOUSEDOWN11_STATUS_RIGHT,
    KEYC_MOUSEDOWN11_STATUS_DEFAULT,
    KEYC_MOUSEDOWN11_BORDER,

    KEYC_MOUSEUP1_PANE,
    KEYC_MOUSEUP1_STATUS,
    KEYC_MOUSEUP1_STATUS_LEFT,
    KEYC_MOUSEUP1_STATUS_RIGHT,
    KEYC_MOUSEUP1_STATUS_DEFAULT,
    KEYC_MOUSEUP1_BORDER,

    KEYC_MOUSEUP2_PANE,
    KEYC_MOUSEUP2_STATUS,
    KEYC_MOUSEUP2_STATUS_LEFT,
    KEYC_MOUSEUP2_STATUS_RIGHT,
    KEYC_MOUSEUP2_STATUS_DEFAULT,
    KEYC_MOUSEUP2_BORDER,

    KEYC_MOUSEUP3_PANE,
    KEYC_MOUSEUP3_STATUS,
    KEYC_MOUSEUP3_STATUS_LEFT,
    KEYC_MOUSEUP3_STATUS_RIGHT,
    KEYC_MOUSEUP3_STATUS_DEFAULT,
    KEYC_MOUSEUP3_BORDER,

    KEYC_MOUSEUP6_PANE,
    KEYC_MOUSEUP6_STATUS,
    KEYC_MOUSEUP6_STATUS_LEFT,
    KEYC_MOUSEUP6_STATUS_RIGHT,
    KEYC_MOUSEUP6_STATUS_DEFAULT,
    KEYC_MOUSEUP6_BORDER,

    KEYC_MOUSEUP7_PANE,
    KEYC_MOUSEUP7_STATUS,
    KEYC_MOUSEUP7_STATUS_LEFT,
    KEYC_MOUSEUP7_STATUS_RIGHT,
    KEYC_MOUSEUP7_STATUS_DEFAULT,
    KEYC_MOUSEUP7_BORDER,

    KEYC_MOUSEUP8_PANE,
    KEYC_MOUSEUP8_STATUS,
    KEYC_MOUSEUP8_STATUS_LEFT,
    KEYC_MOUSEUP8_STATUS_RIGHT,
    KEYC_MOUSEUP8_STATUS_DEFAULT,
    KEYC_MOUSEUP8_BORDER,

    KEYC_MOUSEUP9_PANE,
    KEYC_MOUSEUP9_STATUS,
    KEYC_MOUSEUP9_STATUS_LEFT,
    KEYC_MOUSEUP9_STATUS_RIGHT,
    KEYC_MOUSEUP9_STATUS_DEFAULT,
    KEYC_MOUSEUP9_BORDER,

    KEYC_MOUSEUP10_PANE,
    KEYC_MOUSEUP10_STATUS,
    KEYC_MOUSEUP10_STATUS_LEFT,
    KEYC_MOUSEUP10_STATUS_RIGHT,
    KEYC_MOUSEUP10_STATUS_DEFAULT,
    KEYC_MOUSEUP10_BORDER,

    KEYC_MOUSEUP11_PANE,
    KEYC_MOUSEUP11_STATUS,
    KEYC_MOUSEUP11_STATUS_LEFT,
    KEYC_MOUSEUP11_STATUS_RIGHT,
    KEYC_MOUSEUP11_STATUS_DEFAULT,
    KEYC_MOUSEUP11_BORDER,

    KEYC_MOUSEDRAG1_PANE,
    KEYC_MOUSEDRAG1_STATUS,
    KEYC_MOUSEDRAG1_STATUS_LEFT,
    KEYC_MOUSEDRAG1_STATUS_RIGHT,
    KEYC_MOUSEDRAG1_STATUS_DEFAULT,
    KEYC_MOUSEDRAG1_BORDER,

    KEYC_MOUSEDRAG2_PANE,
    KEYC_MOUSEDRAG2_STATUS,
    KEYC_MOUSEDRAG2_STATUS_LEFT,
    KEYC_MOUSEDRAG2_STATUS_RIGHT,
    KEYC_MOUSEDRAG2_STATUS_DEFAULT,
    KEYC_MOUSEDRAG2_BORDER,

    KEYC_MOUSEDRAG3_PANE,
    KEYC_MOUSEDRAG3_STATUS,
    KEYC_MOUSEDRAG3_STATUS_LEFT,
    KEYC_MOUSEDRAG3_STATUS_RIGHT,
    KEYC_MOUSEDRAG3_STATUS_DEFAULT,
    KEYC_MOUSEDRAG3_BORDER,

    KEYC_MOUSEDRAG6_PANE,
    KEYC_MOUSEDRAG6_STATUS,
    KEYC_MOUSEDRAG6_STATUS_LEFT,
    KEYC_MOUSEDRAG6_STATUS_RIGHT,
    KEYC_MOUSEDRAG6_STATUS_DEFAULT,
    KEYC_MOUSEDRAG6_BORDER,

    KEYC_MOUSEDRAG7_PANE,
    KEYC_MOUSEDRAG7_STATUS,
    KEYC_MOUSEDRAG7_STATUS_LEFT,
    KEYC_MOUSEDRAG7_STATUS_RIGHT,
    KEYC_MOUSEDRAG7_STATUS_DEFAULT,
    KEYC_MOUSEDRAG7_BORDER,

    KEYC_MOUSEDRAG8_PANE,
    KEYC_MOUSEDRAG8_STATUS,
    KEYC_MOUSEDRAG8_STATUS_LEFT,
    KEYC_MOUSEDRAG8_STATUS_RIGHT,
    KEYC_MOUSEDRAG8_STATUS_DEFAULT,
    KEYC_MOUSEDRAG8_BORDER,

    KEYC_MOUSEDRAG9_PANE,
    KEYC_MOUSEDRAG9_STATUS,
    KEYC_MOUSEDRAG9_STATUS_LEFT,
    KEYC_MOUSEDRAG9_STATUS_RIGHT,
    KEYC_MOUSEDRAG9_STATUS_DEFAULT,
    KEYC_MOUSEDRAG9_BORDER,

    KEYC_MOUSEDRAG10_PANE,
    KEYC_MOUSEDRAG10_STATUS,
    KEYC_MOUSEDRAG10_STATUS_LEFT,
    KEYC_MOUSEDRAG10_STATUS_RIGHT,
    KEYC_MOUSEDRAG10_STATUS_DEFAULT,
    KEYC_MOUSEDRAG10_BORDER,

    KEYC_MOUSEDRAG11_PANE,
    KEYC_MOUSEDRAG11_STATUS,
    KEYC_MOUSEDRAG11_STATUS_LEFT,
    KEYC_MOUSEDRAG11_STATUS_RIGHT,
    KEYC_MOUSEDRAG11_STATUS_DEFAULT,
    KEYC_MOUSEDRAG11_BORDER,

    KEYC_MOUSEDRAGEND1_PANE,
    KEYC_MOUSEDRAGEND1_STATUS,
    KEYC_MOUSEDRAGEND1_STATUS_LEFT,
    KEYC_MOUSEDRAGEND1_STATUS_RIGHT,
    KEYC_MOUSEDRAGEND1_STATUS_DEFAULT,
    KEYC_MOUSEDRAGEND1_BORDER,

    KEYC_MOUSEDRAGEND2_PANE,
    KEYC_MOUSEDRAGEND2_STATUS,
    KEYC_MOUSEDRAGEND2_STATUS_LEFT,
    KEYC_MOUSEDRAGEND2_STATUS_RIGHT,
    KEYC_MOUSEDRAGEND2_STATUS_DEFAULT,
    KEYC_MOUSEDRAGEND2_BORDER,

    KEYC_MOUSEDRAGEND3_PANE,
    KEYC_MOUSEDRAGEND3_STATUS,
    KEYC_MOUSEDRAGEND3_STATUS_LEFT,
    KEYC_MOUSEDRAGEND3_STATUS_RIGHT,
    KEYC_MOUSEDRAGEND3_STATUS_DEFAULT,
    KEYC_MOUSEDRAGEND3_BORDER,

    KEYC_MOUSEDRAGEND6_PANE,
    KEYC_MOUSEDRAGEND6_STATUS,
    KEYC_MOUSEDRAGEND6_STATUS_LEFT,
    KEYC_MOUSEDRAGEND6_STATUS_RIGHT,
    KEYC_MOUSEDRAGEND6_STATUS_DEFAULT,
    KEYC_MOUSEDRAGEND6_BORDER,

    KEYC_MOUSEDRAGEND7_PANE,
    KEYC_MOUSEDRAGEND7_STATUS,
    KEYC_MOUSEDRAGEND7_STATUS_LEFT,
    KEYC_MOUSEDRAGEND7_STATUS_RIGHT,
    KEYC_MOUSEDRAGEND7_STATUS_DEFAULT,
    KEYC_MOUSEDRAGEND7_BORDER,

    KEYC_MOUSEDRAGEND8_PANE,
    KEYC_MOUSEDRAGEND8_STATUS,
    KEYC_MOUSEDRAGEND8_STATUS_LEFT,
    KEYC_MOUSEDRAGEND8_STATUS_RIGHT,
    KEYC_MOUSEDRAGEND8_STATUS_DEFAULT,
    KEYC_MOUSEDRAGEND8_BORDER,

    KEYC_MOUSEDRAGEND9_PANE,
    KEYC_MOUSEDRAGEND9_STATUS,
    KEYC_MOUSEDRAGEND9_STATUS_LEFT,
    KEYC_MOUSEDRAGEND9_STATUS_RIGHT,
    KEYC_MOUSEDRAGEND9_STATUS_DEFAULT,
    KEYC_MOUSEDRAGEND9_BORDER,

    KEYC_MOUSEDRAGEND10_PANE,
    KEYC_MOUSEDRAGEND10_STATUS,
    KEYC_MOUSEDRAGEND10_STATUS_LEFT,
    KEYC_MOUSEDRAGEND10_STATUS_RIGHT,
    KEYC_MOUSEDRAGEND10_STATUS_DEFAULT,
    KEYC_MOUSEDRAGEND10_BORDER,

    KEYC_MOUSEDRAGEND11_PANE,
    KEYC_MOUSEDRAGEND11_STATUS,
    KEYC_MOUSEDRAGEND11_STATUS_LEFT,
    KEYC_MOUSEDRAGEND11_STATUS_RIGHT,
    KEYC_MOUSEDRAGEND11_STATUS_DEFAULT,
    KEYC_MOUSEDRAGEND11_BORDER,

    KEYC_WHEELUP_PANE,
    KEYC_WHEELUP_STATUS,
    KEYC_WHEELUP_STATUS_LEFT,
    KEYC_WHEELUP_STATUS_RIGHT,
    KEYC_WHEELUP_STATUS_DEFAULT,
    KEYC_WHEELUP_BORDER,

    KEYC_WHEELDOWN_PANE,
    KEYC_WHEELDOWN_STATUS,
    KEYC_WHEELDOWN_STATUS_LEFT,
    KEYC_WHEELDOWN_STATUS_RIGHT,
    KEYC_WHEELDOWN_STATUS_DEFAULT,
    KEYC_WHEELDOWN_BORDER,

    KEYC_SECONDCLICK1_PANE,
    KEYC_SECONDCLICK1_STATUS,
    KEYC_SECONDCLICK1_STATUS_LEFT,
    KEYC_SECONDCLICK1_STATUS_RIGHT,
    KEYC_SECONDCLICK1_STATUS_DEFAULT,
    KEYC_SECONDCLICK1_BORDER,

    KEYC_SECONDCLICK2_PANE,
    KEYC_SECONDCLICK2_STATUS,
    KEYC_SECONDCLICK2_STATUS_LEFT,
    KEYC_SECONDCLICK2_STATUS_RIGHT,
    KEYC_SECONDCLICK2_STATUS_DEFAULT,
    KEYC_SECONDCLICK2_BORDER,

    KEYC_SECONDCLICK3_PANE,
    KEYC_SECONDCLICK3_STATUS,
    KEYC_SECONDCLICK3_STATUS_LEFT,
    KEYC_SECONDCLICK3_STATUS_RIGHT,
    KEYC_SECONDCLICK3_STATUS_DEFAULT,
    KEYC_SECONDCLICK3_BORDER,

    KEYC_SECONDCLICK6_PANE,
    KEYC_SECONDCLICK6_STATUS,
    KEYC_SECONDCLICK6_STATUS_LEFT,
    KEYC_SECONDCLICK6_STATUS_RIGHT,
    KEYC_SECONDCLICK6_STATUS_DEFAULT,
    KEYC_SECONDCLICK6_BORDER,

    KEYC_SECONDCLICK7_PANE,
    KEYC_SECONDCLICK7_STATUS,
    KEYC_SECONDCLICK7_STATUS_LEFT,
    KEYC_SECONDCLICK7_STATUS_RIGHT,
    KEYC_SECONDCLICK7_STATUS_DEFAULT,
    KEYC_SECONDCLICK7_BORDER,

    KEYC_SECONDCLICK8_PANE,
    KEYC_SECONDCLICK8_STATUS,
    KEYC_SECONDCLICK8_STATUS_LEFT,
    KEYC_SECONDCLICK8_STATUS_RIGHT,
    KEYC_SECONDCLICK8_STATUS_DEFAULT,
    KEYC_SECONDCLICK8_BORDER,

    KEYC_SECONDCLICK9_PANE,
    KEYC_SECONDCLICK9_STATUS,
    KEYC_SECONDCLICK9_STATUS_LEFT,
    KEYC_SECONDCLICK9_STATUS_RIGHT,
    KEYC_SECONDCLICK9_STATUS_DEFAULT,
    KEYC_SECONDCLICK9_BORDER,

    KEYC_SECONDCLICK10_PANE,
    KEYC_SECONDCLICK10_STATUS,
    KEYC_SECONDCLICK10_STATUS_LEFT,
    KEYC_SECONDCLICK10_STATUS_RIGHT,
    KEYC_SECONDCLICK10_STATUS_DEFAULT,
    KEYC_SECONDCLICK10_BORDER,

    KEYC_SECONDCLICK11_PANE,
    KEYC_SECONDCLICK11_STATUS,
    KEYC_SECONDCLICK11_STATUS_LEFT,
    KEYC_SECONDCLICK11_STATUS_RIGHT,
    KEYC_SECONDCLICK11_STATUS_DEFAULT,
    KEYC_SECONDCLICK11_BORDER,

    KEYC_DOUBLECLICK1_PANE,
    KEYC_DOUBLECLICK1_STATUS,
    KEYC_DOUBLECLICK1_STATUS_LEFT,
    KEYC_DOUBLECLICK1_STATUS_RIGHT,
    KEYC_DOUBLECLICK1_STATUS_DEFAULT,
    KEYC_DOUBLECLICK1_BORDER,

    KEYC_DOUBLECLICK2_PANE,
    KEYC_DOUBLECLICK2_STATUS,
    KEYC_DOUBLECLICK2_STATUS_LEFT,
    KEYC_DOUBLECLICK2_STATUS_RIGHT,
    KEYC_DOUBLECLICK2_STATUS_DEFAULT,
    KEYC_DOUBLECLICK2_BORDER,

    KEYC_DOUBLECLICK3_PANE,
    KEYC_DOUBLECLICK3_STATUS,
    KEYC_DOUBLECLICK3_STATUS_LEFT,
    KEYC_DOUBLECLICK3_STATUS_RIGHT,
    KEYC_DOUBLECLICK3_STATUS_DEFAULT,
    KEYC_DOUBLECLICK3_BORDER,

    KEYC_DOUBLECLICK6_PANE,
    KEYC_DOUBLECLICK6_STATUS,
    KEYC_DOUBLECLICK6_STATUS_LEFT,
    KEYC_DOUBLECLICK6_STATUS_RIGHT,
    KEYC_DOUBLECLICK6_STATUS_DEFAULT,
    KEYC_DOUBLECLICK6_BORDER,

    KEYC_DOUBLECLICK7_PANE,
    KEYC_DOUBLECLICK7_STATUS,
    KEYC_DOUBLECLICK7_STATUS_LEFT,
    KEYC_DOUBLECLICK7_STATUS_RIGHT,
    KEYC_DOUBLECLICK7_STATUS_DEFAULT,
    KEYC_DOUBLECLICK7_BORDER,

    KEYC_DOUBLECLICK8_PANE,
    KEYC_DOUBLECLICK8_STATUS,
    KEYC_DOUBLECLICK8_STATUS_LEFT,
    KEYC_DOUBLECLICK8_STATUS_RIGHT,
    KEYC_DOUBLECLICK8_STATUS_DEFAULT,
    KEYC_DOUBLECLICK8_BORDER,

    KEYC_DOUBLECLICK9_PANE,
    KEYC_DOUBLECLICK9_STATUS,
    KEYC_DOUBLECLICK9_STATUS_LEFT,
    KEYC_DOUBLECLICK9_STATUS_RIGHT,
    KEYC_DOUBLECLICK9_STATUS_DEFAULT,
    KEYC_DOUBLECLICK9_BORDER,

    KEYC_DOUBLECLICK10_PANE,
    KEYC_DOUBLECLICK10_STATUS,
    KEYC_DOUBLECLICK10_STATUS_LEFT,
    KEYC_DOUBLECLICK10_STATUS_RIGHT,
    KEYC_DOUBLECLICK10_STATUS_DEFAULT,
    KEYC_DOUBLECLICK10_BORDER,

    KEYC_DOUBLECLICK11_PANE,
    KEYC_DOUBLECLICK11_STATUS,
    KEYC_DOUBLECLICK11_STATUS_LEFT,
    KEYC_DOUBLECLICK11_STATUS_RIGHT,
    KEYC_DOUBLECLICK11_STATUS_DEFAULT,
    KEYC_DOUBLECLICK11_BORDER,

    KEYC_TRIPLECLICK1_PANE,
    KEYC_TRIPLECLICK1_STATUS,
    KEYC_TRIPLECLICK1_STATUS_LEFT,
    KEYC_TRIPLECLICK1_STATUS_RIGHT,
    KEYC_TRIPLECLICK1_STATUS_DEFAULT,
    KEYC_TRIPLECLICK1_BORDER,

    KEYC_TRIPLECLICK2_PANE,
    KEYC_TRIPLECLICK2_STATUS,
    KEYC_TRIPLECLICK2_STATUS_LEFT,
    KEYC_TRIPLECLICK2_STATUS_RIGHT,
    KEYC_TRIPLECLICK2_STATUS_DEFAULT,
    KEYC_TRIPLECLICK2_BORDER,

    KEYC_TRIPLECLICK3_PANE,
    KEYC_TRIPLECLICK3_STATUS,
    KEYC_TRIPLECLICK3_STATUS_LEFT,
    KEYC_TRIPLECLICK3_STATUS_RIGHT,
    KEYC_TRIPLECLICK3_STATUS_DEFAULT,
    KEYC_TRIPLECLICK3_BORDER,

    KEYC_TRIPLECLICK6_PANE,
    KEYC_TRIPLECLICK6_STATUS,
    KEYC_TRIPLECLICK6_STATUS_LEFT,
    KEYC_TRIPLECLICK6_STATUS_RIGHT,
    KEYC_TRIPLECLICK6_STATUS_DEFAULT,
    KEYC_TRIPLECLICK6_BORDER,

    KEYC_TRIPLECLICK7_PANE,
    KEYC_TRIPLECLICK7_STATUS,
    KEYC_TRIPLECLICK7_STATUS_LEFT,
    KEYC_TRIPLECLICK7_STATUS_RIGHT,
    KEYC_TRIPLECLICK7_STATUS_DEFAULT,
    KEYC_TRIPLECLICK7_BORDER,

    KEYC_TRIPLECLICK8_PANE,
    KEYC_TRIPLECLICK8_STATUS,
    KEYC_TRIPLECLICK8_STATUS_LEFT,
    KEYC_TRIPLECLICK8_STATUS_RIGHT,
    KEYC_TRIPLECLICK8_STATUS_DEFAULT,
    KEYC_TRIPLECLICK8_BORDER,

    KEYC_TRIPLECLICK9_PANE,
    KEYC_TRIPLECLICK9_STATUS,
    KEYC_TRIPLECLICK9_STATUS_LEFT,
    KEYC_TRIPLECLICK9_STATUS_RIGHT,
    KEYC_TRIPLECLICK9_STATUS_DEFAULT,
    KEYC_TRIPLECLICK9_BORDER,

    KEYC_TRIPLECLICK10_PANE,
    KEYC_TRIPLECLICK10_STATUS,
    KEYC_TRIPLECLICK10_STATUS_LEFT,
    KEYC_TRIPLECLICK10_STATUS_RIGHT,
    KEYC_TRIPLECLICK10_STATUS_DEFAULT,
    KEYC_TRIPLECLICK10_BORDER,

    KEYC_TRIPLECLICK11_PANE,
    KEYC_TRIPLECLICK11_STATUS,
    KEYC_TRIPLECLICK11_STATUS_LEFT,
    KEYC_TRIPLECLICK11_STATUS_RIGHT,
    KEYC_TRIPLECLICK11_STATUS_DEFAULT,
    KEYC_TRIPLECLICK11_BORDER,

    // Backspace key.
    KEYC_BSPACE,

    // Function keys.
    KEYC_F1,
    KEYC_F2,
    KEYC_F3,
    KEYC_F4,
    KEYC_F5,
    KEYC_F6,
    KEYC_F7,
    KEYC_F8,
    KEYC_F9,
    KEYC_F10,
    KEYC_F11,
    KEYC_F12,
    KEYC_IC,
    KEYC_DC,
    KEYC_HOME,
    KEYC_END,
    KEYC_NPAGE,
    KEYC_PPAGE,
    KEYC_BTAB,

    // Arrow keys.
    KEYC_UP,
    KEYC_DOWN,
    KEYC_LEFT,
    KEYC_RIGHT,

    // Numeric keypad.
    KEYC_KP_SLASH,
    KEYC_KP_STAR,
    KEYC_KP_MINUS,
    KEYC_KP_SEVEN,
    KEYC_KP_EIGHT,
    KEYC_KP_NINE,
    KEYC_KP_PLUS,
    KEYC_KP_FOUR,
    KEYC_KP_FIVE,
    KEYC_KP_SIX,
    KEYC_KP_ONE,
    KEYC_KP_TWO,
    KEYC_KP_THREE,
    KEYC_KP_ENTER,
    KEYC_KP_ZERO,
    KEYC_KP_PERIOD,

    // End of special keys.
    KEYC_BASE_END,
}
