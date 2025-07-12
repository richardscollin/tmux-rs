use ::std::{
    alloc::{GlobalAlloc, Layout},
    ffi::CString,
};

struct MyAlloc;
#[global_allocator]
static ALLOCATOR: MyAlloc = MyAlloc;
unsafe impl GlobalAlloc for MyAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { libc::malloc(layout.size()) as *mut u8 }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { libc::free(ptr.cast()) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        // exploit we know align must be a non-zero power of 2 to do a faster division
        let nmemb = (layout.size() + align - 1) >> align.trailing_zeros();
        unsafe { libc::calloc(nmemb, align) as *mut u8 }
    }
    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        unsafe { libc::realloc(ptr.cast(), new_size) as *mut u8 }
    }
}

// TODO idea:
// I noticed in the tmux code base there are many places an empty string is allocated so that
// there's data there which is valid and can be freed or realloced later. Since we hook into
// the allocator I wonder if it would be worth it to reuse a common empty string, and coding
// the allocator to allow multiple frees of that empty string. I suspect it wouldn't because
// it would be adding unecessary code to free in the common case.

// It could also be interesting to add in a histogram for viewing memory allocations

fn main() {
    let args = std::env::args()
        .map(|s| CString::new(s).unwrap())
        .collect::<Vec<CString>>();

    // note these &str are also nul terminated, but not included in the &str len()
    let args: Vec<&str> = args
        .iter()
        .map(|cstr| str::from_utf8(cstr.as_bytes()).unwrap())
        .collect();

    // TODO
    // passing null_mut() as env is ok for now because setproctitle call was removed
    // a similar shim will need to be added when that call is re-added
    unsafe { tmux_rs::tmux_main(&args, std::ptr::null_mut()) }
}
