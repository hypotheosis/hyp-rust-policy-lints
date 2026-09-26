// A `harness = false` bench: an ordinary binary, compiled without `--test`.
// Out of hyp_tracing's scope.
fn measure() -> u32 {
    test_targets::VALUE
}

fn main() {
    let _ = measure();
}
