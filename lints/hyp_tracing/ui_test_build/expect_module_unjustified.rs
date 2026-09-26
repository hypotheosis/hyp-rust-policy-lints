//! In the `--test` compilation, an `expect` -- even one with no reason, and
//! even on a parent module covering several functions -- is silently
//! fulfilled. The missing reason is reported by the ordinary compilation, not
//! this one.

#[expect(uninstrumented_fn)]
pub mod legacy {
    pub fn first() {}
    pub fn second() {}
}
