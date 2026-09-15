// The escape hatch for the crate-level lint, and the only fixture that holds
// it open. A crate that has deliberately decided against property testing
// silences `crate_without_hegel_tests` with one justified crate-level `allow`;
// this fixture fails if that stops working.
//
// It also pins the documented interaction in the lint's own doc comment: the
// per-test exemption below does *not* on its own suppress the crate-level
// lint. Drop the first `allow` and this fixture starts failing, even though
// every test in the crate is still individually exempted with a reason.
#![allow(
    crate_without_hegel_tests,
    reason = "golden-file corpus; property testing does not apply to this crate"
)]
#![allow(
    non_hegel_test,
    reason = "golden-file corpus; property testing does not apply to this crate"
)]

#[test]
fn only_example_based() {
    assert_eq!(1 + 1, 2);
}

fn main() {}
