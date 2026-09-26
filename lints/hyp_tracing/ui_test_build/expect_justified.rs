//! A justified `expect` on a library function must stay fulfilled in the
//! `--test` compilation, where the pass otherwise reports nothing --
//! or rustc warns that the expectation is unfulfilled.

#[expect(uninstrumented_fn, reason = "legacy entry point, instrumented in the follow-up refactor")]
pub fn legacy() {}
