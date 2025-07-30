// https://man.openbsd.org/tree.3
// probably best way define a generic struct
// make the macros call the generic struct
use ::core::cmp::Ordering;
use ::core::ptr::{NonNull, null_mut};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct rb_head<T> {
    pub rbh_root: *mut T,
}

impl<T> Default for rb_head<T> {
    fn default() -> Self {
        Self {
            rbh_root: null_mut(),
        }
    }
}

impl<T> rb_head<T> {
    pub(crate) const fn rb_init() -> Self {
        Self {
            rbh_root: null_mut(),
        }
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum rb_color {
    #[default]
    RB_BLACK = 0,
    RB_RED = 1,
}

#[repr(C)]
pub struct rb_entry<T> {
    pub rbe_left: *mut T,
    pub rbe_right: *mut T,
    pub rbe_parent: *mut T,
    rbe_color: rb_color,
}

impl<T> std::fmt::Debug for rb_entry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("rb_entry")
            .field("rbe_left", &self.rbe_left)
            .field("rbe_right", &self.rbe_right)
            .field("rbe_parent", &self.rbe_parent)
            .field("rbe_color", &self.rbe_color)
            .finish()
    }
}

impl<T> Default for rb_entry<T> {
    fn default() -> Self {
        Self {
            rbe_left: null_mut(),
            rbe_right: null_mut(),
            rbe_parent: null_mut(),
            rbe_color: Default::default(),
        }
    }
}

impl<T> Copy for rb_entry<T> {}
impl<T> Clone for rb_entry<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub trait GetEntry<T, D = ()> {
    unsafe fn entry_mut(this: *mut Self) -> *mut rb_entry<T>;
    unsafe fn entry(this: *const Self) -> *const rb_entry<T>;
    fn cmp(this: &Self, other: &Self) -> std::cmp::Ordering;
}

pub const unsafe fn rb_init<T>(head: *mut rb_head<T>) {
    unsafe {
        (*head).rbh_root = null_mut();
    }
}

pub const fn rb_initializer<T>() -> rb_head<T> {
    rb_head {
        rbh_root: null_mut(),
    }
}

macro_rules! rb_left {
    ($elm:expr) => {
        (*GetEntry::entry_mut($elm)).rbe_left
    };
}
pub unsafe fn rb_left<T, D>(this: *mut T) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe { (*T::entry_mut(this)).rbe_left }
}
pub unsafe fn rb_left_const<T, D>(this: *const T) -> *const T
where
    T: GetEntry<T, D>,
{
    unsafe { (*T::entry(this)).rbe_left }
}

#[inline]
pub unsafe fn is_left_sibling<T, D>(this: *mut T) -> bool
where
    T: GetEntry<T, D>,
{
    unsafe { this == rb_left(rb_parent(this)) }
}

#[inline]
pub unsafe fn is_right_sibling<T, D>(this: *mut T) -> bool
where
    T: GetEntry<T, D>,
{
    unsafe { this == rb_right(rb_parent(this)) }
}

macro_rules! rb_right {
    ($elm:expr) => {
        (*GetEntry::entry_mut($elm)).rbe_right
    };
}
pub unsafe fn rb_right<T, D>(this: *mut T) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe { (*T::entry_mut(this)).rbe_right }
}
pub unsafe fn rb_right_const<T, D>(this: *const T) -> *const T
where
    T: GetEntry<T, D>,
{
    unsafe { (*T::entry(this)).rbe_right }
}

macro_rules! rb_parent {
    ($elm:expr) => {
        (*GetEntry::entry_mut($elm)).rbe_parent
    };
}
pub unsafe fn rb_parent<T, D>(this: *mut T) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe { (*T::entry_mut(this)).rbe_parent }
}

macro_rules! rb_color {
    ($elm:expr) => {
        (*GetEntry::entry_mut($elm)).rbe_color
    };
}
pub unsafe fn rb_color<T, D>(elm: *mut T) -> rb_color
where
    T: GetEntry<T, D>,
{
    unsafe { (*T::entry_mut(elm)).rbe_color }
}

macro_rules! rb_root {
    ($head:expr) => {
        (*$head).rbh_root
    };
}
pub unsafe fn rb_root<T>(head: *mut rb_head<T>) -> *mut T {
    unsafe { (*head).rbh_root }
}
pub unsafe fn rb_empty<T>(head: *const rb_head<T>) -> bool {
    unsafe { (*head).rbh_root.is_null() }
}
pub unsafe fn rb_set<T, D>(elm: *mut T, parent: *mut T)
where
    T: GetEntry<T, D>,
{
    unsafe {
        let ptr = T::entry_mut(elm);
        (*ptr).rbe_parent = parent;
        (*ptr).rbe_right = null_mut();
        (*ptr).rbe_left = null_mut();
        (*ptr).rbe_color = rb_color::RB_RED;
    }
}

