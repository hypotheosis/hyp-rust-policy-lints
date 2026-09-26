#[expect(uninstrumented_fn)]
#[tracing::instrument]
pub fn instrumented() {}
