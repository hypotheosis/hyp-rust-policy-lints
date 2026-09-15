use crate::test_fns::find_test_fns;
use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::def_id::DefId;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, impl_lint_pass};

declare_lint! {
    /// ### What it does
    ///
    /// Checks for test functions that do not use the hegel property-testing
    /// framework. For this lint to be effective, `--all-targets` must be passed
    /// to `cargo dylint`.
    ///
    /// ### Why is this bad?
    ///
    /// Example-based tests only exercise the cases someone thought to write.
    /// Property-based tests explore combinations and boundary conditions that
    /// humans do not think of, and shrink failures to minimal counterexamples.
    ///
    /// ### Example
    ///
    /// ```rust
    /// #[test]
    /// fn addition_works() {
    ///     assert_eq!(2 + 2, 4);
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// #[hegel::test]
    /// fn addition_commutes(tc: hegel::TestCase) {
    ///     let a = tc.draw(hegel::generators::integers::<i64>());
    ///     let b = tc.draw(hegel::generators::integers::<i64>());
    ///     assert_eq!(a.wrapping_add(b), b.wrapping_add(a));
    /// }
    /// ```
    pub NON_HEGEL_TEST,
    Deny,
    "test does not use the hegel property-testing framework"
}

#[derive(Default)]
pub struct HegelTests {
    test_fns: Vec<DefId>,
}

impl_lint_pass!(HegelTests => [NON_HEGEL_TEST]);

impl<'tcx> LateLintPass<'tcx> for HegelTests {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        self.test_fns = find_test_fns(cx);

        for &def_id in &self.test_fns {
            let span = cx.tcx.def_span(def_id);
            span_lint_and_help(
                cx,
                NON_HEGEL_TEST,
                span,
                "test does not use the hegel property-testing framework",
                None,
                "rewrite this as a property test using `#[hegel::test]`",
            );
        }
    }
}
