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
use crate::libc::{fnmatch, isdigit, sscanf, strchr, strcmp, strncmp, strstr};
use crate::options_table::OPTIONS_OTHER_NAMES_STR;
use crate::*;

// Option handling; each option has a name, type and value and is stored in a red-black tree.

#[repr(C)]
#[derive(Copy, Clone)]
pub struct options_array_item {
    pub index: u32,
    pub value: options_value,
    pub entry: rb_entry<options_array_item>,
}

pub fn options_array_cmp(a1: &options_array_item, a2: &options_array_item) -> cmp::Ordering {
    a1.index.cmp(&a2.index)
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
    pub name: *const u8,
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

#[expect(non_snake_case)]
#[inline]
pub unsafe fn OPTIONS_IS_STRING(o: *const options_entry) -> bool {
    unsafe {
        (*o).tableentry.is_null()
            || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_STRING
    }
}

#[expect(non_snake_case)]
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

#[expect(non_snake_case)]
#[inline]
pub unsafe fn OPTIONS_IS_COMMAND(o: *const options_entry) -> bool {
    unsafe {
        !(*o).tableentry.is_null()
            && (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_COMMAND
    }
}

#[expect(non_snake_case)]
#[inline]
pub unsafe fn OPTIONS_IS_ARRAY(o: *const options_entry) -> bool {
    unsafe {
        !(*o).tableentry.is_null() && ((*(*o).tableentry).flags & OPTIONS_TABLE_IS_ARRAY) != 0
    }
}

RB_GENERATE!(options_tree, options_entry, entry, discr_entry, options_cmp);

pub fn options_cmp(lhs: &options_entry, rhs: &options_entry) -> cmp::Ordering {
    unsafe { i32_to_ordering(libc::strcmp(lhs.name, rhs.name)) }
}

pub unsafe fn options_map_name(name: *const u8) -> *const u8 {
    unsafe {
        let mut map = &raw const OPTIONS_OTHER_NAMES as *const options_name_map;
        while !(*map).from.is_null() {
            if libc::strcmp((*map).from, name) == 0 {
                return (*map).to;
            }
            map = map.add(1);
        }
        name
    }
}

pub fn options_map_name_str(name: &str) -> &str {
    for map in &OPTIONS_OTHER_NAMES_STR {
        if map.from == name {
            return map.to;
        }
    }
    name
}

pub unsafe fn options_parent_table_entry(
    oo: *mut options,
    s: *const u8,
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
) -> *mut u8 {
    unsafe {
        if OPTIONS_IS_COMMAND(o) {
            return cmd_list_print(&mut *(*ov).cmdlist, 0);
        }

        if OPTIONS_IS_NUMBER(o) {
            let s = match (*(*o).tableentry).type_ {
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
                            c!("on")
                        } else {
                            c!("off")
                        })
                        .as_ptr()
                    }
                }
                options_table_type::OPTIONS_TABLE_CHOICE => {
                    xstrdup(*(*(*o).tableentry).choices.add((*ov).number as usize)).as_ptr()
                }
                _ => {
                    fatalx("not a number option type");
                }
            };
            return s;
        }

        if OPTIONS_IS_STRING(o) {
            return xstrdup((*ov).string).as_ptr();
        }

        xstrdup(c!("")).as_ptr()
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

