pub extern "C" fn getdtablecount_() -> libc::c_int {
    if let Ok(read_dir) = std::fs::read_dir("/proc/self/fd") {
        let mut i = 0;
        for _ in read_dir {
            i += 1;
        }
        i
    } else {
        0
    }
}

pub extern "C" fn getdtablecount() -> libc::c_int {
    let mut n = 0;
    let mut g: libc::glob_t = unsafe { std::mem::zeroed() };

    unsafe {
        if libc::glob(c"/proc/self/fd".as_ptr(), 0, None, &raw mut g) == 0 {
            n = g.gl_pathc as libc::c_int;
        }
        libc::globfree(&raw mut g);
    }

    n
}

// #[test]
// fn test_getdtablecount() {
//     let descriptor_count = getdtablecount1();
//     assert_eq!(descriptor_count, 1);
// }
