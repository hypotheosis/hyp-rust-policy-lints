use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::hir::nested_filter;
use rustc_middle::ty::TypeckResults;
use rustc_span::Span;

/// Crate names that count as hegel.
///
/// These are `[lib]` names, not Cargo package names: `tcx.crate_name` reports
/// the former, and a consumer cannot change it by renaming the dependency. The
/// packages are published as `hegeltest` and `hegeltest-macros`, but declare
/// `[lib] name = "hegel"` and `[lib] name = "hegel_macros"`.
///
/// `hegel_macros` is the name the expansion chain reports for `#[hegel::test]`.
/// `hegel` is what the builder form (Task 5) resolves to. Both are required:
/// omitting `hegel_macros` inverts the lint, making it fire on every genuine
/// hegel test.
const HEGEL_CRATE_NAMES: &[&str] = &["hegel", "hegel_macros"];

/// Does this span's macro-expansion chain pass through the hegel crate?
///
/// `#[hegel::test]` expands to a plain `#[test]` function, so the generated
/// test lives *inside* the hegel proc-macro expansion. Walking outward from the
/// span to the root context finds it. Also catches `#[hegel::state_machine]`
/// for free.
pub fn expansion_chain_includes_hegel(cx: &LateContext<'_>, span: Span) -> bool {
    let mut span = span;
    while !span.ctxt().is_root() {
        let data = span.ctxt().outer_expn_data();
        if let Some(macro_def_id) = data.macro_def_id
            && HEGEL_CRATE_NAMES.contains(&cx.tcx.crate_name(macro_def_id.krate).as_str())
        {
            return true;
        }
        span = data.call_site;
    }
    false
}

/// Does this function's body call into the hegel crate?
///
/// Catches the builder form, which uses a plain `#[test]` and therefore leaves
/// no trace in the expansion chain:
///
/// ```ignore
/// #[test]
/// fn t() { Hegel::new(|tc| { let n: i64 = tc.draw(); }).run(); }
/// ```
///
/// "Calls into hegel" means an expression that *resolves* to an item in a
/// hegel crate. Naming a hegel type in a type annotation does not count, and
/// neither does anything a non-hegel macro expands to: `assert_eq!` resolves
/// into `core`.
pub fn body_calls_hegel(cx: &LateContext<'_>, def_id: LocalDefId) -> bool {
    let Some(body) = cx.tcx.hir_maybe_body_owned_by(def_id) else {
        return false;
    };
    // `typeck` is keyed on the typeck *root*, and closures and inline consts
    // are typeck children of the enclosing function. So this one set of
    // results covers every nested body the visitor descends into, and the
    // `hir_owner` assertion inside `TypeckResults` cannot trip.
    let typeck = cx.tcx.typeck(def_id);
    let mut finder = HegelCallFinder {
        cx,
        typeck,
        found: false,
    };
    finder.visit_expr(body.value);
    finder.found
}

struct HegelCallFinder<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
    typeck: &'tcx TypeckResults<'tcx>,
    found: bool,
}

impl HegelCallFinder<'_, '_> {
    fn is_hegel_def(&self, def_id: DefId) -> bool {
        HEGEL_CRATE_NAMES.contains(&self.cx.tcx.crate_name(def_id.krate).as_str())
    }
}

impl<'tcx> Visitor<'tcx> for HegelCallFinder<'_, 'tcx> {
    // The builder form buries `tc.draw()` inside a closure, which is a
    // separate body that the default `nested_filter::None` would skip. `INTER`
    // stays false, so nested *items* — which have their own typeck root — are
    // not entered.
    type NestedFilter = nested_filter::OnlyBodies;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if self.found {
            return;
        }

        // Anything resolved through type inference: method calls such as
        // `.run()` and `tc.draw()`, and type-relative paths such as
        // `Hegel::new`, whose `new` segment is only resolvable once `Hegel`'s
        // type is known.
        if let Some(def_id) = self.typeck.type_dependent_def_id(expr.hir_id)
            && self.is_hegel_def(def_id)
        {
            self.found = true;
            return;
        }

        // Paths the resolver settled without inference: a free function such
        // as `hegel::check`, or a unit struct or constant. `Hegel::new` also
        // arrives here, but the branch above has already claimed it.
        if let ExprKind::Path(qpath) = &expr.kind
            && let Some(def_id) = self.typeck.qpath_res(qpath, expr.hir_id).opt_def_id()
            && self.is_hegel_def(def_id)
        {
            self.found = true;
            return;
        }

        walk_expr(self, expr);
    }
}
