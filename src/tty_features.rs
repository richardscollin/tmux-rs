// Copyright (c) 2020 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use crate::*;

unsafe impl Sync for tty_feature {}
#[repr(C)]
struct tty_feature {
    name: &'static str,
    capabilities: &'static [&'static str],
    flags: term_flags,
}
impl tty_feature {
    const fn new(
        name: &'static str,
        capabilities: &'static [&'static str],
        flags: term_flags,
    ) -> Self {
        Self {
            name,
            capabilities,
            flags,
        }
    }
}

static TTY_FEATURE_TITLE_CAPABILITIES: &[&str] = &[
    "tsl=\\E]0;", // should be using TS really
    "fsl=\\a",
];
static TTY_FEATURE_TITLE: tty_feature =
    tty_feature::new("title", TTY_FEATURE_TITLE_CAPABILITIES, term_flags::empty());

/// Terminal has OSC 7 working directory.
static TTY_FEATURE_OSC7_CAPABILITIES: &[&str] = &["Swd=\\E]7;", "fsl=\\a"];
static TTY_FEATURE_OSC7: tty_feature =
    tty_feature::new("osc7", TTY_FEATURE_OSC7_CAPABILITIES, term_flags::empty());

/// Terminal has mouse support.
static TTY_FEATURE_MOUSE_CAPABILITIES: &[&str] = &["kmous=\\E[M"];
static TTY_FEATURE_MOUSE: tty_feature =
    tty_feature::new("mouse", TTY_FEATURE_MOUSE_CAPABILITIES, term_flags::empty());

/// Terminal can set the clipboard with OSC 52.
static TTY_FEATURE_CLIPBOARD_CAPABILITIES: &[&str] = &["Ms=\\E]52;%p1%s;%p2%s\\a"];
static TTY_FEATURE_CLIPBOARD: tty_feature = tty_feature::new(
    "clipboard",
    TTY_FEATURE_CLIPBOARD_CAPABILITIES,
    term_flags::empty(),
);

// #if defined (__OpenBSD__) || (defined(NCURSES_VERSION_MAJOR) && (NCURSES_VERSION_MAJOR > 5 ||  (NCURSES_VERSION_MAJOR == 5 && NCURSES_VERSION_MINOR > 8)))

/// Terminal supports OSC 8 hyperlinks.
#[cfg(feature = "hyperlinks")]
static TTY_FEATURE_HYPERLINKS_CAPABILITIES: &[&str] =
    &["*:Hls=\\E]8;%?%p1%l%tid=%p1%s%;;%p2%s\\E\\\\"];
#[cfg(not(feature = "hyperlinks"))]
static TTY_FEATURE_HYPERLINKS_CAPABILITIES: &[&str] = &[];
static TTY_FEATURE_HYPERLINKS: tty_feature = tty_feature::new(
    "hyperlinks",
    TTY_FEATURE_HYPERLINKS_CAPABILITIES,
    term_flags::empty(),
);

/// Terminal supports RGB colour. This replaces setab and setaf also since
/// terminals with RGB have versions that do not allow setting colours from the
/// 256 palette.
static TTY_FEATURE_RGB_CAPABILITIES: &[&str] = &[
    "AX",
    "setrgbf=\\E[38;2;%p1%d;%p2%d;%p3%dm",
    "setrgbb=\\E[48;2;%p1%d;%p2%d;%p3%dm",
    "setab=\\E[%?%p1%{8}%<%t4%p1%d%e%p1%{16}%<%t10%p1%{8}%-%d%e48;5;%p1%d%;m",
    "setaf=\\E[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m",
];
static TTY_FEATURE_RGB: tty_feature = tty_feature::new(
    "RGB",
    TTY_FEATURE_RGB_CAPABILITIES,
    term_flags::TERM_256COLOURS.union(term_flags::TERM_RGBCOLOURS),
);

