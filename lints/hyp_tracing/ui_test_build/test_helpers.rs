//! Compiled with `--test`. Neither the test nor the helper it calls is
//! production code, and neither needs a span.

fn helper() -> u32 {
    1
}

#[test]
fn uses_helper() {
    assert_eq!(helper(), 1);
}
