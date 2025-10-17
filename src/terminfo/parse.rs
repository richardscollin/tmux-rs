use std::collections::HashMap;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;

const ABSENT_ENTRY_U16: u16 = u16::MAX;
const CANCELED_ENTRY_U16: u16 = u16::MAX - 1;

const ABSENT_ENTRY_U32: u32 = u32::MAX;
const CANCELED_ENTRY_U32: u32 = u32::MAX - 1;

pub static BOOL_NAMES: &[&str] = &[
    "bw", "am", "xsb", "xhp", "xenl", "eo", "gn", "hc", "km", "hs", "in", "db", "da", "mir",
    "msgr", "os", "eslok", "xt", "hz", "ul", "xon", "nxon", "mc5i", "chts", "nrrmc", "npc",
    "ndscr", "ccc", "bce", "hls", "xhpa", "crxm", "daisy", "xvpa", "sam", "cpix", "lpix", "OTbs",
    "OTns", "OTnc", "OTMT", "OTNL", "OTpt", "OTxr",
];

pub static NUMBER_NAMES: &[&str] = &[
    "cols", "it", "lines", "lm", "xmc", "pb", "vt", "wsl", "nlab", "lh", "lw", "ma", "wnum",
    "colors", "pairs", "ncv", "bufsz", "spinv", "spinh", "maddr", "mjump", "mcs", "mls", "npins",
    "orc", "orl", "orhi", "orvi", "cps", "widcs", "btns", "bitwin", "bitype", "UTug", "OTdC",
    "OTdN", "OTdB", "OTdT", "OTkn",
];

pub static STRING_NAMES: &[&str] = &[
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
    MAGIC_16BIT = 0x011a,
    MAGIC_32BIT = 0x021e,
}

