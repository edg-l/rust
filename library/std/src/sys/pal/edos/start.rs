use crate::ffi::{c_char, c_int};

unsafe extern "C" {
    fn main(argc: c_int, argv: *const *const c_char) -> c_int;
}

#[unsafe(no_mangle)]
#[allow(unused)]
pub extern "C" fn _start(argc: c_int, argv: *const *const c_char) {
    unsafe {
        main(argc, argv);
    };
}
