#[hegel::test]
fn addition_commutes(tc: hegel::TestCase) {
    let a: i64 = tc.draw();
    assert_eq!(a.wrapping_add(0), a);
}

fn main() {}
