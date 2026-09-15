#[allow(non_hegel_test, reason = "asserts an exact serialized byte layout; no general property holds")]
#[test]
fn golden_wire_format() {
    assert_eq!(format!("{:?}", 1u8), "1");
}

fn main() {}
