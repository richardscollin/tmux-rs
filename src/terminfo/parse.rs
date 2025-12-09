use std::collections::HashMap;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;

const ABSENT_ENTRY: i32 = -1;
const CANCELED_ENTRY: i32 = -2;

const BOOL_NAMES: [&str; 44] = [
    "bw", "am", "xsb", "xhp", "xenl", "eo", "gn", "hc", "km", "hs", "in", "db", "da", "mir",
    "msgr", "os", "eslok", "xt", "hz", "ul", "xon", "nxon", "mc5i", "chts", "nrrmc", "npc",
    "ndscr", "ccc", "bce", "hls", "xhpa", "crxm", "daisy", "xvpa", "sam", "cpix", "lpix", "OTbs",
    "OTns", "OTnc", "OTMT", "OTNL", "OTpt", "OTxr",
];

const NUM_NAMES: [&str; 39] = [
    "cols", "it", "lines", "lm", "xmc", "pb", "vt", "wsl", "nlab", "lh", "lw", "ma", "wnum",
    "colors", "pairs", "ncv", "bufsz", "spinv", "spinh", "maddr", "mjump", "mcs", "mls", "npins",
    "orc", "orl", "orhi", "orvi", "cps", "widcs", "btns", "bitwin", "bitype", "UTug", "OTdC",
    "OTdN", "OTdB", "OTdT", "OTkn",
];

const STR_NAMES: [&str; 414] = [
    "cbt", "bel", "cr", "csr", "tbc", "clear", "el", "ed", "hpa", "cmdch", "cup", "cud1", "home",
    "civis", "cub1", "mrcup", "cnorm", "cuf1", "ll", "cuu1", "cvvis", "dch1", "dl1", "dsl", "hd",
    "smacs", "blink", "bold", "smcup", "smdc", "dim", "smir", "invis", "prot", "rev", "smso",
    "smul", "ech", "rmacs", "sgr0", "rmcup", "rmdc", "rmir", "rmso", "rmul", "flash", "ff", "fsl",
    "is1", "is2", "is3", "if", "ich1", "il1", "ip", "kbs", "ktbc", "kclr", "kctab", "kdch1",
    "kdl1", "kcud1", "krmir", "kel", "ked", "kf0", "kf1", "kf10", "kf2", "kf3", "kf4", "kf5",
    "kf6", "kf7", "kf8", "kf9", "khome", "kich1", "kil1", "kcub1", "kll", "knp", "kpp", "kcuf1",
    "kind", "kri", "khts", "kcuu1", "rmkx", "smkx", "lf0", "lf1", "lf10", "lf2", "lf3", "lf4",
    "lf5", "lf6", "lf7", "lf8", "lf9", "rmm", "smm", "nel", "pad", "dch", "dl", "cud", "ich",
    "indn", "il", "cub", "cuf", "rin", "cuu", "pfkey", "pfloc", "pfx", "mc0", "mc4", "mc5", "rep",
    "rs1", "rs2", "rs3", "rf", "rc", "vpa", "sc", "ind", "ri", "sgr", "hts", "wind", "ht", "tsl",
    "uc", "hu", "iprog", "ka1", "ka3", "kb2", "kc1", "kc3", "mc5p", "rmp", "acsc", "pln", "kcbt",
    "smxon", "rmxon", "smam", "rmam", "xonc", "xoffc", "enacs", "smln", "rmln", "kbeg", "kcan",
    "kclo", "kcmd", "kcpy", "kcrt", "kend", "kent", "kext", "kfnd", "khlp", "kmrk", "kmsg", "kmov",
    "knxt", "kopn", "kopt", "kprv", "kprt", "krdo", "kref", "krfr", "krpl", "krst", "kres", "ksav",
    "kspd", "kund", "kBEG", "kCAN", "kCMD", "kCPY", "kCRT", "kDC", "kDL", "kslt", "kEND", "kEOL",
    "kEXT", "kFND", "kHLP", "kHOM", "kIC", "kLFT", "kMSG", "kMOV", "kNXT", "kOPT", "kPRV", "kPRT",
    "kRDO", "kRPL", "kRIT", "kRES", "kSAV", "kSPD", "kUND", "rfi", "kf11", "kf12", "kf13", "kf14",
    "kf15", "kf16", "kf17", "kf18", "kf19", "kf20", "kf21", "kf22", "kf23", "kf24", "kf25", "kf26",
    "kf27", "kf28", "kf29", "kf30", "kf31", "kf32", "kf33", "kf34", "kf35", "kf36", "kf37", "kf38",
    "kf39", "kf40", "kf41", "kf42", "kf43", "kf44", "kf45", "kf46", "kf47", "kf48", "kf49", "kf50",
    "kf51", "kf52", "kf53", "kf54", "kf55", "kf56", "kf57", "kf58", "kf59", "kf60", "kf61", "kf62",
    "kf63", "el1", "mgc", "smgl", "smgr", "fln", "sclk", "dclk", "rmclk", "cwin", "wingo", "hup",
    "dial", "qdial", "tone", "pulse", "hook", "pause", "wait", "u0", "u1", "u2", "u3", "u4", "u5",
    "u6", "u7", "u8", "u9", "op", "oc", "initc", "initp", "scp", "setf", "setb", "cpi", "lpi",
    "chr", "cvr", "defc", "swidm", "sdrfq", "sitm", "slm", "smicm", "snlq", "snrmq", "sshm",
    "ssubm", "ssupm", "sum", "rwidm", "ritm", "rlm", "rmicm", "rshm", "rsubm", "rsupm", "rum",
    "mhpa", "mcud1", "mcub1", "mcuf1", "mvpa", "mcuu1", "porder", "mcud", "mcub", "mcuf", "mcuu",
    "scs", "smgb", "smgbp", "smglp", "smgrp", "smgt", "smgtp", "sbim", "scsd", "rbim", "rcsd",
    "subcs", "supcs", "docr", "zerom", "csnm", "kmous", "minfo", "reqmp", "getm", "setaf", "setab",
    "pfxl", "devt", "csin", "s0ds", "s1ds", "s2ds", "s3ds", "smglr", "smgtb", "birep", "binel",
    "bicr", "colornm", "defbi", "endbi", "setcolor", "slines", "dispc", "smpch", "rmpch", "smsc",
    "rmsc", "pctrm", "scesc", "scesa", "ehhlm", "elhlm", "elohlm", "erhlm", "ethlm", "evhlm",
    "sgr1", "slength", "OTi2", "OTrs", "OTnl", "OTbs", "OTko", "OTma", "OTG2", "OTG3", "OTG1",
    "OTG4", "OTGR", "OTGL", "OTGU", "OTGD", "OTGH", "OTGV", "OTGC", "meml", "memu", "box1",
];

