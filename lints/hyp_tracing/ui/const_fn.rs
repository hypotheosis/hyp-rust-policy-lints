//! `#[instrument]` cannot be applied to a `const fn`, so the lint must not
//! demand it.

pub const fn answer() -> u32 {
    42
}