#[derive(thiserror::Error, Debug)]
pub enum TerminfoError {
    #[error("bad or unsupported magic")]
    BadMagic,
    #[error("string without terminating NUL")]
    UnterminatedString,
    #[error("unsupported terminfo format")]
    UnsupportedFormat,
    #[error("I/O error")]
    IO(#[from] std::io::Error),
    #[error("not a valid UTF-8 string")]
    BadUtf8(#[from] std::string::FromUtf8Error),
}

pub struct Terminfo {
    pub booleans: HashMap<&'static str, bool>,
    pub numbers: HashMap<&'static str, u32>,
    pub strings: HashMap<&'static str, Vec<u8>>,
}

fn read_u8(reader: &mut impl Read) -> Result<u8, TerminfoError> {
    let mut buffer = [0u8; 1];
    reader.read_exact(&mut buffer)?;
    Ok(buffer[0])
}

fn read_le16(reader: &mut impl Read) -> Result<u16, TerminfoError> {
    let mut buffer = [0u8; 2];
    reader.read_exact(&mut buffer)?;
    let value = u16::from_le_bytes(buffer);
    Ok(value)
}

fn read_le32(reader: &mut impl Read) -> Result<u32, TerminfoError> {
    let mut buffer = [0u8; 4];
    reader.read_exact(&mut buffer)?;
    let value = u32::from_le_bytes(buffer);
    Ok(value)
}

/// Convert ABSENT and CANCELED to None
fn adjust_u16(size: u16) -> Option<u16> {
    match size {
        ABSENT_ENTRY_U16 => None,
        CANCELED_ENTRY_U16 => None,
        n => Some(n),
    }
}

/// Convert ABSENT and CANCELED to None
fn adjust_u32(size: u32) -> Option<u32> {
    match size {
        ABSENT_ENTRY_U32 => None,
        CANCELED_ENTRY_U32 => None,
        n => Some(n),
    }
}

pub fn terminfo_parse(buffer: &[u8]) -> Result<Terminfo, TerminfoError> {
    let mut reader = Cursor::new(buffer);

    let magic = read_le16(&mut reader)?;

    let numbers_32bit = match TerminfoMagic::try_from(magic) {
        Ok(TerminfoMagic::MAGIC_16BIT) => false,
        Ok(TerminfoMagic::MAGIC_32BIT) => true,
        Err(_) => return Err(TerminfoError::BadMagic),
    };

    let name_size = adjust_u16(read_le16(&mut reader)?).unwrap_or(0) as usize;
    let bool_count = adjust_u16(read_le16(&mut reader)?).unwrap_or(0) as usize;
    let number_count = adjust_u16(read_le16(&mut reader)?).unwrap_or(0) as usize;
    let string_count = adjust_u16(read_le16(&mut reader)?).unwrap_or(0) as usize;
    let string_table_size = adjust_u16(read_le16(&mut reader)?).unwrap_or(0) as usize;

    reader.seek_relative(name_size as i64)?;

    let mut booleans = HashMap::new();
    for name in BOOL_NAMES.iter().take(bool_count) {
        let value = read_u8(&mut reader)?;
        if value == 1 {
            booleans.insert(*name, true);
        }
    }

    // Restore 2-byte alignment
    if (name_size + bool_count) % 2 == 1 {
        reader.seek_relative(1)?;
    }

    let mut numbers = HashMap::new();
    if numbers_32bit {
        for name in NUMBER_NAMES.iter().take(number_count) {
            let number = read_le32(&mut reader)?;
            if let Some(number) = adjust_u32(number) {
                numbers.insert(*name, number);
            }
        }
    } else {
        for name in NUMBER_NAMES.iter().take(number_count) {
            let number = read_le16(&mut reader)?;
            if let Some(number) = adjust_u16(number) {
                numbers.insert(*name, number.into());
            }
        }
    }

    let mut strings = HashMap::new();
    let mut string_offsets = Vec::new();
    for _i in 0..string_count {
        let offset = read_le16(&mut reader)?;
        string_offsets.push(offset);
    }

    let mut string_table = vec![0u8; string_table_size];
    reader.read_exact(&mut string_table)?;

    for (i, offset) in string_offsets.iter().enumerate() {
        let Some(offset) = adjust_u16(*offset) else {
            continue;
        };
        let offset = usize::from(offset);
        let key = STRING_NAMES[i];
        let index = &string_table[offset..].iter().position(|c| *c == b'\0');
        let Some(index) = index else {
            return Err(TerminfoError::UnterminatedString);
        };
        let value = (string_table[offset..offset + index]).to_vec();
        strings.insert(key, value);
    }

    // Restore 2-byte alignment
    if string_table_size % 2 == 1 {
        reader.seek_relative(1)?;
    }

    let ext_bool_count = adjust_u16(read_le16(&mut reader)?).unwrap_or(0) as usize;
    let ext_number_count = adjust_u16(read_le16(&mut reader)?).unwrap_or(0) as usize;
    let ext_string_count = adjust_u16(read_le16(&mut reader)?).unwrap_or(0) as usize;
    let ext_offset_count = adjust_u16(read_le16(&mut reader)?).unwrap_or(0) as usize;
    let ext_string_table_size = adjust_u16(read_le16(&mut reader)?).unwrap_or(0) as usize;

    let mut ext_booleans = vec![];
    for _i in 0..ext_bool_count {
        let value = read_u8(&mut reader)?;
        ext_booleans.push(value == 1);
    }

    // Restore 2-byte alignment
    if ext_bool_count % 2 == 1 {
        reader.seek_relative(1)?;
    }

    let mut ext_numbers = vec![];
    if numbers_32bit {
        for _i in 0..ext_number_count {
            let number = read_le32(&mut reader)?;
            ext_numbers.push(number);
        }
    } else {
        for _i in 0..ext_number_count {
            let number = read_le16(&mut reader)?;
            ext_numbers.push(number as u32);
        }
    }

    let mut ext_string_offsets = Vec::new();
    for _i in 0..ext_string_count {
        let offset = read_le16(&mut reader)?;
        ext_string_offsets.push(offset);
    }

    let mut ext_name_offsets = Vec::new();
    let ext_name_count = ext_bool_count + ext_number_count + ext_string_count;
    if ext_offset_count != ext_string_count + ext_name_count {
        return Err(TerminfoError::UnsupportedFormat);
    }
    for _i in 0..ext_name_count {
        let offset = read_le16(&mut reader)?;
        ext_name_offsets.push(offset);
    }

    let mut ext_string_table = vec![0u8; ext_string_table_size];
    reader.read_exact(&mut ext_string_table)?;

    let terminfo = Terminfo {
        booleans,
        numbers,
        strings,
    };
    Ok(terminfo)
}
