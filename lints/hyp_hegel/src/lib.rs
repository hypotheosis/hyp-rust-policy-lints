#![feature(rustc_private)]
#![warn(unused_extern_crates)]

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;

pub mod test_fns;

use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::lint::LintLevelSource;
use rustc_session::lint::Level;
use rustc_session::{declare_lint, impl_lint_pass};

declare_lint! {
    /// ### What it does
    /// Temporary probe used to confirm unstable rustc APIs. Removed in Task 3.
    ///
    /// ### Why is this bad?
    /// It is not; this lint never fires.
    ///
    /// ### Example
    /// ```rust
    /// // nothing
    /// ```
    /// Use instead:
    /// ```rust
    /// // nothing
    /// ```
    pub HYP_PROBE,
    Warn,
    "temporary API probe"
}

#[derive(Default)]
pub struct Probe;
impl_lint_pass!(Probe => [HYP_PROBE]);

impl<'tcx> LateLintPass<'tcx> for Probe {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        for path in test_fns::probe_const_type_paths(cx) {
            eprintln!("PROBE const ty: {path}");
        }

        for def_id in test_fns::find_test_fns(cx) {
            let span = cx.tcx.def_span(def_id);
            eprintln!("PROBE test fn: {}", cx.tcx.def_path_str(def_id));

            // Question 1: what crate name does the expansion chain report?
            let mut s = span;
            while !s.ctxt().is_root() {
                let data = s.ctxt().outer_expn_data();
                if let Some(macro_def_id) = data.macro_def_id {
                    eprintln!(
                        "PROBE   expn crate: {} (kind {:?})",
                        cx.tcx.crate_name(macro_def_id.krate),
                        data.kind
                    );
                } else {
                    eprintln!("PROBE   expn (no macro_def_id) kind {:?}", data.kind);
                }
                s = data.call_site;
            }

            // Question 2: what shape does the lint-level query return?
            if let Some(local) = def_id.as_local() {
                let hir_id = cx.tcx.local_def_id_to_hir_id(local);
                let spec = cx.tcx.lint_level_spec_at_node(HYP_PROBE, hir_id);
                eprintln!("PROBE   spec debug: {spec:?}");
                eprintln!("PROBE   spec.level(): {:?}", spec.level());
                match (spec.level(), spec.src) {
                    (Level::Allow, LintLevelSource::Node { name, span: attr_span, reason }) => {
                        eprintln!(
                            "PROBE   ALLOW via Node name={name} reason={reason:?} attr_span={attr_span:?}"
                        );
                    }
                    (level, src) => {
                        eprintln!("PROBE   level={level:?} src={src:?}");
                    }
                }
            }
        }
    }
}

#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[HYP_PROBE]);
    lint_store.register_late_lint_pass(Box::new(|_| Box::<Probe>::default()));
}