/// Terminal supports 256 colours.
static TTY_FEATURE_256_CAPABILITIES: &[&str] = &[
    "AX",
    "setab=\\E[%?%p1%{8}%<%t4%p1%d%e%p1%{16}%<%t10%p1%{8}%-%d%e48;5;%p1%d%;m",
    "setaf=\\E[%?%p1%{8}%<%t3%p1%d%e%p1%{16}%<%t9%p1%{8}%-%d%e38;5;%p1%d%;m",
];
static TTY_FEATURE_256: tty_feature = tty_feature::new(
    "256",
    TTY_FEATURE_256_CAPABILITIES,
    term_flags::TERM_256COLOURS,
);

/// Terminal supports overline.
static TTY_FEATURE_OVERLINE_CAPABILITIES: &[&str] = &["Smol=\\E[53m"];
static TTY_FEATURE_OVERLINE: tty_feature = tty_feature::new(
    "overline",
    TTY_FEATURE_OVERLINE_CAPABILITIES,
    term_flags::empty(),
);

/// Terminal supports underscore styles.
static TTY_FEATURE_USSTYLE_CAPABILITIES: &[&str] = &[
    "Smulx=\\E[4::%p1%dm",
    "Setulc=\\E[58::2::%p1%{65536}%/%d::%p1%{256}%/%{255}%&%d::%p1%{255}%&%d%;m",
    "Setulc1=\\E[58::5::%p1%dm",
    "ol=\\E[59m",
];
static TTY_FEATURE_USSTYLE: tty_feature = tty_feature::new(
    "usstyle",
    TTY_FEATURE_USSTYLE_CAPABILITIES,
    term_flags::empty(),
);

/// Terminal supports bracketed paste.
static TTY_FEATURE_BPASTE_CAPABILITIES: &[&str] = &["Enbp=\\E[?2004h", "Dsbp=\\E[?2004l"];
static TTY_FEATURE_BPASTE: tty_feature = tty_feature::new(
    "bpaste",
    TTY_FEATURE_BPASTE_CAPABILITIES,
    term_flags::empty(),
);

/// Terminal supports focus reporting.
static TTY_FEATURE_FOCUS_CAPABILITIES: &[&str] = &["Enfcs=\\E[?1004h", "Dsfcs=\\E[?1004l"];
static TTY_FEATURE_FOCUS: tty_feature =
    tty_feature::new("focus", TTY_FEATURE_FOCUS_CAPABILITIES, term_flags::empty());

/// Terminal supports cursor styles.
static TTY_FEATURE_CSTYLE_CAPABILITIES: &[&str] = &["Ss=\\E[%p1%d q", "Se=\\E[2 q"];
static TTY_FEATURE_CSTYLE: tty_feature = tty_feature::new(
    "cstyle",
    TTY_FEATURE_CSTYLE_CAPABILITIES,
    term_flags::empty(),
);

/// Terminal supports cursor colours.
static TTY_FEATURE_CCOLOUR_CAPABILITIES: &[&str] = &["Cs=\\E]12;%p1%s\\a", "Cr=\\E]112\\a"];
static TTY_FEATURE_CCOLOUR: tty_feature = tty_feature::new(
    "ccolour",
    TTY_FEATURE_CCOLOUR_CAPABILITIES,
    term_flags::empty(),
);

/// Terminal supports strikethrough.
static TTY_FEATURE_STRIKETHROUGH_CAPABILITIES: &[&str] = &["smxx=\\E[9m"];
static TTY_FEATURE_STRIKETHROUGH: tty_feature = tty_feature::new(
    "strikethrough",
    TTY_FEATURE_STRIKETHROUGH_CAPABILITIES,
    term_flags::empty(),
);

/// Terminal supports synchronized updates.
static TTY_FEATURE_SYNC_CAPABILITIES: &[&str] = &["Sync=\\E[?2026%?%p1%{1}%-%tl%eh%;"];
static TTY_FEATURE_SYNC: tty_feature =
    tty_feature::new("sync", TTY_FEATURE_SYNC_CAPABILITIES, term_flags::empty());

