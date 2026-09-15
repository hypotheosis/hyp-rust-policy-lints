#[test]
fn closure_only() {
    std::iter::once(0).for_each(|_| {
        let tc = hegel::TestCase;
        let _: i64 = tc.draw();
    });
}

fn main() {}
