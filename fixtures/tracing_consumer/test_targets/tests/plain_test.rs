// A `harness = false` integration test: an ordinary binary, compiled without
// `--test`. Out of hyp_tracing's scope.
fn check() {
    assert_eq!(test_targets::VALUE, 1);
}

fn main() {
    check();
}
