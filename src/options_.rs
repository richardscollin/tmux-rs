// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use libc::{fnmatch, isdigit, sscanf, strcasecmp, strchr, strcmp, strncmp, strstr};

use std::cmp::Ordering;

use crate::compat::{
    RB_GENERATE,
    queue::tailq_foreach,
    tree::{rb_find, rb_foreach, rb_init, rb_insert, rb_min, rb_next, rb_remove},
};
use crate::log::fatalx_c;

//
// Option handling; each option has a name, type and value and is stored in
// a red-black tree.
//

#[repr(C)]
#[derive(Copy, Clone)]
pub struct options_array_item {
    pub index: u32,
    pub value: options_value,
    pub entry: rb_entry<options_array_item>,
}

pub unsafe fn options_array_cmp(
    a1: *const options_array_item,
    a2: *const options_array_item,
) -> Ordering {
    unsafe { (*a1).index.cmp(&(*a2).index) }
}
RB_GENERATE!(
    options_array,
    options_array_item,
    entry,
    discr_entry,
    options_array_cmp
);

#[repr(C)]
pub struct options_entry {
    pub owner: *mut options,
    pub name: *const c_char,
    pub tableentry: *const options_table_entry,
    pub value: options_value,
    pub cached: i32,
    pub style: style,
    pub entry: rb_entry<options_entry>,
}

#[repr(C)]
pub struct options {
    pub tree: rb_head<options_entry>,
    pub parent: *mut options,
}

#[allow(non_snake_case)]
#[inline]
pub fn OPTIONS_IS_STRING(o: *const options_entry) -> bool {
    unsafe {
        (*o).tableentry.is_null()
            || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_STRING
    }
}

