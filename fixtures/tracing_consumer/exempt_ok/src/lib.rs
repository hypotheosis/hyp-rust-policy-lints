#[cfg_attr(
    dylint_lib = "hyp_tracing",
    allow(
        uninstrumented_fn,
        reason = "called once per byte by the decoder; a span per call would dominate its cost"
    )
)]
pub fn scramble(b: u8) -> u8 {
    b ^ 0x5a
}