pub unsafe fn rb_set_blackred<T, D>(black: *mut T, red: *mut T)
where
    T: GetEntry<T, D>,
{
    unsafe {
        (*T::entry_mut(black)).rbe_color = rb_color::RB_BLACK;
        (*T::entry_mut(red)).rbe_color = rb_color::RB_RED;
    }
}

pub unsafe fn rb_rotate_left<T, D>(head: *mut rb_head<T>, elm: *mut T)
where
    T: GetEntry<T, D>,
{
    unsafe {
        let tmp = rb_right(elm);
        rb_right!(elm) = rb_left(tmp);
        if !rb_right!(elm).is_null() {
            rb_parent!(rb_left(tmp)) = elm;
        }
        rb_parent!(tmp) = rb_parent(elm);
        if !rb_parent(tmp).is_null() {
            if is_left_sibling(elm) {
                rb_left!(rb_parent(elm)) = tmp;
            } else {
                rb_right!(rb_parent(elm)) = tmp;
            }
        } else {
            (*head).rbh_root = tmp;
        }

        rb_left!(tmp) = elm;
        rb_parent!(elm) = tmp;
    }
}

pub unsafe fn rb_rotate_right<T, D>(head: *mut rb_head<T>, elm: *mut T)
where
    T: GetEntry<T, D>,
{
    unsafe {
        let tmp = rb_left(elm);
        rb_left!(elm) = rb_right(tmp);
        if !rb_left(elm).is_null() {
            rb_parent!(rb_right(tmp)) = elm;
        }
        rb_parent!(tmp) = rb_parent(elm);
        if !rb_parent(tmp).is_null() {
            if is_left_sibling(elm) {
                rb_left!(rb_parent(elm)) = tmp;
            } else {
                rb_right!(rb_parent(elm)) = tmp;
            }
        } else {
            (*head).rbh_root = tmp;
        }
        rb_right!(tmp) = elm;
        rb_parent!(elm) = tmp;
    }
}

// RB_GENERATE_STATIC name, type, field, cmp
macro_rules! RB_GENERATE {
    ($head_ty:ty, $ty:ty, $entry_field:ident, $entry_field_discr:ty, $cmp_fn:ident) => {
        impl $crate::compat::tree::GetEntry<$ty, $entry_field_discr> for $ty {
            unsafe fn entry(this: *const Self) -> *const rb_entry<$ty> {
                unsafe { &raw const (*this).$entry_field }
            }
            unsafe fn entry_mut(this: *mut Self) -> *mut rb_entry<$ty> {
                unsafe { &raw mut (*this).$entry_field }
            }
            fn cmp(this: &Self, other: &Self) -> std::cmp::Ordering {
                $cmp_fn(this, other)
            }
        }
    };
}
pub(crate) use RB_GENERATE;

pub unsafe fn rb_minmax_const<T, D>(head: *const rb_head<T>, val: i32) -> *const T
where
    T: GetEntry<T, D>,
{
    unsafe {
        let mut tmp: *const T = (*head).rbh_root;
        let mut parent: *const T = null_mut();

        while !tmp.is_null() {
            parent = tmp;
            if val < 0 {
                tmp = rb_left_const(tmp);
            } else {
                tmp = rb_right_const(tmp);
            }
        }

        parent
    }
}

pub unsafe fn rb_minmax<T, D>(head: *mut rb_head<T>, val: i32) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe {
        let mut tmp: *mut T = (*head).rbh_root;
        let mut parent: *mut T = null_mut();

        while !tmp.is_null() {
            parent = tmp;
            if val < 0 {
                tmp = rb_left(tmp);
            } else {
                tmp = rb_right(tmp);
            }
        }

        parent
    }
}

