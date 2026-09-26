#![allow(uninstrumented_fn)]

// Pins that an unjustified crate-level `allow` is reported at the crate
// attribute, and that the two functions it covers produce one diagnostic
// between them (rustc deduplicates identical diagnostics), not one each.

pub fn first() {}

pub fn second() {}