#[derive(num_enum::TryFromPrimitive)]
#[repr(u16)]
enum TerminfoMagic {
    /// Original format, 16-bit numbers
    Magic1 = 0x011a,
    /// 32-bit numbers
    Magic2 = 0x021e,
}

/// Errors detected when parsing terminfo database
#[derive(thiserror::Error, Debug)]
pub enum TerminfoError {
    #[error("String without terminating NUL")]
    UnterminatedString,
    #[error("Unsupported terminfo format")]
    UnsupportedFormat,
    #[error("I/O error")]
    IO(#[from] std::io::Error),
    #[error("Invalid UTF-8 string")]
    Utf8(#[from] std::str::Utf8Error),
}

fn read_boolean(reader: &mut impl Read) -> Result<Option<bool>, TerminfoError> {
    let mut buffer = [0u8; 1];
    reader.read_exact(&mut buffer)?;
    let value = match buffer[0] {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    };
    Ok(value)
}

fn read_le16(reader: &mut impl Read) -> Result<u16, TerminfoError> {
    let mut buffer = [0u8; 2];
    reader.read_exact(&mut buffer)?;
    let value = u16::from_le_bytes(buffer);
    Ok(value)
}

fn read_slice<'a>(reader: &mut Cursor<&'a [u8]>, size: usize) -> Result<&'a [u8], TerminfoError> {
    let start = reader.position() as usize;
    let end = start + size;
    reader.seek_relative(size as i64)?;
    Ok(&reader.get_ref()[start..end])
}