pub unsafe fn rb_insert_color<T, D>(head: *mut rb_head<T>, mut elm: *mut T)
where
    T: GetEntry<T, D>,
{
    unsafe {
        while let Some(parent) = NonNull::new(rb_parent(elm))
            && rb_color(parent.as_ptr()) == rb_color::RB_RED
        {
            #[expect(clippy::shadow_reuse)]
            let mut parent = parent.as_ptr();
            let gparent = rb_parent(parent);
            if parent == rb_left(gparent) {
                let mut tmp = rb_right(gparent);
                if !tmp.is_null() && rb_color(tmp) == rb_color::RB_RED {
                    rb_color!(tmp) = rb_color::RB_BLACK;
                    rb_set_blackred(parent, gparent);
                    elm = gparent;
                    continue;
                }
                if rb_right(parent) == elm {
                    rb_rotate_left(head, parent);
                    tmp = parent;
                    parent = elm;
                    elm = tmp;
                }
                rb_set_blackred(parent, gparent);
                rb_rotate_right(head, gparent);
            } else {
                let mut tmp = rb_left(gparent);
                if !tmp.is_null() && rb_color(tmp) == rb_color::RB_RED {
                    rb_color!(tmp) = rb_color::RB_BLACK;
                    rb_set_blackred(parent, gparent);
                    elm = gparent;
                    continue;
                }
                if rb_left(parent) == elm {
                    rb_rotate_right(head, parent);
                    tmp = parent;
                    parent = elm;
                    elm = tmp;
                }
                rb_set_blackred(parent, gparent);
                rb_rotate_left(head, gparent);
            }
        }
        (*T::entry_mut((*head).rbh_root)).rbe_color = rb_color::RB_BLACK;
    }
}

pub unsafe fn rb_remove_color<T, D>(head: *mut rb_head<T>, mut parent: *mut T, mut elm: *mut T)
where
    T: GetEntry<T, D>,
{
    unsafe {
        let mut tmp: *mut T;
        while (elm.is_null() || rb_color(elm) == rb_color::RB_BLACK) && elm != rb_root(head) {
            if rb_left(parent) == elm {
                tmp = rb_right(parent);

                if rb_color(tmp) == rb_color::RB_RED {
                    rb_set_blackred(tmp, parent);
                    rb_rotate_left(head, parent);
                    tmp = rb_right(parent);
                }
                if (rb_left(tmp).is_null() || rb_color(rb_left(tmp)) == rb_color::RB_BLACK)
                    && (rb_right(tmp).is_null() || rb_color(rb_right(tmp)) == rb_color::RB_BLACK)
                {
                    rb_color!(tmp) = rb_color::RB_RED;
                    elm = parent;
                    parent = rb_parent(elm);
                } else {
                    if rb_right(tmp).is_null() || rb_color(rb_right(tmp)) == rb_color::RB_BLACK {
                        let oleft = rb_left(tmp);
                        if !oleft.is_null() {
                            rb_color!(oleft) = rb_color::RB_BLACK;
                        }
                        rb_color!(tmp) = rb_color::RB_RED;
                        rb_rotate_right(head, tmp);
                        tmp = rb_right(parent);
                    }
                    rb_color!(tmp) = rb_color(parent);
                    rb_color!(parent) = rb_color::RB_BLACK;
                    if !rb_right(tmp).is_null() {
                        rb_color!(rb_right!(tmp)) = rb_color::RB_BLACK;
                    }
                    rb_rotate_left(head, parent);
                    elm = rb_root(head);
                    break;
                }
            } else {
                tmp = rb_left(parent);
                if rb_color(tmp) == rb_color::RB_RED {
                    rb_set_blackred(tmp, parent);
                    rb_rotate_right(head, parent);
                    tmp = rb_left(parent);
                }
                if (rb_left(tmp).is_null() || rb_color(rb_left(tmp)) == rb_color::RB_BLACK)
                    && (rb_right(tmp).is_null() || rb_color(rb_right(tmp)) == rb_color::RB_BLACK)
                {
                    rb_color!(tmp) = rb_color::RB_RED;
                    elm = parent;
                    parent = rb_parent(elm);
                } else {
                    if rb_left(tmp).is_null() || rb_color(rb_left(tmp)) == rb_color::RB_BLACK {
                        let oright = rb_right(tmp);
                        if !oright.is_null() {
                            rb_color!(oright) = rb_color::RB_BLACK;
                        }
                        rb_color!(tmp) = rb_color::RB_RED;
                        rb_rotate_left(head, oright);
                        tmp = rb_left(parent);
                    }
                    rb_color!(tmp) = rb_color(parent);
                    rb_color!(parent) = rb_color::RB_BLACK;
                    if !rb_left(tmp).is_null() {
                        rb_color!(rb_left(tmp)) = rb_color::RB_BLACK;
                    }
                    rb_rotate_right(head, parent);
                    elm = rb_root(head);
                    break;
                }
            }
        }

        if !elm.is_null() {
            rb_color!(elm) = rb_color::RB_BLACK;
        }
    }
}

