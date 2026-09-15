#[allow(non_hegel_test, reason = "")]
#[test]
fn empty_reason() {
    assert_eq!(1 + 1, 2);
}

#[allow(non_hegel_test, reason = "   ")]
#[test]
fn whitespace_reason() {
    assert_eq!(2 + 2, 4);
}

fn main() {}