fn get_string(string_table: &[u8], offset: usize) -> Result<&[u8], TerminfoError> {
    let string_length = &string_table[offset..].iter().position(|c| *c == b'\0');
    let Some(string_length) = string_length else {
        return Err(TerminfoError::UnterminatedString);
    };
    Ok(&string_table[offset..offset + string_length])
}

/// Convert ABSENT and CANCELED to None
fn check_offset(size: u16) -> Option<usize> {
    match i32::from(size as i16) {
        ABSENT_ENTRY => None,
        CANCELED_ENTRY => None,
        _ => Some(usize::from(size)),
    }
}

/// Skip a byte if needed to ensure 2-byte alignment
fn align_cursor(reader: &mut Cursor<&[u8]>) -> Result<(), TerminfoError> {
    let position = reader.position();
    if position & 1 == 1 {
        reader.seek_relative(1)?;
    }
    Ok(())
}

/// Parsed terminfo entry
pub struct Terminfo<'a> {
    pub booleans: HashMap<&'a str, bool>,
    pub numbers: HashMap<&'a str, i32>,
    pub strings: HashMap<&'a str, &'a [u8]>,
    number_size: usize,
}

impl<'a> Terminfo<'a> {
    fn new() -> Self {
        Self {
            booleans: HashMap::default(),
            numbers: HashMap::default(),
            strings: HashMap::default(),
            number_size: 2,
        }
    }

    /// Parse terminfo database from the supplied buffer
    pub fn parse(buffer: &'a [u8]) -> Result<Self, TerminfoError> {
        let mut terminfo = Self::new();
        let mut reader = Cursor::new(buffer);
        terminfo.parse_base(&mut reader)?;
        match terminfo.parse_extended(&mut reader) {
            Ok(()) => {}
            Err(TerminfoError::IO(_)) => {} // missing extended data is OK
            Err(err) => return Err(err),
        }
        Ok(terminfo)
    }

    fn read_number(&self, reader: &mut Cursor<&'a [u8]>) -> Result<Option<i32>, TerminfoError> {
        let value = if self.number_size == 4 {
            let mut buffer = [0u8; 4];
            reader.read_exact(&mut buffer)?;
            i32::from_le_bytes(buffer)
        } else {
            let mut buffer = [0u8; 2];
            reader.read_exact(&mut buffer)?;
            i32::from(i16::from_le_bytes(buffer))
        };
        if value > 0 { Ok(Some(value)) } else { Ok(None) }
    }

    /// Parse base capabilities
    fn parse_base(&mut self, mut reader: &mut Cursor<&'a [u8]>) -> Result<(), TerminfoError> {
        let magic = read_le16(&mut reader)?;
        let name_size = usize::from(read_le16(&mut reader)?);
        let bool_count = usize::from(read_le16(&mut reader)?);
        let num_count = usize::from(read_le16(&mut reader)?);
        let str_count = usize::from(read_le16(&mut reader)?);
        let str_size = usize::from(read_le16(&mut reader)?);

        self.number_size = match TerminfoMagic::try_from(magic) {
            Ok(TerminfoMagic::Magic1) => 2,
            Ok(TerminfoMagic::Magic2) => 4,
            Err(_) => return Err(TerminfoError::UnsupportedFormat),
        };

        if bool_count > BOOL_NAMES.len()
            || num_count > NUM_NAMES.len()
            || str_count > STR_NAMES.len()
        {
            return Err(TerminfoError::UnsupportedFormat);
        }

        // Skip terminal names/aliases, we are not using them
        reader.seek_relative(name_size as i64)?;

        for name in BOOL_NAMES.iter().take(bool_count) {
            let value = read_boolean(&mut reader)?;
            if value == Some(true) {
                self.booleans.insert(*name, true);
            }
        }

        align_cursor(reader)?;

        for name in NUM_NAMES.iter().take(num_count) {
            if let Some(number) = self.read_number(reader)? {
                self.numbers.insert(*name, number);
            }
        }

        let str_offsets = read_slice(reader, std::mem::size_of::<u16>() * str_count)?;
        let mut str_offsets_reader = Cursor::new(str_offsets);

        let str_table = read_slice(reader, str_size)?;

        for name in STR_NAMES.iter().take(str_count) {
            let offset = read_le16(&mut str_offsets_reader)?;
            let Some(offset) = check_offset(offset) else {
                continue;
            };
            let value = get_string(str_table, offset)?;
            self.strings.insert(*name, value);
        }

        Ok(())
    }

