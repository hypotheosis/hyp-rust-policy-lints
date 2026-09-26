#![expect(uninstrumented_fn)]

// Pins that an unjustified crate-level `expect` is reported once at the crate
// attribute (rustc deduplicates the identical diagnostic from each function),
// and that the expectation is still fulfilled, so no
// `unfulfilled_lint_expectations` warning joins it.

pub fn first() {}

pub fn second() {}
