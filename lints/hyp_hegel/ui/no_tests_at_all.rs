// The policy's deliberate silence. A crate with no test harness at all is not
// reported: "this crate has no tests" is a different policy question and is
// out of scope for `crate_without_hegel_tests`.
//
// This fixture is the only one that constrains the `test_fns.is_empty()` guard
// in `check_crate_post`. Every other fixture has at least one test, so without
// this one the guard could be deleted and the whole suite would still pass.
fn not_a_test() {}

fn main() {
    not_a_test();
}