#[allow(non_snake_case)]
#[inline]
pub fn OPTIONS_IS_NUMBER(o: *const options_entry) -> bool {
    unsafe {
        !(*o).tableentry.is_null()
            && ((*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_NUMBER
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_KEY
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_COLOUR
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_FLAG
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_CHOICE)
    }
}

#[allow(non_snake_case)]
#[inline]
pub fn OPTIONS_IS_COMMAND(o: *const options_entry) -> bool {
    unsafe {
        !(*o).tableentry.is_null()
            && (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_COMMAND
    }
}

#[allow(non_snake_case)]
#[inline]
pub fn OPTIONS_IS_ARRAY(o: *const options_entry) -> bool {
    unsafe {
        !(*o).tableentry.is_null() && ((*(*o).tableentry).flags & OPTIONS_TABLE_IS_ARRAY) != 0
    }
}

RB_GENERATE!(options_tree, options_entry, entry, discr_entry, options_cmp);

pub unsafe fn options_cmp(
    lhs: *const options_entry,
    rhs: *const options_entry,
) -> Ordering {
    unsafe { i32_to_ordering(libc::strcmp((*lhs).name, (*rhs).name)) }
}

pub unsafe fn options_map_name(name: *const c_char) -> *const c_char {
    unsafe {
        let mut map = &raw const options_other_names as *const options_name_map;
        while !(*map).from.is_null() {
            if libc::strcmp((*map).from, name) == 0 {
                return (*map).to;
            }
            map = map.add(1);
        }
        name
    }
}

pub unsafe fn options_parent_table_entry(
    oo: *mut options,
    s: *const c_char,
) -> *const options_table_entry {
    unsafe {
        if (*oo).parent.is_null() {
            fatalx_!("no parent options for {}", _s(s));
        }

        let o = options_get((*oo).parent, s);
        if o.is_null() {
            fatalx_!("{} not in parent options", _s(s));
        }

        (*o).tableentry
    }
}

pub unsafe fn options_value_free(o: *const options_entry, ov: *mut options_value) {
    unsafe {
        if OPTIONS_IS_STRING(o) {
            free_((*ov).string);
        }
        if OPTIONS_IS_COMMAND(o) && !(*ov).cmdlist.is_null() {
            cmd_list_free((*ov).cmdlist);
        }
    }
}

pub unsafe fn options_value_to_string(
    o: *mut options_entry,
    ov: *mut options_value,
    numeric: i32,
) -> *mut c_char {
    unsafe {
        let mut s: *mut c_char = null_mut();

        if OPTIONS_IS_COMMAND(o) {
            return cmd_list_print(&mut *(*ov).cmdlist, 0);
        }

        if OPTIONS_IS_NUMBER(o) {
            s = match (*(*o).tableentry).type_ {
                options_table_type::OPTIONS_TABLE_NUMBER => {
                    format_nul!("{}", (*ov).number)
                }
                options_table_type::OPTIONS_TABLE_KEY => {
                    xstrdup(key_string_lookup_key((*ov).number as u64, 0)).as_ptr()
                }
                options_table_type::OPTIONS_TABLE_COLOUR => {
                    xstrdup(colour_tostring((*ov).number as i32)).as_ptr()
                }
                options_table_type::OPTIONS_TABLE_FLAG => {
                    if numeric != 0 {
                        format_nul!("{}", (*ov).number)
                    } else {
                        xstrdup(if (*ov).number != 0 {
                            c"on".as_ptr()
                        } else {
                            c"off".as_ptr()
                        })
                        .as_ptr()
                    }
                }
                options_table_type::OPTIONS_TABLE_CHOICE => {
                    xstrdup(*(*(*o).tableentry).choices.add((*ov).number as usize)).as_ptr()
                }
                _ => {
                    fatalx(c"not a number option type");
                }
            };
            return s;
        }

        if OPTIONS_IS_STRING(o) {
            return xstrdup((*ov).string).as_ptr();
        }

        xstrdup(c"".as_ptr()).as_ptr()
    }
}

pub unsafe fn options_create(parent: *mut options) -> *mut options {
    unsafe {
        let oo = xcalloc1::<options>() as *mut options;
        rb_init(&raw mut (*oo).tree);
        (*oo).parent = parent;
        oo
    }
}

pub unsafe fn options_free(oo: *mut options) {
    unsafe {
        for o in rb_foreach(&raw mut (*oo).tree) {
            options_remove(o.as_ptr());
        }
        free_(oo);
    }
}

pub unsafe fn options_get_parent(oo: *mut options) -> *mut options {
    unsafe { (*oo).parent }
}

pub unsafe fn options_set_parent(oo: *mut options, parent: *mut options) {
    unsafe {
        (*oo).parent = parent;
    }
}

pub unsafe fn options_first(oo: *mut options) -> *mut options_entry {
    unsafe { rb_min(&raw mut (*oo).tree) }
}

pub unsafe fn options_next(o: *mut options_entry) -> *mut options_entry {
    unsafe { rb_next(o) }
}

pub unsafe fn options_get_only(
    oo: *mut options,
    name: *const c_char,
) -> *mut options_entry {
    unsafe {
        let mut o = options_entry {
            name,
            ..unsafe { zeroed() } // TODO use uninit
        };

        let found = rb_find(&raw mut (*oo).tree, &raw const o);
        if found.is_null() {
            o.name = options_map_name(name);
            rb_find(&raw mut (*oo).tree, &o)
        } else {
            found
        }
    }
}

pub unsafe fn options_get(
    mut oo: *mut options,
    name: *const c_char,
) -> *mut options_entry {
    unsafe {
        let mut o = options_get_only(oo, name);
        while o.is_null() {
            oo = (*oo).parent;
            if oo.is_null() {
                break;
            }
            o = options_get_only(oo, name);
        }
        o
    }
}

pub unsafe fn options_get_(mut oo: *mut options, name: &CStr) -> *mut options_entry {
    unsafe {
        let mut o;
        while {
            o = options_get_only(oo, name.as_ptr());
            o.is_null()
        } {
            oo = (*oo).parent;
            if oo.is_null() {
                break;
            }
        }
        o
    }
}

pub unsafe fn options_empty(
    oo: *mut options,
    oe: *const options_table_entry,
) -> *mut options_entry {
    unsafe {
        let o = options_add(oo, (*oe).name);
        (*o).tableentry = oe;

        if (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
            rb_init(&raw mut (*o).value.array);
        }
        o
    }
}

pub unsafe fn options_default(
    oo: *mut options,
    oe: *const options_table_entry,
) -> *mut options_entry {
    unsafe {
        let o = options_empty(oo, oe);
        let ov = &raw mut (*o).value;

        if (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
            if (*oe).default_arr.is_null() {
                options_array_assign(o, (*oe).default_str, null_mut());
                return o;
            }
            let mut i = 0usize;
            while !(*(*oe).default_arr.add(i)).is_null() {
                options_array_set(o, i as u32, *(*oe).default_arr.add(i), 0, null_mut());
                i += 1;
            }
            return o;
        }

        match (*oe).type_ {
            options_table_type::OPTIONS_TABLE_STRING => {
                (*ov).string = xstrdup((*oe).default_str).as_ptr();
            }
            _ => {
                (*ov).number = (*oe).default_num;
            }
        }
        o
    }
}

pub unsafe fn options_default_to_string(
    oe: *const options_table_entry,
) -> NonNull<c_char> {
    unsafe {
        match (*oe).type_ {
            options_table_type::OPTIONS_TABLE_STRING
            | options_table_type::OPTIONS_TABLE_COMMAND => xstrdup((*oe).default_str),
            options_table_type::OPTIONS_TABLE_NUMBER => {
                NonNull::new(format_nul!("{}", (*oe).default_num)).unwrap()
            }
            options_table_type::OPTIONS_TABLE_KEY => {
                xstrdup(key_string_lookup_key((*oe).default_num as u64, 0))
            }
            options_table_type::OPTIONS_TABLE_COLOUR => {
                xstrdup(colour_tostring((*oe).default_num as i32))
            }
            options_table_type::OPTIONS_TABLE_FLAG => xstrdup(if (*oe).default_num != 0 {
                c"on".as_ptr()
            } else {
                c"off".as_ptr()
            } as *const c_char),
            options_table_type::OPTIONS_TABLE_CHOICE => {
                xstrdup(*(*oe).choices.add((*oe).default_num as usize))
            }
        }
    }
}

unsafe fn options_add(oo: *mut options, name: *const c_char) -> *mut options_entry {
    unsafe {
        let mut o = options_get_only(oo, name);
        if !o.is_null() {
            options_remove(o);
        }

        o = xcalloc1::<options_entry>() as *mut options_entry;
        (*o).owner = oo;
        (*o).name = xstrdup(name).as_ptr();

        rb_insert(&raw mut (*oo).tree, o);
        o
    }
}

pub unsafe fn options_remove(o: *mut options_entry) {
    unsafe {
        let oo = (*o).owner;

        if options_is_array(o) != 0 {
            options_array_clear(o);
        } else {
            options_value_free(o, &mut (*o).value);
        }
        rb_remove(&mut (*oo).tree, o);
        free_((*o).name.cast_mut()); // TODO cast away const
        free_(o);
    }
}

pub unsafe fn options_name(o: *mut options_entry) -> *const c_char {
    unsafe { (*o).name }
}

pub unsafe fn options_owner(o: *mut options_entry) -> *mut options {
    unsafe { (*o).owner }
}

pub unsafe fn options_table_entry(o: *mut options_entry) -> *const options_table_entry {
    unsafe { (*o).tableentry }
}

unsafe fn options_array_item(o: *mut options_entry, idx: c_uint) -> *mut options_array_item {
    unsafe {
        let mut a = options_array_item {
            index: idx,
            ..unsafe { zeroed() } // TODO use uninit
        };
        rb_find(&raw mut (*o).value.array, &raw mut a)
    }
}

unsafe fn options_array_new(o: *mut options_entry, idx: c_uint) -> *mut options_array_item {
    unsafe {
        let a = xcalloc1::<options_array_item>() as *mut options_array_item;
        (*a).index = idx;
        rb_insert(&mut (*o).value.array, a);
        a
    }
}

unsafe fn options_array_free(o: *mut options_entry, a: *mut options_array_item) {
    unsafe {
        options_value_free(o, &mut (*a).value);
        rb_remove(&mut (*o).value.array, a);
        free_(a);
    }
}

pub unsafe fn options_array_clear(o: *mut options_entry) {
    unsafe {
        if options_is_array(o) == 0 {
            return;
        }

        let mut a = rb_min(&raw mut (*o).value.array);
        while !a.is_null() {
            let next: *mut options_array_item = rb_next(a);
            options_array_free(o, a);
            a = next;
        }
    }
}

pub unsafe fn options_array_get(o: *mut options_entry, idx: u32) -> *mut options_value {
    unsafe {
        if options_is_array(o) == 0 {
            return null_mut();
        }
        let a = options_array_item(o, idx);
        if a.is_null() {
            return null_mut();
        }
        &raw mut (*a).value
    }
}

pub unsafe fn options_array_set(
    o: *mut options_entry,
    idx: u32,
    value: *const c_char,
    append: i32,
    cause: *mut *mut c_char,
) -> i32 {
    unsafe {
        if !OPTIONS_IS_ARRAY(o) {
            if !cause.is_null() {
                *cause = xstrdup(c"not an array".as_ptr()).as_ptr();
            }
            return -1;
        }

        if value.is_null() {
            let a = options_array_item(o, idx);
            if !a.is_null() {
                options_array_free(o, a);
            }
            return 0;
        }

        if OPTIONS_IS_COMMAND(o) {
            let cmdlist = match cmd_parse_from_string(cstr_to_str(value), None) {
                Err(error) => {
                    if !cause.is_null() {
                        *cause = error;
                    } else {
                        free_(error);
                    }
                    return -1;
                }
                Ok(cmdlist) => cmdlist,
            };

            let mut a = options_array_item(o, idx);
            if a.is_null() {
                a = options_array_new(o, idx);
            } else {
                options_value_free(o, &raw mut (*a).value);
            }
            (*a).value.cmdlist = cmdlist;
            return 0;
        }

        if OPTIONS_IS_STRING(o) {
            let mut a = options_array_item(o, idx);
            let new = if !a.is_null() && append != 0 {
                format_nul!("{}{}", _s((*a).value.string), _s(value))
            } else {
                xstrdup(value).as_ptr()
            };

            if a.is_null() {
                a = options_array_new(o, idx);
            } else {
                options_value_free(o, &mut (*a).value);
            }
            (*a).value.string = new;
            return 0;
        }

        if !(*o).tableentry.is_null()
            && (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_COLOUR
        {
            let number = colour_fromstring(value);
            if number == -1 {
                *cause = format_nul!("bad colour: {}", _s(value));
                return -1;
            }
            let mut a = options_array_item(o, idx);
            if a.is_null() {
                a = options_array_new(o, idx);
            } else {
                options_value_free(o, &raw mut (*a).value);
            }
            (*a).value.number = number as i64;
            return 0;
        }

        if !cause.is_null() {
            *cause = xstrdup(c"wrong array type".as_ptr()).as_ptr();
        }
        -1
    }
}

pub unsafe fn options_array_assign(
    o: *mut options_entry,
    s: *const c_char,
    cause: *mut *mut c_char,
) -> i32 {
    unsafe {
        let mut separator = (*(*o).tableentry).separator;
        if separator.is_null() {
            separator = c" ,".as_ptr();
        }
        if *separator == 0 {
            if *s == 0 {
                return 0;
            }
            let mut i = 0;
            while i < u32::MAX {
                if options_array_item(o, i).is_null() {
                    break;
                }
                i += 1;
            }
            return options_array_set(o, i, s, 0, cause);
        }

        if *s == 0 {
            return 0;
        }
        let copy = xstrdup(s).as_ptr();
        let mut string = copy;
        while let Some(next) = NonNull::new(strsep(&raw mut string, separator)) {
            let next = next.as_ptr();
            if *next == 0 {
                continue;
            }
            let mut i = 0;
            while i < u32::MAX {
                if options_array_item(o, i).is_null() {
                    break;
                }
                i += 1;
            }
            if i == u32::MAX {
                break;
            }
            if options_array_set(o, i, next, 0, cause) != 0 {
                free_(copy);
                return -1;
            }
        }
        free_(copy);
        0
    }
}

pub unsafe fn options_array_first(o: *mut options_entry) -> *mut options_array_item {
    unsafe {
        if !OPTIONS_IS_ARRAY(o) {
            return null_mut();
        }
        rb_min(&raw mut (*o).value.array)
    }
}

pub unsafe fn options_array_next(a: *mut options_array_item) -> *mut options_array_item {
    unsafe { rb_next(a) }
}

pub unsafe fn options_array_item_index(a: *mut options_array_item) -> u32 {
    unsafe { (*a).index }
}

pub unsafe fn options_array_item_value(
    a: *mut options_array_item,
) -> *mut options_value {
    unsafe { &raw mut (*a).value }
}

pub unsafe fn options_is_array(o: *mut options_entry) -> i32 {
    unsafe { OPTIONS_IS_ARRAY(o) as i32 }
}

pub unsafe fn options_is_string(o: *mut options_entry) -> i32 {
    unsafe { OPTIONS_IS_STRING(o) as i32 }
}

pub unsafe fn options_to_string(
    o: *mut options_entry,
    idx: i32,
    numeric: i32,
) -> *mut c_char {
    unsafe {
        if OPTIONS_IS_ARRAY(o) {
            if idx == -1 {
                let mut result = null_mut();
                let mut last: *mut i8 = null_mut();

                let mut a = rb_min(&raw mut (*o).value.array);
                while !a.is_null() {
                    let next = options_value_to_string(
                        o,
                        &raw mut (*a.cast::<options_array_item>()).value,
                        numeric,
                    );

                    if last.is_null() {
                        result = next;
                    } else {
                        let mut new_result = format_nul!("{} {}", _s(last), _s(next));
                        free_(last);
                        free_(next);
                        result = new_result;
                    }
                    last = result;

                    a = rb_next(a);
                }

                if result.is_null() {
                    return xstrdup(c"".as_ptr()).as_ptr();
                }
                return result;
            }

            let a = options_array_item(o, idx as u32);
            if a.is_null() {
                return xstrdup(c"".as_ptr()).as_ptr();
            }
            return options_value_to_string(o, &raw mut (*a).value, numeric);
        }

        options_value_to_string(o, &raw mut (*o).value, numeric)
    }
}

pub unsafe fn options_parse(name: *const c_char, idx: *mut i32) -> *mut c_char {
    unsafe {
        if *name == 0 {
            return null_mut();
        }

        let copy = xstrdup(name).as_ptr();
        let cp = strchr(copy, b'[' as i32);

        if cp.is_null() {
            *idx = -1;
            return copy;
        }

        let end = strchr(cp.offset(1), b']' as i32);
        if end.is_null() || *end.offset(1) != 0 || isdigit(*end.offset(-1) as i32) == 0 {
            free_(copy);
            return null_mut();
        }

        let mut parsed_idx = 0;
        if sscanf(cp, c"[%d]".as_ptr(), &mut parsed_idx) != 1 || parsed_idx < 0 {
            free_(copy);
            return null_mut();
        }

        *idx = parsed_idx;
        *cp = 0;
        copy
    }
}

pub unsafe fn options_parse_get(
    oo: *mut options,
    s: *const c_char,
    idx: *mut i32,
    only: i32,
) -> *mut options_entry {
    unsafe {
        let name = options_parse(s, idx);
        if name.is_null() {
            return null_mut();
        }

        let o = if only != 0 {
            options_get_only(oo, name)
        } else {
            options_get(oo, name)
        };

        free_(name);
        o
    }
}

pub unsafe fn options_match(
    s: *const c_char,
    idx: *mut i32,
    ambiguous: *mut i32,
) -> *mut c_char {
    unsafe {
        let parsed = options_parse(s, idx);
        if parsed.is_null() {
            return null_mut();
        }

        if *parsed == b'@' as i8 {
            *ambiguous = 0;
            return parsed;
        }

        let name = options_map_name(parsed);
        let namelen = strlen(name);

        let mut found: *const options_table_entry = null();
        let mut oe = &raw const options_table as *const options_table_entry;

        while !(*oe).name.is_null() {
            if strcmp((*oe).name, name) == 0 {
                found = oe;
                break;
            }
            if strncmp((*oe).name, name, namelen) == 0 {
                if !found.is_null() {
                    *ambiguous = 1;
                    free_(parsed);
                    return null_mut();
                }
                found = oe;
            }
            oe = oe.add(1);
        }

        free_(parsed);
        if found.is_null() {
            *ambiguous = 0;
            return null_mut();
        }

        xstrdup((*found).name).as_ptr()
    }
}

pub unsafe fn options_match_get(
    oo: *mut options,
    s: *const c_char,
    idx: *mut i32,
    only: i32,
    ambiguous: *mut i32,
) -> *mut options_entry {
    unsafe {
        let name = options_match(s, idx, ambiguous);
        if name.is_null() {
            return null_mut();
        }

        *ambiguous = 0;
        let o = if only != 0 {
            options_get_only(oo, name)
        } else {
            options_get(oo, name)
        };

        free_(name);
        o
    }
}

pub unsafe fn options_get_string(
    oo: *mut options,
    name: *const c_char,
) -> *const c_char {
    unsafe {
        let o = options_get(oo, name);
        if o.is_null() {
            fatalx_!("missing option {}", _s(name));
        }
        if !OPTIONS_IS_STRING(o) {
            fatalx_!("option {} is not a string", _s(name));
        }
        (*o).value.string
    }
}

pub unsafe fn options_get_string_(oo: *mut options, name: &CStr) -> *const c_char {
    unsafe {
        let o = options_get_(oo, name);
        if o.is_null() {
            fatalx_!("missing option {}", _s(name.as_ptr()));
        }
        if !OPTIONS_IS_STRING(o) {
            fatalx_!("option {} is not a string", _s(name.as_ptr()));
        }
        (*o).value.string
    }
}

pub unsafe fn options_get_number(oo: *mut options, name: *const c_char) -> i64 {
    unsafe {
        let o = options_get(oo, name);
        if o.is_null() {
            fatalx_!("missing option {}", _s(name));
        }
        if !OPTIONS_IS_NUMBER(o) {
            fatalx_!("option {} is not a number", _s(name));
        }
        (*o).value.number
    }
}

pub unsafe fn options_get_number_(oo: *mut options, name: &CStr) -> i64 {
    unsafe {
        let o = options_get_(oo, name);
        if o.is_null() {
            fatalx_!("missing option {}", _s(name.as_ptr()));
        }
        if !OPTIONS_IS_NUMBER(o) {
            fatalx_!("option {} is not a number", _s(name.as_ptr()));
        }
        (*o).value.number
    }
}

macro_rules! options_set_string {
   ($oo:expr, $name:expr, $append:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::options_::options_set_string_($oo, $name, $append, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use options_set_string;

pub unsafe fn options_set_string_(
    oo: *mut options,
    name: *const c_char,
    append: c_int,
    args: std::fmt::Arguments,
) -> *mut options_entry {
    unsafe {
        let mut separator = c"".as_ptr();
        let mut value: *mut c_char = null_mut();

        let mut s = args.to_string();
        s.push('\0');
        let s = s.leak().as_mut_ptr().cast();

        let mut o = options_get_only(oo, name);
        if !o.is_null() && append != 0 && OPTIONS_IS_STRING(o) {
            if *name != b'@' as c_char {
                separator = (*(*o).tableentry).separator;
                if separator.is_null() {
                    separator = c"".as_ptr();
                }
            }
            value = format_nul!("{}{}{}", _s((*o).value.string), _s(separator), _s(s),);
            free_(s);
        } else {
            value = s;
        }

        if o.is_null() && *name == b'@' as c_char {
            o = options_add(oo, name);
        } else if o.is_null() {
            o = options_default(oo, options_parent_table_entry(oo, name));
            if o.is_null() {
                return null_mut();
            }
        }

        if !OPTIONS_IS_STRING(o) {
            panic!("option {} is not a string", _s(name));
        }
        free_((*o).value.string);
        (*o).value.string = value;
        (*o).cached = 0;
        o
    }
}

pub unsafe fn options_set_number(
    oo: *mut options,
    name: *const c_char,
    value: i64,
) -> *mut options_entry {
    unsafe {
        if *name == b'@' as c_char {
            panic!("user option {} must be a string", _s(name));
        }

        let mut o = options_get_only(oo, name);
        if o.is_null() {
            o = options_default(oo, options_parent_table_entry(oo, name));
            if o.is_null() {
                return null_mut();
            }
        }

        if !OPTIONS_IS_NUMBER(o) {
            panic!("option {} is not a number", _s(name));
        }
        (*o).value.number = value;
        o
    }
}

pub unsafe fn options_scope_from_name(
    args: *mut args,
    window: i32,
    name: *const c_char,
    fs: *mut cmd_find_state,
    oo: *mut *mut options,
    cause: *mut *mut c_char,
) -> i32 {
    unsafe {
        let s = (*fs).s;
        let wl = (*fs).wl;
        let wp = (*fs).wp;
        let target = args_get_(args, 't');
        let mut scope = OPTIONS_TABLE_NONE;

        if *name == b'@' as c_char {
            return options_scope_from_flags(args, window, fs, oo, cause);
        }

        let mut oe = &raw const options_table as *const options_table_entry;
        while !(*oe).name.is_null() {
            if strcmp((*oe).name, name) == 0 {
                break;
            }
            oe = oe.add(1);
        }

        if (*oe).name.is_null() {
            *cause = format_nul!("unknown option: {}", _s(name));
            return OPTIONS_TABLE_NONE;
        }

        const OPTIONS_TABLE_WINDOW_AND_PANE: i32 = OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE;
        match (*oe).scope {
            OPTIONS_TABLE_SERVER => {
                *oo = global_options;
                scope = OPTIONS_TABLE_SERVER;
            }
            OPTIONS_TABLE_SESSION => {
                if args_has_(args, 'g') {
                    *oo = global_s_options;
                    scope = OPTIONS_TABLE_SESSION;
                } else if s.is_null() && !target.is_null() {
                    *cause = format_nul!("no such session: {}", _s(target));
                } else if s.is_null() {
                    *cause = format_nul!("no current session");
                } else {
                    *oo = (*s).options;
                    scope = OPTIONS_TABLE_SESSION;
                }
            }
            OPTIONS_TABLE_WINDOW_AND_PANE => {
                if args_has_(args, 'p') {
                    if wp.is_null() && !target.is_null() {
                        *cause = format_nul!("no such pane: {}", _s(target));
                    } else if wp.is_null() {
                        *cause = format_nul!("no current pane");
                    } else {
                        *oo = (*wp).options;
                        scope = OPTIONS_TABLE_PANE;
                    }
                } else {
                    // FALLTHROUGH same as OPTIONS_TABLE_WINDOW case
                    if args_has_(args, 'g') {
                        *oo = global_w_options;
                        scope = OPTIONS_TABLE_WINDOW;
                    } else if wl.is_null() && !target.is_null() {
                        *cause = format_nul!("no such window: {}", _s(target));
                    } else if wl.is_null() {
                        *cause = format_nul!("no current window");
                    } else {
                        *oo = (*(*wl).window).options;
                        scope = OPTIONS_TABLE_WINDOW;
                    }
                }
            }
            OPTIONS_TABLE_WINDOW => {
                if args_has_(args, 'g') {
                    *oo = global_w_options;
                    scope = OPTIONS_TABLE_WINDOW;
                } else if wl.is_null() && !target.is_null() {
                    *cause = format_nul!("no such window: {}", _s(target));
                } else if wl.is_null() {
                    *cause = format_nul!("no current window");
                } else {
                    *oo = (*(*wl).window).options;
                    scope = OPTIONS_TABLE_WINDOW;
                }
            }
            _ => {}
        }
        scope
    }
}

pub unsafe fn options_scope_from_flags(
    args: *mut args,
    window: i32,
    fs: *mut cmd_find_state,
    oo: *mut *mut options,
    cause: *mut *mut c_char,
) -> i32 {
    unsafe {
        let s = (*fs).s;
        let wl = (*fs).wl;
        let wp = (*fs).wp;
        let target = args_get_(args, 't');

        if args_has_(args, 's') {
            *oo = global_options;
            return OPTIONS_TABLE_SERVER;
        }

        if args_has_(args, 'p') {
            if wp.is_null() {
                if !target.is_null() {
                    *cause = format_nul!("no such pane: {}", _s(target));
                } else {
                    *cause = format_nul!("no current pane");
                }
                return OPTIONS_TABLE_NONE;
            }
            *oo = (*wp).options;
            OPTIONS_TABLE_PANE
        } else if window != 0 || args_has_(args, 'w') {
            if args_has_(args, 'g') {
                *oo = global_w_options;
                return OPTIONS_TABLE_WINDOW;
            }
            if wl.is_null() {
                if !target.is_null() {
                    *cause = format_nul!("no such window: {}", _s(target));
                } else {
                    *cause = format_nul!("no current window");
                }
                return OPTIONS_TABLE_NONE;
            }
            *oo = (*(*wl).window).options;
            OPTIONS_TABLE_WINDOW
        } else {
            if args_has_(args, 'g') {
                *oo = global_s_options;
                return OPTIONS_TABLE_SESSION;
            }
            if s.is_null() {
                if !target.is_null() {
                    *cause = format_nul!("no such session: {}", _s(target));
                } else {
                    *cause = format_nul!("no current session");
                }
                return OPTIONS_TABLE_NONE;
            }
            *oo = (*s).options;
            OPTIONS_TABLE_SESSION
        }
    }
}

pub unsafe fn options_string_to_style(
    oo: *mut options,
    name: *const c_char,
    ft: *mut format_tree,
) -> *mut style {
    let __func__ = c"options_string_to_style".as_ptr();
    unsafe {
        let o = options_get(oo, name);
        if o.is_null() || !OPTIONS_IS_STRING(o) {
            return null_mut();
        }

        if (*o).cached != 0 {
            return &mut (*o).style;
        }
        let s = (*o).value.string;
        log_debug!("{}: {} is '{}'", _s(__func__), _s(name), _s(s));

        style_set(&mut (*o).style, &grid_default_cell);
        (*o).cached = if strstr(s, c"#{".as_ptr()).is_null() {
            1
        } else {
            0
        };

        if !ft.is_null() && (*o).cached == 0 {
            let expanded = format_expand(ft, s);
            if style_parse(&mut (*o).style, &grid_default_cell, expanded) != 0 {
                free_(expanded);
                return null_mut();
            }
            free_(expanded);
        } else if style_parse(&mut (*o).style, &grid_default_cell, s) != 0 {
            return null_mut();
        }
        &mut (*o).style
    }
}

unsafe fn options_from_string_check(
    oe: *const options_table_entry,
    value: *const c_char,
    cause: *mut *mut c_char,
) -> c_int {
    unsafe {
        let mut sy: style = std::mem::zeroed();

        if oe.is_null() {
            return 0;
        }
        if streq_((*oe).name, "default-shell") && !checkshell(value) {
            *cause = format_nul!("not a suitable shell: {}", _s(value));
            return -1;
        }
        if !(*oe).pattern.is_null() && fnmatch((*oe).pattern, value, 0) != 0 {
            *cause = format_nul!("value is invalid: {}", _s(value));
            return -1;
        }
        if ((*oe).flags & OPTIONS_TABLE_IS_STYLE) != 0
            && strstr(value, c"#{".as_ptr()).is_null()
            && style_parse(&mut sy, &grid_default_cell, value) != 0
        {
            *cause = format_nul!("invalid style: {}", _s(value));
            return -1;
        }
        0
    }
}

unsafe fn options_from_string_flag(
    oo: *mut options,
    name: *const c_char,
    value: *const c_char,
    cause: *mut *mut c_char,
) -> c_int {
    unsafe {
        let flag = if value.is_null() || *value == 0 {
            !options_get_number(oo, name)
        } else if streq_(value, "1")
            || strcasecmp(value, c"on".as_ptr()) == 0
            || strcasecmp(value, c"yes".as_ptr()) == 0
        {
            1
        } else if streq_(value, "0")
            || strcasecmp(value, c"off".as_ptr()) == 0
            || strcasecmp(value, c"no".as_ptr()) == 0
        {
            0
        } else {
            *cause = format_nul!("bad value: {}", _s(value));
            return -1;
        };
        options_set_number(oo, name, flag);
        0
    }
}

pub unsafe fn options_find_choice(
    oe: *const options_table_entry,
    value: *const c_char,
    cause: *mut *mut c_char,
) -> c_int {
    unsafe {
        let mut n = 0;
        let mut choice = -1;
        let mut cp = (*oe).choices;

        while !(*cp).is_null() {
            if strcmp(*cp, value) == 0 {
                choice = n;
            }
            n += 1;
            cp = cp.add(1);
        }
        if choice == -1 {
            *cause = format_nul!("unknown value: {}", _s(value));
            return -1;
        }
        choice
    }
}

unsafe fn options_from_string_choice(
    oe: *const options_table_entry,
    oo: *mut options,
    name: *const c_char,
    value: *const c_char,
    cause: *mut *mut c_char,
) -> c_int {
    unsafe {
        let choice = if value.is_null() {
            let mut choice = options_get_number(oo, name);
            if choice < 2 {
                choice = !choice;
            }
            choice
        } else {
            let choice = options_find_choice(oe, value, cause) as i64;
            if choice < 0 {
                return -1;
            }
            choice
        };
        options_set_number(oo, name, choice);
        0
    }
}

pub unsafe fn options_from_string(
    oo: *mut options,
    oe: *const options_table_entry,
    name: *const c_char,
    value: *const c_char,
    append: c_int,
    cause: *mut *mut c_char,
) -> c_int {
    unsafe {
        let mut errstr: *const c_char;
        let new: *const c_char;
        let old: *mut c_char;
        let key: key_code;

        let type_: options_table_type = if !oe.is_null() {
            if value.is_null()
                && (*oe).type_ != options_table_type::OPTIONS_TABLE_FLAG
                && (*oe).type_ != options_table_type::OPTIONS_TABLE_CHOICE
            {
                *cause = format_nul!("empty value");
                return -1;
            }
            (*oe).type_
        } else {
            if *name != b'@' as c_char {
                *cause = format_nul!("bad option name");
                return -1;
            }
            options_table_type::OPTIONS_TABLE_STRING
        };

        match type_ {
            options_table_type::OPTIONS_TABLE_STRING => {
                old = xstrdup(options_get_string(oo, name)).as_ptr();
                options_set_string!(oo, name, append, "{}", _s(value));

                new = options_get_string(oo, name);
                if options_from_string_check(oe, new, cause) != 0 {
                    options_set_string!(oo, name, 0, "{}", _s(old));
                    free_(old);
                    return -1;
                }
                free_(old);
                return 0;
            }

            options_table_type::OPTIONS_TABLE_NUMBER => {
                match strtonum(value, (*oe).minimum as i64, (*oe).maximum as i64) {
                    Ok(number) => {
                        options_set_number(oo, name, number);
                        return 0;
                    }
                    Err(errstr) => {
                        *cause = format_nul!("value is {}: {}", _s(errstr.as_ptr()), _s(value));
                        return -1;
                    }
                }
            }

            options_table_type::OPTIONS_TABLE_KEY => {
                key = key_string_lookup_string(value);
                if key == KEYC_UNKNOWN {
                    *cause = format_nul!("bad key: {}", _s(value));
                    return -1;
                }
                options_set_number(oo, name, key as i64);
                return 0;
            }

            options_table_type::OPTIONS_TABLE_COLOUR => {
                let number = colour_fromstring(value) as i64;
                if number == -1 {
                    *cause = format_nul!("bad colour: {}", _s(value));
                    return -1;
                }
                options_set_number(oo, name, number);
                return 0;
            }

            options_table_type::OPTIONS_TABLE_FLAG => {
                return options_from_string_flag(oo, name, value, cause);
            }

            options_table_type::OPTIONS_TABLE_CHOICE => {
                return options_from_string_choice(oe, oo, name, value, cause);
            }

            options_table_type::OPTIONS_TABLE_COMMAND => {}

            _ => {}
        }
        -1
    }
}

pub unsafe fn options_push_changes(name: *const c_char) {
    let __func__ = c"options_push_changes".as_ptr();
    unsafe {
        let mut loop_: *mut client;
        let mut s: *mut session;
        let mut w: *mut window;
        let mut wp: *mut window_pane;

        log_debug!("{}: {}", _s(__func__), _s(name));

        if streq_(name, "automatic-rename") {
            for w in rb_foreach(&raw mut windows).map(NonNull::as_ptr) {
                if (*w).active.is_null() {
                    continue;
                }
                if options_get_number((*w).options, name) != 0 {
                    (*(*w).active).flags |= window_pane_flags::PANE_CHANGED;
                }
            }
        }

        if streq_(name, "cursor-colour") {
            for wp in rb_foreach(&raw mut all_window_panes) {
                window_pane_default_cursor(wp.as_ptr());
            }
        }

        if streq_(name, "cursor-style") {
            for wp in rb_foreach(&raw mut all_window_panes) {
                window_pane_default_cursor(wp.as_ptr());
            }
        }

        if streq_(name, "fill-character") {
            for w in rb_foreach(&raw mut windows) {
                window_set_fill_character(w);
            }
        }

        if streq_(name, "key-table") {
            for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
                server_client_set_key_table(loop_, null_mut());
            }
        }

        if streq_(name, "user-keys") {
            for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
                if (*loop_).tty.flags.intersects(tty_flags::TTY_OPENED) {
                    tty_keys_build(&mut (*loop_).tty);
                }
            }
        }

        if streq_(name, "status") || streq_(name, "status-interval") {
            status_timer_start_all();
        }

        if streq_(name, "monitor-silence") {
            alerts_reset_all();
        }

        if streq_(name, "window-style") || streq_(name, "window-active-style") {
            for wp in rb_foreach(&raw mut all_window_panes) {
                (*wp.as_ptr()).flags |= window_pane_flags::PANE_STYLECHANGED;
            }
        }

        if streq_(name, "pane-colours") {
            for wp in rb_foreach(&raw mut all_window_panes).map(NonNull::as_ptr) {
                colour_palette_from_option(&raw mut (*wp).palette, (*wp).options);
            }
        }

        if streq_(name, "pane-border-status") {
            for w in rb_foreach(&raw mut windows) {
                layout_fix_panes(w.as_ptr(), null_mut());
            }
        }

        for s in rb_foreach(&raw mut sessions) {
            status_update_cache(s.as_ptr());
        }

        recalculate_sizes();

        for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if !(*loop_).session.is_null() {
                server_redraw_client(loop_);
            }
        }
    }
}

pub unsafe fn options_remove_or_default(
    o: *mut options_entry,
    idx: i32,
    cause: *mut *mut c_char,
) -> i32 {
    unsafe {
        let oo = (*o).owner;

        if idx == -1 {
            if !(*o).tableentry.is_null()
                && (oo == global_options || oo == global_s_options || oo == global_w_options)
            {
                options_default(oo, (*o).tableentry);
            } else {
                options_remove(o);
            }
        } else if options_array_set(o, idx as u32, null(), 0, cause) != 0 {
            return -1;
        }
        0
    }
}
