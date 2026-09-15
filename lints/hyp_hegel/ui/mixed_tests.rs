// Guards the `hegel_test_count` bookkeeping. One hegel test is enough to keep
// `crate_without_hegel_tests` quiet, even though a non-hegel test in the same
// crate is still reported on its own. This fixture fails if the count is not
// incremented on the hegel skip path, because the crate-level lint would then
// fire here as well.
#[hegel::test]
fn addition_commutes(tc: hegel::TestCase) {
    let a: i64 = tc.draw();
    assert_eq!(a.wrapping_add(0), a);
}

#[test]
fn plain_assertion() {
    assert_eq!(1 + 1, 2);
}

fn main() {}
