use rustc_lint::LateContext;
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
