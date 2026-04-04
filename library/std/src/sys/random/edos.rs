pub fn fill_bytes(buf: &mut [u8]) {
    edos_rt::io::getrandom(buf).unwrap();
}
