use crate::cmd_::cmd_command_prompt::cmd_command_prompt_cdata;
use crate::cmd_::cmd_confirm_before::cmd_confirm_before_data;
use crate::*;

// compatibility shims for rust 1.85
//
// all of this is code which can be thrown away

// Compatibility Notes:
//
// 1.88:
// - Default for raw pointers
//
// 1.87
// - offset_from_unsigned
// - is_multiple_of

impl Default for SyncCharPtr {
    fn default() -> Self {
        Self(null())
    }
}

impl Default for cmd_confirm_before_data {
    fn default() -> Self {
        Self {
            item: null_mut(),
            cmdlist: null_mut(),
            confirm_key: 0,
            default_yes: false,
        }
    }
}

impl Default for cmd_command_prompt_cdata<'_> {
    fn default() -> Self {
        Self {
            item: null_mut(),
            state: null_mut(),

            flags: prompt_flags::default(),
            prompt_type: prompt_type::default(),

            prompts: null_mut(),
            count: 0,
            current: 0,

            argc: 0,
            argv: null_mut(),
        }
    }
}

impl Default for cmd_find_state {
    fn default() -> Self {
        Self {
            flags: Default::default(),
            current: null_mut(),
            s: null_mut(),
            wl: null_mut(),
            w: null_mut(),
            wp: null_mut(),
            idx: Default::default(),
        }
    }
}

impl Default for cmd_parse_input<'_> {
    fn default() -> Self {
        Self {
            flags: Default::default(),
            file: Default::default(),
            line: Default::default(),
            item: null_mut(),
            c: null_mut(),
            fs: Default::default(),
        }
    }
}

pub trait UnwrapOrDefault {
    type Inner;
    fn unwrap_or_default_(self) -> Self::Inner
    where
        Self: Sized;
}

impl<T> UnwrapOrDefault for Option<*mut T> {
    type Inner = *mut T;
    fn unwrap_or_default_(self) -> Self::Inner {
        self.unwrap_or(std::ptr::null_mut())
    }
}

impl<T> UnwrapOrDefault for Option<*const T> {
    type Inner = *const T;
    fn unwrap_or_default_(self) -> Self::Inner {
        self.unwrap_or(std::ptr::null())
    }
}

pub trait IsMultipleOf {
    fn is_multiple_of_(self, rhs: u32) -> bool
    where
        Self: Sized;
}

impl IsMultipleOf for u32 {
    fn is_multiple_of_(self, rhs: u32) -> bool {
        if rhs == 0 { false } else { self % rhs == 0 }
    }
}

pub trait OffsetFromUnsigned {
    type OffsetFrom;
    unsafe fn offset_from_unsigned_(self, rhs: Self::OffsetFrom) -> usize
    where
        Self: Sized;
}

impl<T> OffsetFromUnsigned for NonNull<T> {
    type OffsetFrom = NonNull<T>;
    unsafe fn offset_from_unsigned_(self, rhs: Self::OffsetFrom) -> usize
    where
        Self: Sized,
    {
        unsafe { self.offset_from(rhs) as usize }
    }
}

impl<T> OffsetFromUnsigned for *mut T {
    type OffsetFrom = *const T;
    unsafe fn offset_from_unsigned_(self, rhs: Self::OffsetFrom) -> usize
    where
        Self: Sized,
    {
        unsafe { self.offset_from(rhs) as usize }
    }
}

impl<T> OffsetFromUnsigned for *const T {
    type OffsetFrom = *const T;
    unsafe fn offset_from_unsigned_(self, rhs: Self::OffsetFrom) -> usize
    where
        Self: Sized,
    {
        unsafe { self.offset_from(rhs) as usize }
    }
}