    /// Parse extended capabilities
    fn parse_extended(&mut self, mut reader: &mut Cursor<&'a [u8]>) -> Result<(), TerminfoError> {
        align_cursor(reader)?;

        let ext_bool_count = usize::from(read_le16(&mut reader)?);
        let ext_num_count = usize::from(read_le16(&mut reader)?);
        let ext_str_count = usize::from(read_le16(&mut reader)?);
        let _ext_str_usage = usize::from(read_le16(&mut reader)?);
        let ext_str_limit = usize::from(read_le16(&mut reader)?);

        let ext_bools = read_slice(reader, ext_bool_count)?;
        let mut ext_bools_reader = Cursor::new(ext_bools);
        align_cursor(reader)?;

        let ext_nums = read_slice(reader, self.number_size * ext_num_count)?;
        let mut ext_nums_reader = Cursor::new(ext_nums);

        let ext_strs = read_slice(reader, std::mem::size_of::<u16>() * ext_str_count)?;
        let mut ext_strs_reader = Cursor::new(ext_strs);

        let ext_name_count = ext_bool_count + ext_num_count + ext_str_count;
        let ext_names = read_slice(reader, std::mem::size_of::<u16>() * ext_name_count)?;
        let mut ext_names_reader = Cursor::new(ext_names);

        let ext_str_table = read_slice(reader, ext_str_limit)?;

        let mut names_base = 0;
        loop {
            let Ok(offset) = read_le16(&mut ext_strs_reader) else {
                break;
            };
            let Some(offset) = check_offset(offset) else {
                continue;
            };
            names_base += get_string(ext_str_table, offset)?.len() + 1;
        }

        let names_table = &ext_str_table[names_base..];

        loop {
            let Ok(value) = read_boolean(&mut ext_bools_reader) else {
                break;
            };
            let Some(value) = value else {
                return Err(TerminfoError::UnsupportedFormat);
            };
            let Ok(name_offset) = read_le16(&mut ext_names_reader) else {
                return Err(TerminfoError::UnsupportedFormat);
            };
            let Some(name_offset) = check_offset(name_offset) else {
                return Err(TerminfoError::UnsupportedFormat);
            };
            let name = get_string(names_table, name_offset)?;
            self.booleans.insert(str::from_utf8(name)?, value);
        }

        loop {
            let Ok(value) = self.read_number(&mut ext_nums_reader) else {
                break;
            };
            let Some(value) = value else {
                return Err(TerminfoError::UnsupportedFormat);
            };
            let Ok(name_offset) = read_le16(&mut ext_names_reader) else {
                return Err(TerminfoError::UnsupportedFormat);
            };
            let Some(name_offset) = check_offset(name_offset) else {
                return Err(TerminfoError::UnsupportedFormat);
            };
            let name = get_string(names_table, name_offset)?;
            self.numbers.insert(str::from_utf8(name)?, value);
        }

        ext_strs_reader.set_position(0);
        loop {
            let Ok(str_offset) = read_le16(&mut ext_strs_reader) else {
                break;
            };
            let Some(str_offset) = check_offset(str_offset) else {
                return Err(TerminfoError::UnsupportedFormat);
            };
            let value = get_string(ext_str_table, str_offset)?;

            let Ok(name_offset) = read_le16(&mut ext_names_reader) else {
                return Err(TerminfoError::UnsupportedFormat);
            };
            let Some(name_offset) = check_offset(name_offset) else {
                return Err(TerminfoError::UnsupportedFormat);
            };
            let name = get_string(names_table, name_offset)?;
            self.strings.insert(str::from_utf8(name)?, value);
        }

        Ok(())
    }
}