pub unsafe fn rb_remove<T, D>(head: *mut rb_head<T>, mut elm: *mut T) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe {
        let old: *mut T = elm;
        let child: *mut T;
        let mut parent: *mut T;
        let color: rb_color;

        'color: {
            if rb_left(elm).is_null() {
                child = rb_right(elm);
            } else if rb_right(elm).is_null() {
                child = rb_left(elm);
            } else {
                elm = rb_right(elm);
                let mut left: *mut T;
                while {
                    left = rb_left(elm);
                    !left.is_null()
                } {
                    elm = left;
                }
                child = rb_right(elm);
                parent = rb_parent(elm);
                color = rb_color(elm);
                if !child.is_null() {
                    rb_parent!(child) = parent;
                }
                if !parent.is_null() {
                    if rb_left(parent) == elm {
                        rb_left!(parent) = child;
                    } else {
                        rb_right!(parent) = child;
                    }
                } else {
                    rb_root!(head) = child
                }
                if rb_parent(elm) == old {
                    parent = elm;
                }
                *GetEntry::entry_mut(elm) = *GetEntry::entry_mut(old);
                if !rb_parent(old).is_null() {
                    if is_left_sibling(old) {
                        rb_left!(rb_parent(old)) = elm;
                    } else {
                        rb_right!(rb_parent(old)) = elm;
                    }
                } else {
                    rb_root!(head) = elm;
                }
                rb_parent!(rb_left(old)) = elm;
                if !rb_right(old).is_null() {
                    rb_parent!(rb_right(old)) = elm;
                }
                if !parent.is_null() {
                    left = parent;

                    while {
                        left = rb_parent(left);
                        !left.is_null()
                    } {}
                }
                break 'color;
            }

            parent = rb_parent(elm);
            color = rb_color(elm);
            if !child.is_null() {
                rb_parent!(child) = parent;
            }
            if !parent.is_null() {
                if rb_left(parent) == elm {
                    rb_left!(parent) = child;
                } else {
                    rb_right!(parent) = child;
                }
            } else {
                rb_root!(head) = child;
            }
        }
        // color:
        if color == rb_color::RB_BLACK {
            rb_remove_color(head, parent, child);
        }
        old
    }
}

pub unsafe fn rb_insert<T, D>(head: *mut rb_head<T>, elm: *mut T) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe {
        let mut parent = null_mut();
        let mut comp = Ordering::Equal;

        let mut tmp = rb_root(head);
        while !tmp.is_null() {
            parent = tmp;

            comp = T::cmp(&*elm, &*parent);
            tmp = match comp {
                Ordering::Less => rb_left(tmp),
                Ordering::Greater => rb_right(tmp),
                Ordering::Equal => return tmp,
            };
        }
        rb_set(elm, parent);
        if !parent.is_null() {
            if matches!(comp, Ordering::Less) {
                rb_left!(parent) = elm;
            } else {
                rb_right!(parent) = elm;
            }
        } else {
            rb_root!(head) = elm;
        }
        rb_insert_color(head, elm);
    }
    null_mut()
}

// note the ordering from this must be the same as the default comparator
pub unsafe fn rb_find_by<T, D, F>(head: *mut rb_head<T>, cmp: F) -> *mut T
where
    T: GetEntry<T, D>,
    F: Fn(&T) -> std::cmp::Ordering,
{
    unsafe {
        let mut tmp: *mut T = (*head).rbh_root;

        while !tmp.is_null() {
            tmp = match cmp(&*tmp) {
                Ordering::Less => rb_left(tmp),
                Ordering::Greater => rb_right(tmp),
                Ordering::Equal => return tmp,
            };
        }
    }

    null_mut()
}

pub unsafe fn rb_find_by_const<T, D, F>(head: &rb_head<T>, cmp: F) -> *const T
where
    T: GetEntry<T, D>,
    F: Fn(&T) -> std::cmp::Ordering,
{
    unsafe {
        let mut tmp: *const T = head.rbh_root.cast_const();

        while !tmp.is_null() {
            tmp = match cmp(&*tmp) {
                Ordering::Less => rb_left_const(tmp),
                Ordering::Greater => rb_right_const(tmp),
                Ordering::Equal => return tmp,
            };
        }
    }

    null_mut()
}

pub unsafe fn rb_find<T, D>(head: *mut rb_head<T>, elm: *const T) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe {
        let mut tmp: *mut T = (*head).rbh_root;

        while !tmp.is_null() {
            tmp = match T::cmp(&*elm, &*tmp) {
                Ordering::Less => rb_left(tmp),
                Ordering::Greater => rb_right(tmp),
                Ordering::Equal => return tmp,
            };
        }
    }

    null_mut()
}

