use ::std::{
    alloc::{GlobalAlloc, Layout},
    ffi::CString,
    str::FromStr as _,
};

struct MyAlloc;
#[global_allocator]
static ALLOCATOR: MyAlloc = MyAlloc;
unsafe impl GlobalAlloc for MyAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            if layout.align() <= MIN_ALIGN {
                libc::malloc(layout.size()) as *mut u8
            } else {
                // malloc only guarantees MIN_ALIGN alignment; use
                // aligned_alloc for larger alignment requirements.
                #[cfg(unix)]
                {
                    libc::aligned_alloc(layout.align(), layout.size()) as *mut u8
                }
                #[cfg(windows)]
                {
                    libc::aligned_malloc(layout.size(), layout.align()) as *mut u8
                }
            }
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            if layout.align() <= MIN_ALIGN {
                libc::free(ptr.cast())
            } else {
                #[cfg(unix)]
                libc::free(ptr.cast());
                #[cfg(windows)]
                libc::aligned_free(ptr.cast());
            }
        }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if layout.align() <= MIN_ALIGN {
            let align = layout.align();
            // exploit we know align must be a non-zero power of 2 to do a faster division
            let nmemb = (layout.size() + align - 1) >> align.trailing_zeros();
            unsafe { libc::calloc(nmemb, align) as *mut u8 }
        } else {
            let ptr = unsafe { self.alloc(layout) };
            if !ptr.is_null() {
                unsafe { core::ptr::write_bytes(ptr, 0, layout.size()) };
            }
            ptr
        }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if layout.align() <= MIN_ALIGN {
            unsafe { libc::realloc(ptr.cast(), new_size) as *mut u8 }
        } else {
            // realloc doesn't guarantee alignment > MIN_ALIGN, so we
            // must allocate new aligned memory and copy.
            let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
            let new_ptr = unsafe { self.alloc(new_layout) };
            if !new_ptr.is_null() {
                let copy_len = layout.size().min(new_size);
                unsafe { core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_len) };
                unsafe { self.dealloc(ptr, layout) };
            }
            new_ptr
        }
    }
}

/// Minimum alignment guaranteed by malloc on this platform.
#[cfg(target_pointer_width = "64")]
const MIN_ALIGN: usize = 16;
#[cfg(target_pointer_width = "32")]
const MIN_ALIGN: usize = 8;

// TODO idea:
// I noticed in the tmux code base there are many places an empty string is allocated so that
// there's data there which is valid and can be freed or realloced later. Since we hook into
// the allocator I wonder if it would be worth it to reuse a common empty string, and coding
// the allocator to allow multiple frees of that empty string. I suspect it wouldn't because
// it would be adding unecessary code to free in the common case.

// It could also be interesting to add in a histogram for viewing memory allocations

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let args = args
        .into_iter()
        .map(|s| CString::from_str(&s).unwrap())
        .collect::<Vec<CString>>();
    let mut args: Vec<*mut u8> = args.into_iter().map(|s| s.into_raw().cast()).collect();

    // TODO
    // passing null_mut() as env is ok for now because setproctitle call was removed
    // a similar shim will need to be added when that call is re-added
    unsafe {
        tmux_rs::tmux_main(
            args.len() as i32,
            args.as_mut_slice().as_mut_ptr(),
            std::ptr::null_mut(),
        );
    }

    drop(
        args.into_iter()
            .map(|ptr| unsafe { CString::from_raw(ptr.cast()) }),
    );
}
