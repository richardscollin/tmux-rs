#![no_main]

use arbitrary::Arbitrary;
use tmux_rs_new::colour::{colour_join_rgb, colour_parse_x11, colour_split_rgb, COLOUR_FLAG_RGB};

/// Structured input covering all valid X11 colour specification formats.
#[derive(Arbitrary, Debug)]
enum ColourInput {
    /// rgb:XX/XX/XX (8-bit hex)
    Rgb8 { r: u8, g: u8, b: u8 },
    /// rgb:XXXX/XXXX/XXXX (16-bit hex)
    Rgb16 { r: u16, g: u16, b: u16 },
    /// #XXXXXX (8-bit hex)
    Hash8 { r: u8, g: u8, b: u8 },
    /// #XXXXXXXXXXXX (16-bit hex)
    Hash16 { r: u16, g: u16, b: u16 },
    /// R,G,B decimal
    Decimal { r: u8, g: u8, b: u8 },
    /// cmyk:C/M/Y/K floats
    Cmyk { c: u8, m: u8, y: u8, k: u8 },
    /// cmy:C/M/Y floats
    Cmy { c: u8, m: u8, y: u8 },
    /// Arbitrary ASCII string (for edge cases and no-panic checking)
    Ascii(Vec<u8>),
}

fn u8_to_frac(v: u8) -> f64 {
    v as f64 / 255.0
}

libfuzzer_sys::fuzz_target!(|input: ColourInput| {
    match &input {
        ColourInput::Rgb8 { r, g, b } => {
            let s = format!("rgb:{r:02x}/{g:02x}/{b:02x}");
            let expected = colour_join_rgb(*r, *g, *b);
            assert_eq!(colour_parse_x11(&s), expected, "Rgb8: {s:?}");
        }
        ColourInput::Rgb16 { r, g, b } => {
            let s = format!("rgb:{r:04x}/{g:04x}/{b:04x}");
            let expected = colour_join_rgb((*r >> 8) as u8, (*g >> 8) as u8, (*b >> 8) as u8);
            assert_eq!(colour_parse_x11(&s), expected, "Rgb16: {s:?}");
        }
        ColourInput::Hash8 { r, g, b } => {
            let s = format!("#{r:02x}{g:02x}{b:02x}");
            let expected = colour_join_rgb(*r, *g, *b);
            assert_eq!(colour_parse_x11(&s), expected, "Hash8: {s:?}");
        }
        ColourInput::Hash16 { r, g, b } => {
            let s = format!("#{r:04x}{g:04x}{b:04x}");
            let expected = colour_join_rgb((*r >> 8) as u8, (*g >> 8) as u8, (*b >> 8) as u8);
            assert_eq!(colour_parse_x11(&s), expected, "Hash16: {s:?}");
        }
        ColourInput::Decimal { r, g, b } => {
            let s = format!("{r},{g},{b}");
            let expected = colour_join_rgb(*r, *g, *b);
            assert_eq!(colour_parse_x11(&s), expected, "Decimal: {s:?}");
        }
        ColourInput::Cmyk { c, m, y, k } => {
            let (cf, mf, yf, kf) = (u8_to_frac(*c), u8_to_frac(*m), u8_to_frac(*y), u8_to_frac(*k));
            let s = format!("cmyk:{cf}/{mf}/{yf}/{kf}");
            let expected = colour_join_rgb(
                ((1.0 - cf) * (1.0 - kf) * 255.0) as u8,
                ((1.0 - mf) * (1.0 - kf) * 255.0) as u8,
                ((1.0 - yf) * (1.0 - kf) * 255.0) as u8,
            );
            assert_eq!(colour_parse_x11(&s), expected, "Cmyk: {s:?}");
        }
        ColourInput::Cmy { c, m, y } => {
            let (cf, mf, yf) = (u8_to_frac(*c), u8_to_frac(*m), u8_to_frac(*y));
            let s = format!("cmy:{cf}/{mf}/{yf}");
            let expected = colour_join_rgb(
                ((1.0 - cf) * 255.0) as u8,
                ((1.0 - mf) * 255.0) as u8,
                ((1.0 - yf) * 255.0) as u8,
            );
            assert_eq!(colour_parse_x11(&s), expected, "Cmy: {s:?}");
        }
        ColourInput::Ascii(bytes) => {
            let s: String = bytes
                .iter()
                .map(|&b| (b % 95 + 32) as char)
                .take(30)
                .collect();
            if s.contains('\0') {
                return;
            }
            let result = colour_parse_x11(&s);
            // If it parsed as an RGB colour, verify round-trip
            if result != -1 && (result & COLOUR_FLAG_RGB) != 0 {
                let (r, g, b) = colour_split_rgb(result);
                assert_eq!(
                    result,
                    colour_join_rgb(r, g, b),
                    "Round-trip failed for {s:?}: split=({r},{g},{b})"
                );
            }
        }
    }
});
