//! `#[instrument]` on an `async fn`. The body is lowered to a coroutine, and
//! the detection must look inside it.

#[tracing::instrument]
pub async fn instrumented(n: u32) -> u32 {
    n + 1
}
