use ::std::{
    ffi::{CString, c_char},
    str::FromStr as _,
};

#[global_allocator]
static ALLOCATOR: std::alloc::System = std::alloc::System;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let args = args
        .into_iter()
        .map(|s| CString::from_str(&s).unwrap())
        .collect::<Vec<CString>>();
    let mut args: Vec<*mut c_char> = args.into_iter().map(|s| s.into_raw()).collect();

    // TODO
    // passing null_mut() as env is ok for now because setproctitle call was removed
    // a similar shim will need to be added when that call is re-added
    unsafe {
        tmux_rs::tmux_main(
            args.len() as i32,
            args.as_mut_slice().as_mut_ptr(),
            std::ptr::null_mut(),
        )
    }

    drop(
        args.into_iter()
            .map(|ptr| unsafe { CString::from_raw(ptr) }),
    );
}
