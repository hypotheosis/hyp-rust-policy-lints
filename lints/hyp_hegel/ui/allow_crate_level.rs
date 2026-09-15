#![allow(non_hegel_test, reason = "this crate is a golden-file corpus; no general property holds")]

#[test]
fn plain_assertion() {
    assert_eq!(1 + 1, 2);
}

// Present only to keep this fixture's subject the *per-test* lint. One hegel
// test is enough to silence `crate_without_hegel_tests`, which has its own
// fixtures (`no_hegel_tests.rs`, `no_tests_at_all.rs`).
//
// Deliberately the builder form, not `#[hegel::test]`: the stub macro rebuilds
// its input from a string and discards every attribute, so a fixture that
// depends on an attribute surviving must avoid it.
#[test]
fn keeps_crate_lint_quiet() {
    hegel::Hegel::new(|tc| {
        let n: i64 = tc.draw();
        assert_eq!(n, n);
    })
    .run();
}

fn main() {}
