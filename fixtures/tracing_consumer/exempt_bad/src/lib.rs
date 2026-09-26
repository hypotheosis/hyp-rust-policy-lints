#[cfg_attr(dylint_lib = "hyp_tracing", allow(uninstrumented_fn))]
pub fn scramble(b: u8) -> u8 {
    b ^ 0x5a
}
