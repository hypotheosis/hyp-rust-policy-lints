use hegel::Hegel;

#[expect(non_hegel_test)]
#[test]
fn built_property() {
    Hegel::new(|tc| {
        let n: i64 = tc.draw();
        assert_eq!(n, n);
    })
    .run();
}

fn main() {}
