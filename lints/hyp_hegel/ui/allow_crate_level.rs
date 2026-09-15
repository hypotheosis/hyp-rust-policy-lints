#![allow(non_hegel_test, reason = "this crate is a golden-file corpus; no general property holds")]

#[test]
fn plain_assertion() {
    assert_eq!(1 + 1, 2);
}

fn main() {}