/// Terminal supports extended keys.
static TTY_FEATURE_EXTKEYS_CAPABILITIES: &[&str] = &["Eneks=\\E[>4;2m", "Dseks=\\E[>4m"];
static TTY_FEATURE_EXTKEYS: tty_feature = tty_feature::new(
    "extkeys",
    TTY_FEATURE_EXTKEYS_CAPABILITIES,
    term_flags::empty(),
);

/// Terminal supports DECSLRM margins.
static TTY_FEATURE_MARGINS_CAPABILITIES: &[&str] = &[
    "Enmg=\\E[?69h",
    "Dsmg=\\E[?69l",
    "Clmg=\\E[s",
    "Cmg=\\E[%i%p1%d;%p2%ds",
];
static TTY_FEATURE_MARGINS: tty_feature = tty_feature::new(
    "margins",
    TTY_FEATURE_MARGINS_CAPABILITIES,
    term_flags::TERM_DECSLRM,
);

/// Terminal supports DECFRA rectangle fill.
static TTY_FEATURE_RECTFILL_CAPABILITIES: &[&str] = &["Rect"];
static TTY_FEATURE_RECTFILL: tty_feature = tty_feature::new(
    "rectfill",
    TTY_FEATURE_RECTFILL_CAPABILITIES,
    term_flags::TERM_DECFRA,
);

/// Use builtin function keys only.
static TTY_FEATURE_IGNOREFKEYS_CAPABILITIES: &[&str] = &[
    "kf0@", "kf1@", "kf2@", "kf3@", "kf4@", "kf5@", "kf6@", "kf7@", "kf8@", "kf9@", "kf10@",
    "kf11@", "kf12@", "kf13@", "kf14@", "kf15@", "kf16@", "kf17@", "kf18@", "kf19@", "kf20@",
    "kf21@", "kf22@", "kf23@", "kf24@", "kf25@", "kf26@", "kf27@", "kf28@", "kf29@", "kf30@",
    "kf31@", "kf32@", "kf33@", "kf34@", "kf35@", "kf36@", "kf37@", "kf38@", "kf39@", "kf40@",
    "kf41@", "kf42@", "kf43@", "kf44@", "kf45@", "kf46@", "kf47@", "kf48@", "kf49@", "kf50@",
    "kf51@", "kf52@", "kf53@", "kf54@", "kf55@", "kf56@", "kf57@", "kf58@", "kf59@", "kf60@",
    "kf61@", "kf62@", "kf63@",
];

static TTY_FEATURE_IGNOREFKEYS: tty_feature = tty_feature::new(
    "ignorefkeys",
    TTY_FEATURE_IGNOREFKEYS_CAPABILITIES,
    term_flags::empty(),
);

/// Terminal has sixel capability.
static TTY_FEATURE_SIXEL_CAPABILITIES: &[&str] = &["Sxl"];
static TTY_FEATURE_SIXEL: tty_feature = tty_feature::new(
    "sixel",
    TTY_FEATURE_SIXEL_CAPABILITIES,
    term_flags::TERM_SIXEL,
);

/// Available terminal features.
static TTY_FEATURES: [&tty_feature; 20] = [
    &TTY_FEATURE_256,
    &TTY_FEATURE_BPASTE,
    &TTY_FEATURE_CCOLOUR,
    &TTY_FEATURE_CLIPBOARD,
    &TTY_FEATURE_HYPERLINKS,
    &TTY_FEATURE_CSTYLE,
    &TTY_FEATURE_EXTKEYS,
    &TTY_FEATURE_FOCUS,
    &TTY_FEATURE_IGNOREFKEYS,
    &TTY_FEATURE_MARGINS,
    &TTY_FEATURE_MOUSE,
    &TTY_FEATURE_OSC7,
    &TTY_FEATURE_OVERLINE,
    &TTY_FEATURE_RECTFILL,
    &TTY_FEATURE_RGB,
    &TTY_FEATURE_SIXEL,
    &TTY_FEATURE_STRIKETHROUGH,
    &TTY_FEATURE_SYNC,
    &TTY_FEATURE_TITLE,
    &TTY_FEATURE_USSTYLE,
];

