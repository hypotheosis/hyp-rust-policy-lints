#[allow(uninstrumented_fn, reason = "generated lookup tables; a span per lookup would be noise")]
pub mod tables {
    // The nearest attribute decides: this unjustified `allow` is reported even
    // though the enclosing module's `allow` carries a reason.
    #[allow(uninstrumented_fn)]
    pub fn first() {}
}
