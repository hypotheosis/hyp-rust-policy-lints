#[tracing::instrument]
pub fn outer() -> u32 {
    fn inner() -> u32 {
        1
    }
    inner()
}