pub unsafe fn tty_add_features(feat: *mut i32, s: &str, separators: *const u8) {
    unsafe {
        log_debug!("adding terminal features {}", s);

        let copy = xstrdup__(s);
        let mut loop_ = copy;
        let mut next;

        while {
            next = strsep(&raw mut loop_, separators);
            !next.is_null()
        } {
            let Some(i) = TTY_FEATURES
                .iter()
                .position(|tf| libc::strcaseeq_(next, tf.name))
            else {
                log_debug!("unknown terminal feature: {}", _s(next));
                break;
            };

            let tf = TTY_FEATURES[i];
            if !(*feat) & (1 << i) != 0 {
                log_debug!("adding terminal feature: {}", tf.name);
                (*feat) |= 1 << i;
            }
        }
        free_(copy);
    }
}

pub unsafe fn tty_get_features(feat: i32) -> *const u8 {
    static mut S_BUF: [MaybeUninit<u8>; 512] = [MaybeUninit::uninit(); 512];
    unsafe {
        let s: *mut u8 = (&raw mut S_BUF).cast();
        // const struct tty_feature *tf;

        *s = b'\0';
        for (i, tf) in TTY_FEATURES.iter().copied().enumerate() {
            if (!feat & (1 << i)) != 0 {
                continue;
            }

            strlcat_(s, tf.name, 512);
            strlcat_(s, ",", 512);
        }
        if *s != b'\0' {
            *s.add(strlen(s) - 1) = b'\0';
        }

        s
    }
}

pub unsafe fn tty_apply_features(term: *mut tty_term, feat: i32) -> bool {
    if feat == 0 {
        return false;
    }

    unsafe {
        log_debug!("applying terminal features: {}", _s(tty_get_features(feat)));

        for (i, tf) in TTY_FEATURES.iter().copied().enumerate() {
            if ((*term).features & (1 << i) != 0) || (!feat & (1 << i)) != 0 {
                continue;
            }

            log_debug!("applying terminal feature: {}", tf.name);
            for capability in tf.capabilities {
                log_debug!("adding capability: {}", capability);
                tty_term_apply(term, capability, 1);
            }
            (*term).flags |= tf.flags;
        }
        if ((*term).features | feat) == (*term).features {
            return false;
        }
        (*term).features |= feat;
    }

    true
}

pub unsafe fn tty_default_features(feat: *mut i32, name: *const u8, version: u32) {
    struct entry {
        name: &'static CStr,
        version: u32,
        features: &'static str,
    }
    macro_rules! TTY_FEATURES_BASE_MODERN_XTERM {
        () => {
            "256,RGB,bpaste,clipboard,mouse,strikethrough,title"
        };
    }

    // TODO note version isn't init in the C code
    #[rustfmt::skip]
    static TABLE: &[entry] = &[
        entry { name: c"mintty", features: concat!( TTY_FEATURES_BASE_MODERN_XTERM!(), ",ccolour,cstyle,extkeys,margins,overline,usstyle"), version: 0, },
        entry { name: c"tmux", features: concat!( TTY_FEATURES_BASE_MODERN_XTERM!(), ",ccolour,cstyle,focus,overline,usstyle,hyperlinks"), version: 0, },
        entry { name: c"rxvt-unicode", features: "256,bpaste,ccolour,cstyle,mouse,title,ignorefkeys", version: 0, },
        entry { name: c"iTerm2", features: concat!( TTY_FEATURES_BASE_MODERN_XTERM!(), ",cstyle,extkeys,margins,usstyle,sync,osc7,hyperlinks"), version: 0, },
        // xterm also supports DECSLRM and DECFRA, but they can be
        // disabled so not set it here - they will be added if
        // secondary DA shows VT420.
        entry { name: c"XTerm", features: concat!(TTY_FEATURES_BASE_MODERN_XTERM!(), ",ccolour,cstyle,extkeys,focus"), version: 0, },
    ];

    unsafe {
        for e in TABLE {
            if libc::strcmp(e.name.as_ptr().cast(), name) != 0 {
                continue;
            }
            if version != 0 && version < e.version {
                continue;
            }
            tty_add_features(feat, e.features, c!(","));
        }
    }
}
