pub fn getdtablecount() -> i32 {
    if let Ok(read_dir) = std::fs::read_dir("/proc/self/fd") {
        read_dir.count() as i32
    } else {
        0
    }
}
