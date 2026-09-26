//! `#[tracing::instrument]` on a synchronous free function is accepted.

#[tracing::instrument]
pub fn instrumented(n: u32) -> u32 {
    n + 1
}
