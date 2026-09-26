// Build scripts are out of hyp_tracing's scope: none of these is instrumented,
// and the package must still pass.
fn emit() {
    println!("cargo::rerun-if-changed=build.rs");
}

fn main() {
    emit();
}