pub unsafe fn options_get_only(oo: *mut options, name: *const u8) -> *mut options_entry {
    unsafe {
        let mut o = options_entry {
            name,
            ..zeroed() // TODO use uninit
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
pub unsafe fn options_get_only_(oo: *mut options, name: &str) -> *mut options_entry {
    unsafe {
        let found = rb_find_by(&raw mut (*oo).tree, |oe| {
            libc::strcmp_(oe.name, name).reverse()
        });
        if found.is_null() {
            let name = options_map_name_str(name);
            rb_find_by(&raw mut (*oo).tree, |oe| {
                libc::strcmp_(oe.name, name).reverse()
            })
        } else {
            found
        }
    }
}

pub unsafe fn options_get_only_const(oo: *const options, name: &str) -> *const options_entry {
    unsafe {
        let found = rb_find_by_const(&(*oo).tree, |oe| libc::strcmp_(oe.name, name).reverse());
        if found.is_null() {
            let name = options_map_name_str(name);
            rb_find_by_const(&(*oo).tree, |oe| libc::strcmp_(oe.name, name).reverse())
        } else {
            found
        }
    }
}

pub unsafe fn options_get(mut oo: *mut options, name: *const u8) -> *mut options_entry {
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

pub unsafe fn options_get_(mut oo: *mut options, name: &str) -> *mut options_entry {
    unsafe {
        let mut o;
        while {
            o = options_get_only_(oo, name);
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

pub unsafe fn options_get_const(mut oo: *const options, name: &str) -> *const options_entry {
    unsafe {
        let mut o;
        while {
            o = options_get_only_const(oo, name);
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
                _ = options_array_assign(o, (*oe).default_str.unwrap());
                return o;
            }
            let mut i = 0usize;
            while !(*(*oe).default_arr.add(i)).is_null() {
                _ = options_array_set(
                    o,
                    i as u32,
                    Some(cstr_to_str(*(*oe).default_arr.add(i))),
                    false,
                );
                i += 1;
            }
            return o;
        }

        match (*oe).type_ {
            options_table_type::OPTIONS_TABLE_STRING => {
                (*ov).string = xstrdup___((*oe).default_str);
            }
            _ => {
                (*ov).number = (*oe).default_num;
            }
        }
        o
    }
}

pub unsafe fn options_default_to_string(oe: *const options_table_entry) -> NonNull<u8> {
    unsafe {
        match (*oe).type_ {
            options_table_type::OPTIONS_TABLE_STRING
            | options_table_type::OPTIONS_TABLE_COMMAND => {
                NonNull::new_unchecked(xstrdup___((*oe).default_str))
            }
            options_table_type::OPTIONS_TABLE_NUMBER => {
                NonNull::new(format_nul!("{}", (*oe).default_num)).unwrap()
            }
            options_table_type::OPTIONS_TABLE_KEY => {
                xstrdup(key_string_lookup_key((*oe).default_num as u64, 0))
            }
            options_table_type::OPTIONS_TABLE_COLOUR => {
                xstrdup(colour_tostring((*oe).default_num as i32))
            }
            options_table_type::OPTIONS_TABLE_FLAG => xstrdup_(if (*oe).default_num != 0 {
                c"on"
            } else {
                c"off"
            }),
            options_table_type::OPTIONS_TABLE_CHOICE => {
                xstrdup(*(*oe).choices.add((*oe).default_num as usize))
            }
        }
    }
}

unsafe fn options_add(oo: *mut options, name: *const u8) -> *mut options_entry {
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

pub unsafe fn options_name(o: *mut options_entry) -> *const u8 {
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
            ..zeroed() // TODO use uninit
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
    value: Option<&str>,
    append: bool,
) -> Result<(), CString> {
    unsafe {
        if !OPTIONS_IS_ARRAY(o) {
            return Err(CString::new("not an array").unwrap());
        }

        let Some(value) = value else {
            let a = options_array_item(o, idx);
            if !a.is_null() {
                options_array_free(o, a);
            }
            return Ok(());
        };

        if OPTIONS_IS_COMMAND(o) {
            let cmdlist = match cmd_parse_from_string(value, None) {
                Err(error) => {
                    return Err(CString::from_raw(error.cast()));
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
            return Ok(());
        }

        if OPTIONS_IS_STRING(o) {
            let mut a = options_array_item(o, idx);
            let new = if !a.is_null() && append {
                format_nul!("{}{}", _s((*a).value.string), value)
            } else {
                xstrdup__(value)
            };

            if a.is_null() {
                a = options_array_new(o, idx);
            } else {
                options_value_free(o, &mut (*a).value);
            }
            (*a).value.string = new;
            return Ok(());
        }

        if !(*o).tableentry.is_null()
            && (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_COLOUR
        {
            let number = colour_fromstring_(value);
            if number == -1 {
                return Err(CString::new(format!("bad colour: {value}")).unwrap());
            }
            let mut a = options_array_item(o, idx);
            if a.is_null() {
                a = options_array_new(o, idx);
            } else {
                options_value_free(o, &raw mut (*a).value);
            }
            (*a).value.number = number as i64;
            return Ok(());
        }

        Err(CString::new("wrong array type").unwrap())
    }
}

// note one difference was that this function previously could avoid allocation on error
pub unsafe fn options_array_assign(o: *mut options_entry, s: &str) -> Result<(), CString> {
    unsafe {
        let mut separator = (*(*o).tableentry).separator;
        if separator.is_null() {
            separator = c!(" ,");
        }
        if *separator == 0 {
            if s.is_empty() {
                return Ok(());
            }
            let mut i = 0;
            while i < u32::MAX {
                if options_array_item(o, i).is_null() {
                    break;
                }
                i += 1;
            }
            return options_array_set(o, i, Some(s), false);
        }

        if s.is_empty() {
            return Ok(());
        }
        let copy = xstrdup__(s);
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
            if let Err(cause) = options_array_set(o, i, Some(cstr_to_str(next)), false) {
                free_(copy);
                return Err(cause);
            }
        }
        free_(copy);
        Ok(())
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

pub unsafe fn options_array_item_value(a: *mut options_array_item) -> *mut options_value {
    unsafe { &raw mut (*a).value }
}

pub unsafe fn options_is_array(o: *mut options_entry) -> i32 {
    unsafe { OPTIONS_IS_ARRAY(o) as i32 }
}

pub unsafe fn options_is_string(o: *mut options_entry) -> i32 {
    unsafe { OPTIONS_IS_STRING(o) as i32 }
}

pub unsafe fn options_to_string(o: *mut options_entry, idx: i32, numeric: i32) -> *mut u8 {
    unsafe {
        if OPTIONS_IS_ARRAY(o) {
            if idx == -1 {
                let mut result = null_mut();
                let mut last: *mut u8 = null_mut();

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
                        let new_result = format_nul!("{} {}", _s(last), _s(next));
                        free_(last);
                        free_(next);
                        result = new_result;
                    }
                    last = result;

                    a = rb_next(a);
                }

                if result.is_null() {
                    return xstrdup(c!("")).as_ptr();
                }
                return result;
            }

            let a = options_array_item(o, idx as u32);
            if a.is_null() {
                return xstrdup(c!("")).as_ptr();
            }
            return options_value_to_string(o, &raw mut (*a).value, numeric);
        }

        options_value_to_string(o, &raw mut (*o).value, numeric)
    }
}

pub unsafe fn options_parse(name: *const u8, idx: *mut i32) -> *mut u8 {
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
        if sscanf(cp.cast(), c"[%d]".as_ptr(), &mut parsed_idx) != 1 || parsed_idx < 0 {
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
    s: *const u8,
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

pub unsafe fn options_match(s: *const u8, idx: *mut i32, ambiguous: *mut i32) -> *mut u8 {
    unsafe {
        let parsed = options_parse(s, idx);
        if parsed.is_null() {
            return null_mut();
        }

        if *parsed == b'@' {
            *ambiguous = 0;
            return parsed;
        }

        let name = options_map_name(parsed);
        let namelen = strlen(name);

        let mut found: *const options_table_entry = null();
        let mut oe = &raw const OPTIONS_TABLE as *const options_table_entry;

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

#[expect(dead_code)]
unsafe fn options_match_get(
    oo: *mut options,
    s: *const u8,
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

pub unsafe fn options_get_string(oo: *mut options, name: *const u8) -> *const u8 {
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

pub unsafe fn options_get_string_(oo: *const options, name: &str) -> *const u8 {
    unsafe {
        let o = options_get_const(oo, name);
        if o.is_null() {
            fatalx_!("missing option {name}");
        }
        if !OPTIONS_IS_STRING(o) {
            fatalx_!("option {name} is not a string");
        }
        (*o).value.string
    }
}

pub unsafe fn options_get_number(oo: *mut options, name: *const u8) -> i64 {
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

pub unsafe fn options_get_number_(oo: *const options, name: &str) -> i64 {
    unsafe {
        let o = options_get_const(oo, name);
        if o.is_null() {
            fatalx_!("missing option {name}");
        }
        if !OPTIONS_IS_NUMBER(o) {
            fatalx_!("option {name} is not a number");
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
    name: *const u8,
    append: bool,
    args: std::fmt::Arguments,
) -> *mut options_entry {
    unsafe {
        let mut separator = c!("");
        let value: *mut u8;

        let mut s = args.to_string();
        s.push('\0');
        let s = s.leak().as_mut_ptr().cast();

        let mut o = options_get_only(oo, name);
        if !o.is_null() && append && OPTIONS_IS_STRING(o) {
            if *name != b'@' {
                separator = (*(*o).tableentry).separator;
                if separator.is_null() {
                    separator = c!("");
                }
            }
            value = format_nul!("{}{}{}", _s((*o).value.string), _s(separator), _s(s),);
            free_(s);
        } else {
            value = s;
        }

        if o.is_null() && *name == b'@' {
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
    name: *const u8,
    value: i64,
) -> *mut options_entry {
    unsafe {
        if *name == b'@' {
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
    name: *const u8,
    fs: *mut cmd_find_state,
    oo: *mut *mut options,
    cause: *mut *mut u8,
) -> i32 {
    unsafe {
        let s = (*fs).s;
        let wl = (*fs).wl;
        let wp = (*fs).wp;
        let target = args_get_(args, 't');
        let mut scope = OPTIONS_TABLE_NONE;

        if *name == b'@' {
            return options_scope_from_flags(args, window, fs, oo, cause);
        }

        let mut oe = &raw const OPTIONS_TABLE as *const options_table_entry;
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
                *oo = GLOBAL_OPTIONS;
                scope = OPTIONS_TABLE_SERVER;
            }
            OPTIONS_TABLE_SESSION => {
                if args_has(args, 'g') {
                    *oo = GLOBAL_S_OPTIONS;
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
                if args_has(args, 'p') {
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
                    if args_has(args, 'g') {
                        *oo = GLOBAL_W_OPTIONS;
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
                if args_has(args, 'g') {
                    *oo = GLOBAL_W_OPTIONS;
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
    cause: *mut *mut u8,
) -> i32 {
    unsafe {
        let s = (*fs).s;
        let wl = (*fs).wl;
        let wp = (*fs).wp;
        let target = args_get_(args, 't');

        if args_has(args, 's') {
            *oo = GLOBAL_OPTIONS;
            return OPTIONS_TABLE_SERVER;
        }

        if args_has(args, 'p') {
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
        } else if window != 0 || args_has(args, 'w') {
            if args_has(args, 'g') {
                *oo = GLOBAL_W_OPTIONS;
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
            if args_has(args, 'g') {
                *oo = GLOBAL_S_OPTIONS;
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
    name: *const u8,
    ft: *mut format_tree,
) -> *mut style {
    let __func__ = c!("options_string_to_style");
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

        style_set(&mut (*o).style, &GRID_DEFAULT_CELL);
        (*o).cached = if strstr(s, c!("#{")).is_null() { 1 } else { 0 };

        if !ft.is_null() && (*o).cached == 0 {
            let expanded = format_expand(ft, s);
            if style_parse(&mut (*o).style, &GRID_DEFAULT_CELL, expanded) != 0 {
                free_(expanded);
                return null_mut();
            }
            free_(expanded);
        } else if style_parse(&mut (*o).style, &GRID_DEFAULT_CELL, s) != 0 {
            return null_mut();
        }
        &mut (*o).style
    }
}

unsafe fn options_from_string_check(
    oe: *const options_table_entry,
    value: *const u8,
) -> Result<(), CString> {
    unsafe {
        let mut sy: style = std::mem::zeroed();

        if oe.is_null() {
            return Ok(());
        }
        if streq_((*oe).name, "default-shell") && !checkshell_(value) {
            return Err(CString::new(format!("not a suitable shell: {}", _s(value))).unwrap());
        }
        if !(*oe).pattern.is_null() && fnmatch((*oe).pattern, value, 0) != 0 {
            return Err(CString::new(format!("value is invalid: {}", _s(value))).unwrap());
        }
        if ((*oe).flags & OPTIONS_TABLE_IS_STYLE) != 0
            && strstr(value, c!("#{")).is_null()
            && style_parse(&mut sy, &GRID_DEFAULT_CELL, value) != 0
        {
            return Err(CString::new(format!("invalid style: {}", _s(value))).unwrap());
        }
        Ok(())
    }
}

unsafe fn options_from_string_flag(
    oo: *mut options,
    name: *const u8,
    value: *const u8,
) -> Result<(), CString> {
    unsafe {
        let flag = if value.is_null() || *value == 0 {
            options_get_number(oo, name) == 0
        } else if streq_(value, "1") || strcaseeq_(value, "on") || strcaseeq_(value, "yes") {
            true
        } else if streq_(value, "0") || strcaseeq_(value, "off") || strcaseeq_(value, "no") {
            false
        } else {
            return Err(CString::new(format!("bad value: {}", _s(value))).unwrap());
        };
        options_set_number(oo, name, flag as i64);
        Ok(())
    }
}

pub unsafe fn options_find_choice(
    oe: *const options_table_entry,
    value: *const u8,
) -> Result<i32, CString> {
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
            return Err(CString::new(format!("unknown value: {}", _s(value))).unwrap());
        }
        Ok(choice)
    }
}

unsafe fn options_from_string_choice(
    oe: *const options_table_entry,
    oo: *mut options,
    name: *const u8,
    value: *const u8,
) -> Result<(), CString> {
    unsafe {
        let choice = if value.is_null() {
            let mut choice = options_get_number(oo, name);
            if choice < 2 {
                choice = if choice == 0 { 1 } else { 0 };
            }
            choice
        } else {
            options_find_choice(oe, value)? as i64
        };
        options_set_number(oo, name, choice);
        Ok(())
    }
}

pub unsafe fn options_from_string(
    oo: *mut options,
    oe: *const options_table_entry,
    name: *const u8,
    value: *const u8,
    append: bool,
) -> Result<(), CString> {
    unsafe {
        let new: *const u8;
        let old: *mut u8;
        let key: key_code;

        let type_: options_table_type = if !oe.is_null() {
            if value.is_null()
                && (*oe).type_ != options_table_type::OPTIONS_TABLE_FLAG
                && (*oe).type_ != options_table_type::OPTIONS_TABLE_CHOICE
            {
                return Err(CString::new("empty value").unwrap());
            }
            (*oe).type_
        } else {
            if *name != b'@' {
                return Err(CString::new("bad option name").unwrap());
            }
            options_table_type::OPTIONS_TABLE_STRING
        };

        match type_ {
            options_table_type::OPTIONS_TABLE_STRING => {
                old = xstrdup(options_get_string(oo, name)).as_ptr();
                options_set_string!(oo, name, append, "{}", _s(value));

                new = options_get_string(oo, name);
                if let Err(err) = options_from_string_check(oe, new) {
                    options_set_string!(oo, name, false, "{}", _s(old));
                    free_(old);
                    return Err(err);
                }
                free_(old);
                return Ok(());
            }

            options_table_type::OPTIONS_TABLE_NUMBER => {
                match strtonum(value, (*oe).minimum as i64, (*oe).maximum as i64) {
                    Ok(number) => {
                        options_set_number(oo, name, number);
                        return Ok(());
                    }
                    Err(errstr) => {
                        return Err(CString::new(format!(
                            "value is {}: {}",
                            _s(errstr.as_ptr()),
                            _s(value)
                        ))
                        .unwrap());
                    }
                }
            }

            options_table_type::OPTIONS_TABLE_KEY => {
                key = key_string_lookup_string(value);
                if key == KEYC_UNKNOWN {
                    return Err(CString::new(format!("bad key: {}", _s(value))).unwrap());
                }
                options_set_number(oo, name, key as i64);
                return Ok(());
            }

            options_table_type::OPTIONS_TABLE_COLOUR => {
                let number = colour_fromstring(value) as i64;
                if number == -1 {
                    return Err(CString::new(format!("bad colour: {}", _s(value))).unwrap());
                }
                options_set_number(oo, name, number);
                return Ok(());
            }

            options_table_type::OPTIONS_TABLE_FLAG => {
                return options_from_string_flag(oo, name, value);
            }

            options_table_type::OPTIONS_TABLE_CHOICE => {
                return options_from_string_choice(oe, oo, name, value);
            }

            options_table_type::OPTIONS_TABLE_COMMAND => {}
        }

        Err(CString::new("").unwrap())
    }
}

pub unsafe fn options_push_changes(name: *const u8) {
    let __func__ = c!("options_push_changes");
    unsafe {
        log_debug!("{}: {}", _s(__func__), _s(name));

        if streq_(name, "automatic-rename") {
            for w in rb_foreach(&raw mut WINDOWS).map(NonNull::as_ptr) {
                if (*w).active.is_null() {
                    continue;
                }
                if options_get_number((*w).options, name) != 0 {
                    (*(*w).active).flags |= window_pane_flags::PANE_CHANGED;
                }
            }
        }

        if streq_(name, "cursor-colour") {
            for wp in rb_foreach(&raw mut ALL_WINDOW_PANES) {
                window_pane_default_cursor(wp.as_ptr());
            }
        }

        if streq_(name, "cursor-style") {
            for wp in rb_foreach(&raw mut ALL_WINDOW_PANES) {
                window_pane_default_cursor(wp.as_ptr());
            }
        }

        if streq_(name, "fill-character") {
            for w in rb_foreach(&raw mut WINDOWS) {
                window_set_fill_character(w);
            }
        }

        if streq_(name, "key-table") {
            for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
                server_client_set_key_table(loop_, null_mut());
            }
        }

        if streq_(name, "user-keys") {
            for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
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
            for wp in rb_foreach(&raw mut ALL_WINDOW_PANES) {
                (*wp.as_ptr()).flags |= window_pane_flags::PANE_STYLECHANGED;
            }
        }

        if streq_(name, "pane-colours") {
            for wp in rb_foreach(&raw mut ALL_WINDOW_PANES).map(NonNull::as_ptr) {
                colour_palette_from_option(&raw mut (*wp).palette, (*wp).options);
            }
        }

        if streq_(name, "pane-border-status") {
            for w in rb_foreach(&raw mut WINDOWS) {
                layout_fix_panes(w.as_ptr(), null_mut());
            }
        }

        for s in rb_foreach(&raw mut SESSIONS) {
            status_update_cache(s.as_ptr());
        }

        recalculate_sizes();

        for loop_ in tailq_foreach(&raw mut CLIENTS).map(NonNull::as_ptr) {
            if !(*loop_).session.is_null() {
                server_redraw_client(loop_);
            }
        }
    }
}

// note one difference was that this function previously could avoid allocation on error
pub unsafe fn options_remove_or_default(o: *mut options_entry, idx: i32) -> Result<(), CString> {
    unsafe {
        let oo = (*o).owner;

        if idx == -1 {
            if !(*o).tableentry.is_null()
                && (oo == GLOBAL_OPTIONS || oo == GLOBAL_S_OPTIONS || oo == GLOBAL_W_OPTIONS)
            {
                options_default(oo, (*o).tableentry);
            } else {
                options_remove(o);
            }
        } else {
            options_array_set(o, idx as u32, None, false)?;
        }
        Ok(())
    }
}
