//! Minimal stand-in for the real `hegeltest` crate, used only by UI fixtures.

pub use hegel_macros::test;

pub struct TestCase;

impl TestCase {
    pub fn draw<T: Default>(&self) -> T {
        T::default()
    }
}

pub struct Hegel;

impl Hegel {
    // Mirrors the real crate's builder entry point, which returns a runner
    // rather than `Self`. Task 9 gates CI on `clippy -D warnings`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<F: Fn(&TestCase)>(f: F) -> Runner<F> {
        Runner(f)
    }
}

pub struct Runner<F>(F);

impl<F: Fn(&TestCase)> Runner<F> {
    pub fn run(self) {
        (self.0)(&TestCase);
    }
}
