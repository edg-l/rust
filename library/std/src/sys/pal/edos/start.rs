use core::arch::naked_asm;

use edos_rt::process::sys_exit;

unsafe extern "C" {
    fn main(argc: isize, argv: *const *const u8) -> i32;
}

/// Process entry.
///
/// The kernel jumps here rather than reaching it through a `call`, so the stack
/// alignment on entry is whatever the kernel chose and not the post-call state
/// a compiled function assumes. Masking `%rsp` down is what every libc's
/// `crt1.o` does for exactly this reason, and it makes this entry point correct
/// under both the psABI's 16-aligned process entry and the post-call alignment
/// the kernel produces today.
///
/// `%rbp` is zeroed so a frame-pointer walk terminates here, as the psABI
/// requires of the outermost frame.
///
/// `argc`, `argv` and `envp` arrive in `rdi`/`rsi`/`rdx` and are left where
/// they are; the `call` below hands them to [`start_rust`] unchanged. The same
/// values are also on the initial stack, together with an auxiliary vector,
/// which is what a C runtime reads instead.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        "xor ebp, ebp",
        "and rsp, -16",
        "call {entry}",
        entry = sym start_rust,
    )
}

extern "C" fn start_rust(argc: isize, argv: *const *const u8, envp: *const *const u8) -> ! {
    unsafe {
        // Before anything else, because everything else may allocate and the
        // allocator's per-thread cache lives in the space this checks for.
        edos_rt::tcb::verify_or_abort(envp);
        edos_rt::env::init_env(envp);
        crate::sys::args::init(argc, argv);
        let code = main(argc, argv);

        sys_exit(code as i32)
    };
}
