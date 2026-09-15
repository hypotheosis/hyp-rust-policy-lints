use hegel::Hegel;

#[test]
fn built_property() {
    Hegel::new(|tc| {
        let n: i64 = tc.draw();
        assert_eq!(n, n);
    })
    .run();
}

fn main() {}
