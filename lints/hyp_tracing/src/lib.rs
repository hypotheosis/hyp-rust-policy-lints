#![feature(rustc_private)]
#![warn(unused_extern_crates)]

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

extern crate rustc_lint;
extern crate rustc_session;

mod tracing_fns;

#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[
        tracing_fns::UNINSTRUMENTED_FN,
        tracing_fns::INSTRUMENT_EXEMPTION_WITHOUT_JUSTIFICATION,
    ]);
    lint_store.register_late_lint_pass(Box::new(|_| Box::<tracing_fns::TracingFns>::default()));
}
