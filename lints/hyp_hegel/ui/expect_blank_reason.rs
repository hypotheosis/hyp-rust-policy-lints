#[expect(non_hegel_test, reason = "  ")]
#[test]
fn blank_expectation() {
    assert_eq!(1 + 1, 2);
}

// Pins that a whitespace-only reason on an `expect` is no reason: the guard on
// the `Expect` arm is `has_reason`, not `reason.is_none()`. Exactly one
// diagnostic, and no unfulfilled-expectation warning on top of it.

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
