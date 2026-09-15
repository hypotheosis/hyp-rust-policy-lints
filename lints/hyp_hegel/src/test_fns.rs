use clippy_utils::res::MaybeResPath;
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::{Closure, ConstItemRhs, ExprKind, ItemKind, def_id::DefId};
use rustc_lint::LateContext;

/// Recover the `DefId` of every `#[test]` function in the crate.
///
/// By the time a late lint pass runs, `#[test]` is gone: the test harness has
/// lowered each test into a generated `const` of type `test::TestDescAndFn`
/// whose `testfn` field holds `StaticTestFn(|| assert_test_result(the_fn()))`.
/// We walk those constants and dig the real function back out.
///
/// Returned as a set: `check_item` membership-tests it once per item rustc
/// visits, which a linear scan would make quadratic on a large crate.
///
/// Returns an empty set unless `--test` was passed to rustc.
pub fn find_test_fns(cx: &LateContext<'_>) -> FxHashSet<DefId> {
    let mut test_fns = FxHashSet::default();
    for item_id in cx.tcx.hir_free_items() {
        let item = cx.tcx.hir_item(item_id);
        if let ItemKind::Const(_ident, _generics, ty, ConstItemRhs::Body(const_body_id)) = item.kind
            && let Some(ty_def_id) = ty.basic_res().opt_def_id()
            && cx.tcx.def_path_str(ty_def_id).ends_with("TestDescAndFn")
            && let const_body = cx.tcx.hir_body(const_body_id)
            && let ExprKind::Struct(_, fields, _) = const_body.value.kind
            && let Some(testfn) = fields.iter().find(|field| field.ident.as_str() == "testfn")
            // Callee is `self::test::StaticTestFn`.
            && let ExprKind::Call(_, [arg]) = testfn.expr.kind
            && let ExprKind::Closure(Closure { body: closure_body_id, .. }) = arg.kind
            && let closure_body = cx.tcx.hir_body(*closure_body_id)
            // Callee is `self::test::assert_test_result`.
            && let ExprKind::Call(_, [arg]) = closure_body.value.kind
            // Callee is the test function itself.
            && let ExprKind::Call(callee, _) = arg.kind
            && let Some(callee_def_id) = callee.basic_res().opt_def_id()
        {
            test_fns.insert(callee_def_id);
        }
    }
    test_fns
}
