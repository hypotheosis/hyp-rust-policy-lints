use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{Body, Expr};
use rustc_lint::LateContext;
use rustc_middle::hir::nested_filter;
use rustc_span::Span;

/// The `[lib]` name of the crate that defines `#[tracing::instrument]`.
///
/// `tcx.crate_name` reports the `[lib]` name (`tracing_attributes`), not the
/// package name (`tracing-attributes`) and not whatever a consumer renamed
/// the `tracing` dependency to, so a rename cannot defeat the match.
const INSTRUMENT_CRATE: &str = "tracing_attributes";

/// Was this function body produced by `#[tracing::instrument]`?
///
/// `#[instrument]` rebuilds the function from the consumer's own signature
/// tokens, so the signature -- and with it `def_span` -- stays in the *root*
/// syntax context: walking `def_span`'s expansion chain, as `hyp_hegel` does
/// for `#[hegel::test]`, finds nothing. The body is different: the
/// `let __tracing_attr_span; ...` prologue (sync) and the
/// `let __tracing_instrument_future = ...` wrapper (async) are emitted in the
/// attribute's own expansion context. So the body is searched for any
/// expression whose span passes through that expansion; every instrumented
/// shape contains one, so `let` statements need no separate check.
///
/// For an instrumented function the first thing visited already matches, so
/// the common case is cheap; only a function about to be *reported* pays for
/// a full walk of its body.
pub fn body_is_instrumented<'tcx>(cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) -> bool {
    let mut finder = InstrumentFinder { cx, found: false };
    finder.visit_expr(body.value);
    finder.found
}

/// Does `span`'s macro-expansion chain pass through `tracing_attributes`?
fn from_instrument(cx: &LateContext<'_>, mut span: Span) -> bool {
    while !span.ctxt().is_root() {
        let data = span.ctxt().outer_expn_data();
        if let Some(macro_def_id) = data.macro_def_id
            && cx.tcx.crate_name(macro_def_id.krate).as_str() == INSTRUMENT_CRATE
        {
            return true;
        }
        span = data.call_site;
    }
    false
}

struct InstrumentFinder<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
    found: bool,
}

impl<'tcx> Visitor<'tcx> for InstrumentFinder<'_, 'tcx> {
    // An `async fn`'s body is a coroutine closure -- a separate body the
    // default `nested_filter::None` would skip. `OnlyBodies` enters it, but
    // not nested *items*: a `fn` declared inside an instrumented function is
    // its own function and is checked on its own.
    type NestedFilter = nested_filter::OnlyBodies;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if self.found {
            return;
        }
        if from_instrument(self.cx, expr.span) {
            self.found = true;
            return;
        }
        walk_expr(self, expr);
    }
}
