use edos_rt::process::sys_exit;

unsafe extern "C" {
    fn main(argc: isize, argv: *const *const u8) -> i32;
}

#[unsafe(no_mangle)]
#[allow(unused)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    unsafe {
        crate::sys::args::init(argc, argv);
        let code = main(argc, argv);

        sys_exit(code as i32)
    };
}
