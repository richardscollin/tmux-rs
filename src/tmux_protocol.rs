pub const PROTOCOL_VERSION: i32 = 8;

/// Message types.
#[repr(i32)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum msgtype {
    MSG_ZERO = 0, // TODO rust added so not ub on static init
    MSG_VERSION = 12,

    MSG_IDENTIFY_FLAGS = 100,
    MSG_IDENTIFY_TERM,
    MSG_IDENTIFY_TTYNAME,
    MSG_IDENTIFY_OLDCWD, /* unused */
    MSG_IDENTIFY_STDIN,
    MSG_IDENTIFY_ENVIRON,
    MSG_IDENTIFY_DONE,
    MSG_IDENTIFY_CLIENTPID,
    MSG_IDENTIFY_CWD,
    MSG_IDENTIFY_FEATURES,
    MSG_IDENTIFY_STDOUT,
    MSG_IDENTIFY_LONGFLAGS,
    MSG_IDENTIFY_TERMINFO,

    MSG_COMMAND = 200,
    MSG_DETACH,
    MSG_DETACHKILL,
    MSG_EXIT,
    MSG_EXITED,
    MSG_EXITING,
    MSG_LOCK,
    MSG_READY,
    MSG_RESIZE,
    MSG_SHELL,
    MSG_SHUTDOWN,
    MSG_OLDSTDERR, /* unused */
    MSG_OLDSTDIN,  /* unused */
    MSG_OLDSTDOUT, /* unused */
    MSG_SUSPEND,
    MSG_UNLOCK,
    MSG_WAKEUP,
    MSG_EXEC,
    MSG_FLAGS,

    MSG_READ_OPEN = 300,
    MSG_READ,
    MSG_READ_DONE,
    MSG_WRITE_OPEN,
    MSG_WRITE,
    MSG_WRITE_READY,
    MSG_WRITE_CLOSE,
    MSG_READ_CANCEL,
}

#[derive(Debug)]
pub struct InvalidEnumValue;
impl TryFrom<u32> for msgtype {
    type Error = InvalidEnumValue;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => msgtype::MSG_ZERO,
            12 => msgtype::MSG_VERSION,
            100 => msgtype::MSG_IDENTIFY_FLAGS,
            101 => msgtype::MSG_IDENTIFY_TERM,
            102 => msgtype::MSG_IDENTIFY_TTYNAME,
            103 => msgtype::MSG_IDENTIFY_OLDCWD,
            104 => msgtype::MSG_IDENTIFY_STDIN,
            105 => msgtype::MSG_IDENTIFY_ENVIRON,
            106 => msgtype::MSG_IDENTIFY_DONE,
            107 => msgtype::MSG_IDENTIFY_CLIENTPID,
            108 => msgtype::MSG_IDENTIFY_CWD,
            109 => msgtype::MSG_IDENTIFY_FEATURES,
            110 => msgtype::MSG_IDENTIFY_STDOUT,
            111 => msgtype::MSG_IDENTIFY_LONGFLAGS,
            112 => msgtype::MSG_IDENTIFY_TERMINFO,
            200 => msgtype::MSG_COMMAND,
            201 => msgtype::MSG_DETACH,
            202 => msgtype::MSG_DETACHKILL,
            203 => msgtype::MSG_EXIT,
            204 => msgtype::MSG_EXITED,
            205 => msgtype::MSG_EXITING,
            206 => msgtype::MSG_LOCK,
            207 => msgtype::MSG_READY,
            208 => msgtype::MSG_RESIZE,
            209 => msgtype::MSG_SHELL,
            210 => msgtype::MSG_SHUTDOWN,
            211 => msgtype::MSG_OLDSTDERR,
            212 => msgtype::MSG_OLDSTDIN,
            213 => msgtype::MSG_OLDSTDOUT,
            214 => msgtype::MSG_SUSPEND,
            215 => msgtype::MSG_UNLOCK,
            216 => msgtype::MSG_WAKEUP,
            217 => msgtype::MSG_EXEC,
            218 => msgtype::MSG_FLAGS,
            300 => msgtype::MSG_READ_OPEN,
            301 => msgtype::MSG_READ,
            302 => msgtype::MSG_READ_DONE,
            303 => msgtype::MSG_WRITE_OPEN,
            304 => msgtype::MSG_WRITE,
            305 => msgtype::MSG_WRITE_READY,
            306 => msgtype::MSG_WRITE_CLOSE,
            307 => msgtype::MSG_READ_CANCEL,
            _ => return Err(InvalidEnumValue),
        })
    }
}

#[repr(C)]
pub struct msg_command {
    pub argc: i32,
}

#[repr(C)]
pub struct msg_read_open {
    pub stream: i32,
    pub fd: i32,
}

#[repr(C)]
pub struct msg_read_data {
    pub stream: i32,
}

#[repr(C)]
pub struct msg_read_done {
    pub stream: i32,
    pub error: i32,
}

#[repr(C)]
pub struct msg_read_cancel {
    pub stream: i32,
}

#[repr(C)]
pub struct msg_write_open {
    pub stream: i32,
    pub fd: i32,
    pub flags: i32,
}

#[repr(C)]
pub struct msg_write_data {
    pub stream: i32,
}

#[repr(C)]
pub struct msg_write_ready {
    pub stream: i32,
    pub error: i32,
}

#[repr(C)]
pub struct msg_write_close {
    pub stream: i32,
}
