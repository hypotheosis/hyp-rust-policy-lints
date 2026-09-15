// Regression guard, not an example: the only hegel reference here lives inside
// a closure body, so this fixture fails if `intravisit`'s `NestedFilter` ever
// stops descending into closures. `builder_form.rs` cannot catch that — its
// `.run()` sits outside the closure and matches first.
#[test]
fn closure_only() {
    std::iter::once(0).for_each(|_| {
        let tc = hegel::TestCase;
        let _: i64 = tc.draw();
    });
}

fn main() {}