pub unsafe fn rb_nfind<T, D>(head: *mut rb_head<T>, elm: *const T) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe {
        let mut tmp = rb_root(head);
        let mut res = null_mut();
        while !tmp.is_null() {
            tmp = match T::cmp(&*elm, &*tmp) {
                Ordering::Less => {
                    res = tmp;
                    rb_left(tmp)
                }
                Ordering::Greater => rb_right(tmp),
                Ordering::Equal => return tmp,
            };
        }
        res
    }
}

pub unsafe fn rb_min<T, D>(head: *mut rb_head<T>) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe { rb_minmax(head, -1) }
}

pub unsafe fn rb_max<T, D>(head: *mut rb_head<T>) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe { rb_minmax(head, 1) }
}

pub unsafe fn rb_foreach_const<T, D>(head: *const rb_head<T>) -> ConstRbForwardIterator<T, D>
where
    T: GetEntry<T, D>,
{
    ConstRbForwardIterator {
        // TODO being a bit lazy reusing NonNull
        curr: NonNull::new(unsafe { rb_minmax_const(head, -1).cast_mut() }),
        _phantom: std::marker::PhantomData,
    }
}
pub struct ConstRbForwardIterator<T, D> {
    curr: Option<NonNull<T>>,
    _phantom: std::marker::PhantomData<D>,
}

impl<T, D> Iterator for ConstRbForwardIterator<T, D>
where
    T: GetEntry<T, D>,
{
    type Item = NonNull<T>;
    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr?.as_ptr();
        std::mem::replace(&mut self.curr, NonNull::new(unsafe { rb_next(curr) }))
    }
}

pub unsafe fn rb_foreach<T, D>(head: *mut rb_head<T>) -> RbForwardIterator<T, D>
where
    T: GetEntry<T, D>,
{
    RbForwardIterator {
        curr: NonNull::new(unsafe { rb_min(head) }),
        _phantom: std::marker::PhantomData,
    }
}
pub struct RbForwardIterator<T, D> {
    curr: Option<NonNull<T>>,
    _phantom: std::marker::PhantomData<D>,
}

impl<T, D> Iterator for RbForwardIterator<T, D>
where
    T: GetEntry<T, D>,
{
    type Item = NonNull<T>;
    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr?.as_ptr();
        std::mem::replace(&mut self.curr, NonNull::new(unsafe { rb_next(curr) }))
    }
}

pub unsafe fn rb_foreach_reverse<T, D>(head: *mut rb_head<T>) -> RbReverseIterator<T, D>
where
    T: GetEntry<T, D>,
{
    RbReverseIterator {
        curr: NonNull::new(unsafe { rb_max(head) }),
        _phantom: std::marker::PhantomData,
    }
}

pub struct RbReverseIterator<T, D> {
    curr: Option<NonNull<T>>,
    _phantom: std::marker::PhantomData<D>,
}

impl<T, D> Iterator for RbReverseIterator<T, D>
where
    T: GetEntry<T, D>,
{
    type Item = NonNull<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr?.as_ptr();
        std::mem::replace(&mut self.curr, NonNull::new(unsafe { rb_prev(curr) }))
    }
}

#[expect(clippy::collapsible_else_if)]
pub unsafe fn rb_next<T, D>(mut elm: *mut T) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe {
        if !rb_right(elm).is_null() {
            elm = rb_right(elm);
            while !rb_left(elm).is_null() {
                elm = rb_left(elm);
            }
        } else {
            if !rb_parent(elm).is_null() && is_left_sibling(elm) {
                elm = rb_parent(elm);
            } else {
                while !rb_parent(elm).is_null() && is_right_sibling(elm) {
                    elm = rb_parent(elm);
                }
                elm = rb_parent(elm);
            }
        }

        elm
    }
}

#[expect(clippy::collapsible_else_if)]
pub unsafe fn rb_prev<T, D>(mut elm: *mut T) -> *mut T
where
    T: GetEntry<T, D>,
{
    unsafe {
        if !rb_left(elm).is_null() {
            elm = rb_left(elm);
            while !rb_right(elm).is_null() {
                elm = rb_right(elm);
            }
        } else {
            if !rb_parent(elm).is_null() && is_right_sibling(elm) {
                elm = rb_parent(elm);
            } else {
                while !rb_parent(elm).is_null() && is_left_sibling(elm) {
                    elm = rb_parent(elm);
                }
                elm = rb_parent(elm);
            }
        }

        elm
    }
}
