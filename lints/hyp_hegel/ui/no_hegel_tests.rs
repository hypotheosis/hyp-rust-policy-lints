#[allow(non_hegel_test, reason = "exercising the crate-level check")]
#[test]
fn only_example_based() {
    assert_eq!(1 + 1, 2);
}

fn main() {}
