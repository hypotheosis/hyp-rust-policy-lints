#![feature(rustc_private)]
#![warn(unused_extern_crates)]

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod hegel_detect;
mod hegel_tests;
mod test_fns;

#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[
        hegel_tests::NON_HEGEL_TEST,
        hegel_tests::HEGEL_EXEMPTION_WITHOUT_JUSTIFICATION,
    ]);
    lint_store.register_late_lint_pass(Box::new(|_| Box::<hegel_tests::HegelTests>::default()));
}
