# Hegel Policy Lints Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a versioned `cargo-dylint` lint library that enforces "every test uses the hegel property-testing framework", consumable by any hypotheosis Rust workspace via one `workspace.metadata.dylint` entry and one GitHub Action step.

**Architecture:** A single `cdylib` crate (`lints/hyp_hegel`) registers three lints driven by one `LateLintPass`. Because `#[test]` is already expanded away by the time a late pass runs, test functions are recovered by walking the generated `test::TestDescAndFn` constants. Hegel usage is detected by walking each test's macro-expansion chain for the hegel crate, with a body-scan fallback for the non-macro builder form. Exemptions are policed by querying the lint level directly so that an unjustified `allow` can be reported under a *different* lint.

**Tech Stack:** Rust nightly-2026-07-09 with `rustc_private`, dylint 6.0.4 (`dylint_linting`, `dylint_testing`), `clippy_utils` pinned to rev `09382ed3c34e091d7705f96964e303b59978533c`, GitHub Actions composite action.

**Spec:** `docs/superpowers/specs/2026-09-15-hegel-policy-lints-design.md`

---

## Background you need before starting

You are almost certainly unfamiliar with dylint. Read this section; it will save you hours.

**What dylint is.** `cargo-dylint` loads dynamic libraries containing custom rustc lints and runs them over a crate. A "lint library" is a `cdylib` exporting a `register_lints` symbol. Because rustc's internals are unstable, a lint library must be compiled with one exact nightly and one exact `clippy_utils` revision. Those are pinned in `lints/rust-toolchain.toml` and `lints/Cargo.toml` and must always move together.

**Why the repo has two workspaces.** `lints/` builds on the pinned nightly. `fixtures/consumer/` builds on stable, because it stands in for a real customer workspace. They are deliberately not one workspace, and there is no repo-root `Cargo.toml`. Always `cd` into the right one.

**The linker wrapper.** Everything in `lints/` builds with `-C linker=dylint-link`. If `dylint-link` is not on `PATH` you get confusing link errors. Install it first.

**Two kinds of test, and why both exist.**
- *UI tests* (`lints/hyp_hegel/ui/`) compile a `.rs` fixture and diff rustc's stderr against a committed `.stderr` file. They test the pass logic. They do not test packaging at all.
- *E2E tests* (`fixtures/consumer/`) run real `cargo dylint` over real crates. They test the glob pattern, the cdylib entry point, the `dylint_lib` cfg, and the `--all-targets` requirement — none of which a UI test can see.

**Doc examples in `declare_lint!` must be `rust,ignore`.** The lint package is built as an `rlib` as well as a `cdylib`, so rustdoc compiles every ```` ```rust ```` block in a `declare_lint!` doc comment as a real doctest. Those examples reference `hegel::test` and friends, and the lint crate deliberately does not depend on the framework it polices — so an unannotated fence fails `cargo test` with `E0433: cannot find module or crate hegel`. The UI-test command skips doctests, so this failure is invisible until CI runs bare `cargo test`.

**`NON_HEGEL_TEST` sets `report_in_external_macro`, and must.** rustc cancels a diagnostic outright when its primary span is inside an external macro and the lint has not opted in, and *every* attribute macro counts as external. Without the flag, a `#[hegel::test]` function's diagnostic is dropped before detection even runs — and so is every `#[tokio::test]`, `#[rstest]` and `#[async_std::test]`, which are exactly the non-hegel tests the policy most needs to catch. The syntax is a bare identifier after the description string; `report_in_external_macro: true` does not compile. See the spike notes addendum.

**The UI harness links its fixtures by hand.** `dylint_testing`'s `src_base` mode does not pass `--extern` for dev-dependencies, and the sanctioned example-target route breaks under this environment's `RUSTC_WRAPPER=sccache`. `tests/ui.rs` therefore passes `--edition=2024`, `-L dependency=` and `--extern hegel=` itself. New fixtures under `ui/` are auto-discovered and need no registration.

**HIR visitors skip closures unless you opt in.** `intravisit::Visitor`'s `visit_nested_body` does nothing unless `NestedFilter::INTRA` is set, and the default is `nested_filter::None`. Any body scan that must see inside a closure needs `type NestedFilter = nested_filter::OnlyBodies` plus a `maybe_tcx` override. `body_calls_hegel` sets this. Note the failure is latent rather than loud — the `builder_form` fixture passes either way because `.run()` sits outside the closure and matches first. See the spike notes addendum.

**The `--all-targets` trap.** The lint can only see tests when the test harness is compiled. Every `cargo dylint` invocation in this project must pass `-- --all-targets`. If you forget, the lint loads, finds zero tests, reports nothing, and looks like it passed.

**Regenerating `.stderr` files.** There is no bless mode. `dylint_testing` 6.0.4 builds its `compiletest::Config` without ever setting `bless`, and `compiletest_rs` 0.11.2 hardcodes `bless: false` in its `Default` impl, so `BLESS=1` and every other env var are silently ignored.

The real workflow is to let the failing run hand you the actual output:

```bash
cd lints && cargo test --package hyp_hegel --test ui 2>&1 | grep 'saved to'
# -> Actual stderr saved to /tmp/<fixture>.stage-id.stderr
cp /tmp/<fixture>.stage-id.stderr hyp_hegel/ui/<fixture>.stderr
```

`compare_output` writes the same string to both the temp file and the diff, so the copied file is byte-identical to what a bless would have produced.

**Always read the file before copying it into place.** Accepting output you have not looked at is how a suite ends up certifying a broken lint — which on this project is a live risk, not a platitude: an inverted `non_hegel_test` would produce a perfectly self-consistent set of `.stderr` files.

---

## File structure

| Path | Responsibility |
|---|---|
| `lints/rust-toolchain.toml` | Pins the nightly + `rustc-dev`/`llvm-tools-preview` |
| `lints/.cargo/config.toml` | Sets the `dylint-link` linker wrapper |
| `lints/Cargo.toml` | Lint workspace root; shared dep + lint config |
| `lints/hyp_hegel/Cargo.toml` | The lint package (cdylib + rlib) |
| `lints/hyp_hegel/src/lib.rs` | `register_lints` entry point only |
| `lints/hyp_hegel/src/hegel_tests.rs` | Lint declarations and the `LateLintPass` |
| `lints/hyp_hegel/src/test_fns.rs` | Recovering test functions from `TestDescAndFn` |
| `lints/hyp_hegel/src/hegel_detect.rs` | Expansion-chain walk + body-scan fallback |
| `lints/hyp_hegel/hegel_stub/` | Facade stub, package `hegeltest` with `[lib] name = "hegel"` |
| `lints/hyp_hegel/hegel_stub_macros/` | Proc-macro stub, package `hegeltest-macros` with `[lib] name = "hegel_macros"` |
| `lints/hyp_hegel/ui/*.rs` + `.stderr` | UI fixtures |
| `fixtures/consumer/` | Stable-toolchain e2e workspace (`good`, `bad`, `exempt_ok`, `exempt_bad`) |
| `scripts/e2e.sh` | Asserts expected pass/fail per fixture package |
| `scripts/verify-pin.sh` | Compares consumer's pinned tag against the action ref |
| `action.yml` | Root composite action |
| `.github/workflows/ci.yml` | `lints` + `e2e` jobs |
| `.github/workflows/release.yml` | Tag validation through the real git path |

Splitting `test_fns.rs` and `hegel_detect.rs` out of `hegel_tests.rs` keeps each file to one job: recovering tests, recognising hegel, and deciding what to report. They are the three things most likely to need independent debugging.

---

## Task 1: Scaffold the lints workspace

**Files:**
- Create: `lints/rust-toolchain.toml`, `lints/.cargo/config.toml`, `lints/Cargo.toml`, `lints/hyp_hegel/Cargo.toml`, `lints/hyp_hegel/src/lib.rs`
- Modify: `.gitignore`

- [ ] **Step 1: Install the dylint tooling**

```bash
cargo install --locked cargo-dylint@6.0.4 dylint-link@6.0.4
```

Expected: both binaries installed. Verify with `cargo dylint --version` printing `6.0.4`.

- [ ] **Step 2: Create the toolchain and linker config**

```bash
mkdir -p lints/hyp_hegel/src lints/.cargo
cat > lints/rust-toolchain.toml <<'EOF'
[toolchain]
channel = "nightly-2026-07-09"
components = ["llvm-tools-preview", "rustc-dev", "rustfmt", "clippy"]
EOF
cat > lints/.cargo/config.toml <<'EOF'
[target.'cfg(all())']
rustflags = ["-C", "linker=dylint-link"]
EOF
```

`rustfmt` and `clippy` go beyond what the dylint template lists, because CI runs
`cargo fmt --check` and `cargo clippy` against this nightly. They are present by
default only when rustup is configured `profile = "default"`; on a runner image
using `profile = "minimal"` the auto-installed toolchain has neither, and the
Format step fails with "rustfmt is not installed for the toolchain". Listing them
keeps the pin self-describing rather than papering over it with a `rustup
component add` step in the workflow.

- [ ] **Step 3: Create the workspace root**

`lints/Cargo.toml`:

```toml
[workspace]
members = ["*"]
exclude = [".cargo", "target"]
resolver = "2"

[workspace.dependencies]
clippy_utils = { git = "https://github.com/rust-lang/rust-clippy", rev = "09382ed3c34e091d7705f96964e303b59978533c" }
dylint_linting = "6.0"
dylint_testing = "6.0"

[workspace.lints.rust.unexpected_cfgs]
level = "deny"
check-cfg = ["cfg(dylint_lib, values(any()))"]
```

`"target"` must be in `exclude`. Without it, `members = ["*"]` matches the
`lints/target/` directory as soon as the first build creates it, and every
subsequent build fails with `failed to load manifest for workspace member`.
The first build from a clean tree succeeds — `target/` does not exist yet when
the glob is resolved — so this defect hides from any check that only builds
once. Upstream dylint avoids it instead by setting `target-dir` outside the
glob root in `.cargo/config.toml`; excluding `target` is equivalent and keeps
the build output where the rest of this plan expects to find it.

- [ ] **Step 4: Create the lint package**

`lints/hyp_hegel/Cargo.toml`:

```toml
[package]
name = "hyp_hegel"
version = "0.1.0"
description = "hypotheosis policy lints: require hegel property tests"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
clippy_utils = { workspace = true }
dylint_linting = { workspace = true }

[dev-dependencies]
dylint_testing = { workspace = true }

[features]
rlib = ["dylint_linting/constituent"]

[lints]
workspace = true

[package.metadata.rust-analyzer]
rustc_private = true
```

- [ ] **Step 5: Create a stub entry point that compiles**

`lints/hyp_hegel/src/lib.rs`:

```rust
#![feature(rustc_private)]
#![warn(unused_extern_crates)]

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(_sess: &rustc_session::Session, _lint_store: &mut rustc_lint::LintStore) {}
```

- [ ] **Step 6: Verify it builds**

```bash
cd lints && cargo build
```

Expected: `Finished` with no errors, and `lints/target/debug/libhyp_hegel.so` exists. If you get a linker error, `dylint-link` is not on `PATH` — revisit Step 1.

- [ ] **Step 7: Add .gitignore and commit**

```bash
cd "$(git rev-parse --show-toplevel)"
printf 'target/\n' > .gitignore
git add .gitignore lints/
git commit -m "feat: scaffold dylint lint workspace"
```

Note: `lints/Cargo.lock` is committed deliberately — it is part of the reproducibility contract for consumers.

---

## Task 2: Spike — confirm the unstable APIs

The spec's §8 lists three things that must be settled against the real compiler rather than guessed. This task answers all three and writes the answers down. **Do not skip it**; Tasks 3–7 all depend on the results.

> **Already executed.** The findings are recorded in `docs/superpowers/notes/2026-09-15-api-spike.md`, and Tasks 4–8 below have been amended to match them. The probe code in this task is preserved as written for the record, and contains two APIs the spike proved wrong (`lint_level_at_node`, and package-name-based crate matching). Read the notes, not this task, for ground truth.

**Files:**
- Create: `lints/hyp_hegel/src/test_fns.rs`, `docs/superpowers/notes/2026-09-15-api-spike.md`
- Modify: `lints/hyp_hegel/src/lib.rs`

- [ ] **Step 1: Write the test-function recovery module**

This is adapted from dylint's own `non_thread_safe_call_in_test`, which is known to compile against this exact nightly and `clippy_utils` rev. The one deliberate change: matching `test::TestDescAndFn` by `def_path_str` instead of via the unpublished `dylint_internal` crate.

`lints/hyp_hegel/src/test_fns.rs`:

```rust
use clippy_utils::res::{MaybeDef, MaybeResPath};
use rustc_hir::{Closure, ConstItemRhs, ExprKind, ItemKind, def_id::DefId};
use rustc_lint::LateContext;

/// Recover the `DefId` of every `#[test]` function in the crate.
///
/// By the time a late lint pass runs, `#[test]` is gone: the test harness has
/// lowered each test into a generated `const` of type `test::TestDescAndFn`
/// whose `testfn` field holds `StaticTestFn(|| assert_test_result(the_fn()))`.
/// We walk those constants and dig the real function back out.
///
/// Returns an empty vec unless `--test` was passed to rustc.
pub fn find_test_fns(cx: &LateContext<'_>) -> Vec<DefId> {
    let mut test_fns = Vec::new();
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
            test_fns.push(callee_def_id);
        }
    }
    test_fns
}
```

- [ ] **Step 2: Wire up a temporary probe pass that prints what we need**

Replace `lints/hyp_hegel/src/lib.rs` with:

```rust
#![feature(rustc_private)]
#![warn(unused_extern_crates)]

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

pub mod test_fns;

use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, impl_lint_pass};

declare_lint! {
    /// ### What it does
    /// Temporary probe used to confirm unstable rustc APIs. Removed in Task 3.
    ///
    /// ### Why is this bad?
    /// It is not; this lint never fires.
    ///
    /// ### Example
    /// ```rust,ignore
    /// // nothing
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// // nothing
    /// ```
    pub HYP_PROBE,
    Allow,
    "temporary API probe"
}

#[derive(Default)]
pub struct Probe;
impl_lint_pass!(Probe => [HYP_PROBE]);

impl<'tcx> LateLintPass<'tcx> for Probe {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        for def_id in test_fns::find_test_fns(cx) {
            let span = cx.tcx.def_span(def_id);
            eprintln!("PROBE test fn: {}", cx.tcx.def_path_str(def_id));

            // Question 1: what crate name does the expansion chain report?
            let mut s = span;
            while !s.ctxt().is_root() {
                let data = s.ctxt().outer_expn_data();
                if let Some(macro_def_id) = data.macro_def_id {
                    eprintln!("PROBE   expn crate: {}", cx.tcx.crate_name(macro_def_id.krate));
                }
                s = data.call_site;
            }

            // Question 2: what shape does lint_level_at_node return?
            if let Some(local) = def_id.as_local() {
                let hir_id = cx.tcx.local_def_id_to_hir_id(local);
                let level = cx.tcx.lint_level_at_node(HYP_PROBE, hir_id);
                eprintln!("PROBE   level: {level:?}");
            }
        }
    }
}

#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[HYP_PROBE]);
    lint_store.register_late_lint_pass(Box::new(|_| Box::<Probe>::default()));
}
```

- [ ] **Step 3: Build and fix compilation errors**

```bash
cd lints && cargo build 2>&1 | tail -40
```

Expected on first try: likely one or two errors, because `lint_level_at_node`'s return type changed across nightlies. Two known shapes:
- Older: returns `(Level, LintLevelSource)` — destructure the tuple.
- Newer: returns `LintLevelSource`-bearing struct `LevelAndSource { level, lint_id, src }` — use `.level` and `.src`.

Fix to whichever compiles. Note also that `rustc_middle::lint::LintLevelSource` may need importing. Iterate until `cargo build` succeeds.

- [ ] **Step 4: Create a scratch crate to probe against**

```bash
mkdir -p /tmp/hegel-probe/src && cd /tmp/hegel-probe
cat > Cargo.toml <<'EOF'
[package]
name = "hegel-probe"
version = "0.1.0"
edition = "2021"

[dev-dependencies]
hegel = { package = "hegeltest", version = "0.14" }
EOF
cat > src/lib.rs <<'EOF'
#[cfg(test)]
mod tests {
    #[test]
    fn plain() {
        assert_eq!(1 + 1, 2);
    }

    #[allow(dead_code, reason = "probing the reason field")]
    #[test]
    fn with_reason() {
        assert_eq!(2 + 2, 4);
    }

    #[hegel::test]
    fn prop(tc: hegel::TestCase) {
        let a = tc.draw(hegel::generators::integers::<i64>());
        assert_eq!(a, a);
    }
}
EOF
```

- [ ] **Step 5: Run the probe**

```bash
cd /tmp/hegel-probe
DYLINT_LIBRARY_PATH="$(git -C /home/pgrace/repos/hypotheosis/hyp-rust-policy-lints/.worktrees/hyp-rust-policy-lints-hyp-dylint rev-parse --show-toplevel)/lints/target/debug" \
  cargo dylint --all -- --all-targets 2>&1 | grep PROBE
```

Expected: three `PROBE test fn:` lines. The `prop` test must show at least one `PROBE   expn crate:` line. Record what crate name it prints — `hegel` or `hegeltest`.

If `hegeltest` fails to build on this nightly, drop it from the scratch crate and record that fact; Task 4 then relies solely on the stub, and the real-crate check moves to the e2e fixture in Task 8.

- [ ] **Step 6: Record the answers**

Write `docs/superpowers/notes/2026-09-15-api-spike.md` containing, in plain prose:
1. The crate name(s) the expansion chain reports for `#[hegel::test]`.
2. The exact return type and destructuring of `lint_level_at_node`, and the exact path to `LintLevelSource`.
3. The exact `def_path_str` observed for the test-descriptor type, confirming the `ends_with("TestDescAndFn")` match is correct.
4. Whether `hegeltest` 0.14 builds on nightly-2026-07-09.

- [ ] **Step 7: Commit**

```bash
cd "$(git rev-parse --show-toplevel)"
git add lints/ docs/superpowers/notes/
git commit -m "chore: spike unstable rustc APIs for test recovery"
```

---

## Task 3: Flag a plain `#[test]`

First real lint behaviour, driven by a UI test.

**Files:**
- Create: `lints/hyp_hegel/src/hegel_tests.rs`, `lints/hyp_hegel/ui/plain_test.rs`, `lints/hyp_hegel/tests/ui.rs`
- Modify: `lints/hyp_hegel/src/lib.rs`

- [ ] **Step 1: Write the failing UI fixture**

`lints/hyp_hegel/ui/plain_test.rs`:

```rust
#[test]
fn plain_assertion() {
    assert_eq!(1 + 1, 2);
}

fn main() {}
```

- [ ] **Step 2: Write the UI test runner**

`lints/hyp_hegel/tests/ui.rs`:

```rust
#[test]
fn ui() {
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui")
        .rustc_flags(["--test"])
        .run();
}
```

`--test` is mandatory: without it the harness never generates the `TestDescAndFn` constants that `find_test_fns` looks for, and every fixture silently passes.

- [ ] **Step 3: Run the test to verify it fails**

```bash
cd lints && cargo test --package hyp_hegel --test ui
```

Expected: FAIL. `plain_test.rs` produces no diagnostics but no `.stderr` exists yet, or the lint is not registered. Either way it must not pass.

- [ ] **Step 4: Write the lint**

`lints/hyp_hegel/src/hegel_tests.rs`:

```rust
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
    /// ```rust,ignore
    /// #[test]
    /// fn addition_works() {
    ///     assert_eq!(2 + 2, 4);
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust,ignore
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
```

Emitting from `check_crate` is a temporary simplification for this slice only, and **Task 6 must move it to `check_item`**. `LateContext` resolves lint levels against `last_node_with_lint_attrs`, which during `check_crate` is `CRATE_HIR_ID`, so a pass that emits there ignores every node-level `#[allow]`/`#[warn]`/`#[deny]`/`#[expect]`. `find_test_fns` genuinely does need the whole-crate scan, so the finished shape is: collect in `check_crate`, judge and emit in `check_item`, summarise in `check_crate_post`. See the spike notes addendum.

- [ ] **Step 5: Replace the probe in lib.rs**

`lints/hyp_hegel/src/lib.rs`:

```rust
#![feature(rustc_private)]
#![warn(unused_extern_crates)]

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod hegel_tests;
mod test_fns;

#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[hegel_tests::NON_HEGEL_TEST]);
    lint_store.register_late_lint_pass(Box::new(|_| Box::<hegel_tests::HegelTests>::default()));
}
```

- [ ] **Step 6: Bless and inspect the expected output**

```bash
cd lints
# The run fails and prints where it saved the actual output; copy that into place.
cargo test --package hyp_hegel --test ui 2>&1 | grep 'saved to'
cp /tmp/plain_test.stage-id.stderr hyp_hegel/ui/plain_test.stderr
cat hyp_hegel/ui/plain_test.stderr
```

Expected: `plain_test.stderr` contains one `error: test does not use the hegel property-testing framework` pointing at `fn plain_assertion`, with the help line. **Read it before continuing** — if the span points somewhere unexpected, fix the lint rather than accepting the blessed output.

- [ ] **Step 7: Run the tests to verify they pass**

```bash
cd lints && cargo test -p hyp_hegel
```

Expected: PASS, including the doctests. Use the bare form rather than
`--test ui` — the latter skips doctests, and the `declare_lint!` doc comments
are compiled as doctests by rustdoc.

- [ ] **Step 8: Commit**

```bash
cd "$(git rev-parse --show-toplevel)"
git add lints/
git commit -m "feat: flag tests that do not use hegel"
```

---

## Task 4: Recognise `#[hegel::test]` via the expansion chain

**Files:**
- Create: `lints/hyp_hegel/src/hegel_detect.rs`, `lints/hyp_hegel/hegel_stub/{Cargo.toml,src/lib.rs}`, `lints/hyp_hegel/hegel_stub_macros/{Cargo.toml,src/lib.rs}`, `lints/hyp_hegel/ui/hegel_test.rs`
- Modify: `lints/hyp_hegel/Cargo.toml`, `lints/hyp_hegel/src/lib.rs`, `lints/hyp_hegel/src/hegel_tests.rs`

- [ ] **Step 1: Create the stub hegel crates**

UI fixtures must not depend on the network or on hegel's test server. The stub provides just enough for `#[hegel::test]` to expand.

It is split into a proc-macro crate and a facade that re-exports it, mirroring how the real crate is built. The Task 2 spike established the facts this depends on:

**`tcx.crate_name()` returns a crate's `[lib]` name, never its Cargo package name.** The real crates are published as packages `hegeltest` and `hegeltest-macros`, but declare `[lib] name = "hegel"` and `[lib] name = "hegel_macros"`. `hegel_macros` is the *only* name the expansion chain ever reports for `#[hegel::test]`, and a consumer cannot change it by renaming the dependency.

The stubs therefore set `[lib] name` explicitly. A stub matching on package names would validate a crate name that never occurs in production, and the UI tests would happily confirm a lint that is inverted in the real world.

`lints/hyp_hegel/hegel_stub_macros/Cargo.toml`:

```toml
[package]
name = "hegeltest-macros"
version = "0.14.0"
edition = "2021"
publish = false

[lib]
name = "hegel_macros"
proc-macro = true
```

`[lib] name` is what the lint sees. Do not drop it.

`lints/hyp_hegel/hegel_stub_macros/src/lib.rs`:

```rust
//! Minimal stand-in for the real hegel attribute macros, used only by UI
//! fixtures.
//!
//! Expands `#[hegel::test]` into a plain `#[test]` with the `tc` parameter
//! dropped, which is enough to reproduce the macro-expansion chain the lint
//! inspects.
//!
//! # Constraint on fixtures
//!
//! This parses the item as a string, not as tokens: it finds the name after
//! the first `"fn "` and the body between the first `{` and the last `}`. So a
//! fixture must not place a doc comment, attribute, or anything else
//! containing `fn `, `{` or `}` **above** the annotated function — the match
//! point shifts into that text and the extraction is corrupted.
//!
//! In every case tried the corruption yields unparseable Rust and panics at
//! expansion time, which is ugly but safe. Do not rely on that: keep fixtures
//! plain, and if one needs commentary, put it below the function or in the
//! `.stderr`.

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = item.to_string();
    let name = input
        .split("fn ")
        .nth(1)
        .and_then(|rest| rest.split('(').next())
        .unwrap_or("generated")
        .trim()
        .to_string();
    let open = input.find('{').expect("test function must have a body");
    let body = &input[open + 1..input.rfind('}').expect("unbalanced body")];
    format!("#[test] fn {name}() {{ let tc = hegel::TestCase; let _ = &tc; {body} }}")
        .parse()
        .unwrap()
}
```

`lints/hyp_hegel/hegel_stub/Cargo.toml`:

```toml
[package]
name = "hegeltest"
version = "0.14.0"
edition = "2021"
publish = false

[lib]
name = "hegel"

[dependencies]
hegeltest-macros = { path = "../hegel_stub_macros" }
```

`lints/hyp_hegel/hegel_stub/src/lib.rs`:

```rust
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
```

- [ ] **Step 2: Add the stub as a dev-dependency**

In `lints/hyp_hegel/Cargo.toml`, under `[dev-dependencies]`:

```toml
hegeltest = { path = "hegel_stub" }
```

No rename is needed or possible. Cargo binds a dependency under its `[lib]` name, so this is already reachable as `hegel::` in fixtures. Writing `use hegeltest as hegel;` does **not** compile — `error[E0432]: unresolved import hegeltest`.

- [ ] **Step 3: Write the failing UI fixture**

`lints/hyp_hegel/ui/hegel_test.rs`:

```rust
#[hegel::test]
fn addition_commutes(tc: hegel::TestCase) {
    let a: i64 = tc.draw();
    assert_eq!(a.wrapping_add(0), a);
}

fn main() {}
```

No `use` needed: the dev-dependency's `[lib] name` is already `hegel`.

- [ ] **Step 4: Run to verify it fails**

```bash
cd lints && cargo test --package hyp_hegel --test ui
```

Expected: FAIL. `hegel_test.rs` currently triggers `non_hegel_test`, because Task 3's lint flags every test unconditionally.

- [ ] **Step 5: Write the detection module**

`lints/hyp_hegel/src/hegel_detect.rs`:

```rust
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
/// span to the root context finds it.
///
/// Note `#[hegel::state_machine]` is *not* detected here, and does not need to
/// be: it derives a `StateMachine` impl from an `impl` block and generates no
/// test at all, so `find_test_fns` never sees it. A state machine is driven by
/// a separate test — canonically `#[hegel::test] fn t(tc) {
/// hegel::stateful::run(m, tc) }` — and it is that test which this catches.
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
```

- [ ] **Step 6: Use it in the pass**

In `lints/hyp_hegel/src/hegel_tests.rs`, add the import and guard the emission:

```rust
use crate::hegel_detect::expansion_chain_includes_hegel;
```

and replace the body of the `for` loop in `check_crate` with:

```rust
        for &def_id in &self.test_fns {
            let span = cx.tcx.def_span(def_id);
            if expansion_chain_includes_hegel(cx, span) {
                continue;
            }
            span_lint_and_help(
                cx,
                NON_HEGEL_TEST,
                span,
                "test does not use the hegel property-testing framework",
                None,
                "rewrite this as a property test using `#[hegel::test]`",
            );
        }
```

Add `mod hegel_detect;` to `lints/hyp_hegel/src/lib.rs` alongside the other modules.

- [ ] **Step 7: Run to verify it passes**

```bash
cd lints
# Regenerate any .stderr that changed: the failing run prints "Actual stderr
# saved to <path>" for each fixture; read each one, then copy it into
# hyp_hegel/ui/. There is no bless mode -- see the Background section.
cargo test --package hyp_hegel --test ui 2>&1 | grep 'saved to'
# ...copy each into place, then confirm the whole package is green.
# Use the bare form, not `--test ui`: it also runs the doctests generated from
# the `declare_lint!` doc comments, which `--test ui` silently skips.
cargo test -p hyp_hegel
```

Expected: `hegel_test.stderr` is empty (or absent), `plain_test.stderr` unchanged, test PASSes. If `plain_test.stderr` changed, something regressed — investigate before committing.

- [ ] **Step 8: Commit**

```bash
cd "$(git rev-parse --show-toplevel)"
git add lints/
git commit -m "feat: recognise hegel tests via macro expansion chain"
```

---

## Task 5: Recognise the builder form via body scan

The documented builder form uses a plain `#[test]` and calls `Hegel::new(..).run()` in the body. It has no hegel expansion at all, so Task 4's check reports it as a violation. That is a false positive and must be fixed.

**Files:**
- Create: `lints/hyp_hegel/ui/builder_form.rs`, `lints/hyp_hegel/ui/closure_only.rs`
- Modify: `lints/hyp_hegel/src/hegel_detect.rs`, `lints/hyp_hegel/src/hegel_tests.rs`

- [ ] **Step 1: Confirm the crate-name list already covers the builder form**

No change should be needed. The facade stub from Task 4 exports `Hegel` and `TestCase` from a crate whose `[lib] name` is `hegel`, which `HEGEL_CRATE_NAMES` already contains.

Verify before writing any code:

```bash
grep -n 'HEGEL_CRATE_NAMES' lints/hyp_hegel/src/hegel_detect.rs
```

Expected: `const HEGEL_CRATE_NAMES: &[&str] = &["hegel", "hegel_macros"];`

If it says anything else, stop and reconcile against `docs/superpowers/notes/2026-09-15-api-spike.md` — a wrong list here silently inverts the lint.

- [ ] **Step 2: Write the failing UI fixture**

`lints/hyp_hegel/ui/builder_form.rs`:

```rust
use hegel::Hegel;

#[test]
fn built_property() {
    Hegel::new(|tc| {
        let n: i64 = tc.draw();
        assert_eq!(n, n);
    })
    .run();
}

fn main() {}
```

- [ ] **Step 3: Run to verify it fails**

```bash
cd lints && cargo test --package hyp_hegel --test ui
```

Expected: FAIL — `builder_form.rs` emits `non_hegel_test`, which is the false positive we are fixing.

- [ ] **Step 4: Implement the body scan**

Append to `lints/hyp_hegel/src/hegel_detect.rs`:

```rust
use rustc_hir::{
    Expr, ExprKind,
    def_id::LocalDefId,
    intravisit::{Visitor, walk_expr},
};

/// Does this function's body call into the hegel crate?
///
/// Catches the builder form, which uses a plain `#[test]` and therefore leaves
/// no trace in the expansion chain:
///
/// ```ignore
/// #[test]
/// fn t() { Hegel::new(|tc| { /* ... */ }).run(); }
/// ```
pub fn body_calls_hegel(cx: &LateContext<'_>, def_id: LocalDefId) -> bool {
    let typeck = cx.tcx.typeck(def_id);
    let body = cx.tcx.hir_body_owned_by(def_id);
    let mut finder = HegelCallFinder { cx, typeck, found: false };
    finder.visit_expr(body.value);
    finder.found
}

struct HegelCallFinder<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
    typeck: &'tcx rustc_middle::ty::TypeckResults<'tcx>,
    found: bool,
}

impl<'cx, 'tcx> HegelCallFinder<'cx, 'tcx> {
    fn is_hegel_def(&self, def_id: rustc_hir::def_id::DefId) -> bool {
        HEGEL_CRATE_NAMES.contains(&self.cx.tcx.crate_name(def_id.krate).as_str())
    }
}

impl<'tcx> Visitor<'tcx> for HegelCallFinder<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if self.found {
            return;
        }

        // Method calls: `.run()`, `.draw()`.
        if let Some(def_id) = self.typeck.type_dependent_def_id(expr.hir_id)
            && self.is_hegel_def(def_id)
        {
            self.found = true;
            return;
        }

        // Path expressions: `Hegel::new`.
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
```

Note `Res::opt_def_id` requires `use rustc_hir::def::Res;` if not already in scope via the `qpath_res` return type — add the import if the compiler asks for it.

- [ ] **Step 5: Use it in the pass**

In `hegel_tests.rs`, change the skip condition to:

```rust
            let is_hegel = expansion_chain_includes_hegel(cx, span)
                || def_id
                    .as_local()
                    .is_some_and(|local| body_calls_hegel(cx, local));
            if is_hegel {
                continue;
            }
```

and import `body_calls_hegel` alongside `expansion_chain_includes_hegel`.

- [ ] **Step 5b: Add a fixture that actually guards closure descent**

`builder_form.rs` cannot prove the nested filter works: `.run()` sits outside
the closure and matches first, so the fixture passes whether or not the visitor
descends. Add one whose *only* hegel reference is inside a closure.

`lints/hyp_hegel/ui/closure_only.rs`:

```rust
#[test]
fn closure_only() {
    std::iter::once(0).for_each(|_| {
        let tc = hegel::TestCase;
        let _: i64 = tc.draw();
    });
}

fn main() {}
```

No `.stderr`, so the fixture asserts no diagnostic is emitted. Verify it is
load-bearing by deleting the `type NestedFilter` line and the `maybe_tcx`
override, rebuilding, and confirming `closure_only.rs` FAILS while
`builder_form.rs` still passes. Restore afterwards.

- [ ] **Step 6: Run to verify it passes**

```bash
cd lints
# Regenerate any .stderr that changed: the failing run prints "Actual stderr
# saved to <path>" for each fixture; read each one, then copy it into
# hyp_hegel/ui/. There is no bless mode -- see the Background section.
cargo test --package hyp_hegel --test ui 2>&1 | grep 'saved to'
# ...copy each into place, then confirm the whole package is green.
# Use the bare form, not `--test ui`: it also runs the doctests generated from
# the `declare_lint!` doc comments, which `--test ui` silently skips.
cargo test -p hyp_hegel
```

Expected: PASS, with `builder_form.stderr` empty and `plain_test.stderr` unchanged. Confirm `plain_test` still fires — an over-broad body scan that matches everything would silently disable the whole lint.

- [ ] **Step 7: Commit**

```bash
cd "$(git rev-parse --show-toplevel)"
git add lints/
git commit -m "feat: recognise hegel builder form via body scan"
```

---

## Task 6: Require a justification on every exemption

`#[allow(non_hegel_test)]` suppresses `non_hegel_test` by definition, so the pass must query the lint level itself and report unjustified exemptions under a *separate* lint that the allow does not cover.

**Files:**
- Create: `lints/hyp_hegel/ui/allow_justified.rs`, `lints/hyp_hegel/ui/allow_unjustified.rs`, `lints/hyp_hegel/ui/allow_module_level.rs`
- Modify: `lints/hyp_hegel/src/hegel_tests.rs`, `lints/hyp_hegel/src/lib.rs`

- [ ] **Step 1: Write the two failing UI fixtures**

`lints/hyp_hegel/ui/allow_justified.rs`:

```rust
#[allow(non_hegel_test, reason = "asserts an exact serialized byte layout; no general property holds")]
#[test]
fn golden_wire_format() {
    assert_eq!(format!("{:?}", 1u8), "1");
}

fn main() {}
```

`lints/hyp_hegel/ui/allow_unjustified.rs`:

```rust
#[allow(non_hegel_test)]
#[test]
fn unexplained() {
    assert_eq!(1 + 1, 2);
}

fn main() {}
```

`lints/hyp_hegel/ui/allow_module_level.rs`:

```rust
#[allow(non_hegel_test, reason = "whole module is example-based by design")]
mod legacy {
    #[test]
    fn one() {
        assert_eq!(1 + 1, 2);
    }

    #[test]
    fn two() {
        assert_eq!(2 + 2, 4);
    }
}

fn main() {}
```

The Task 2 spike observed that a lint level set on a module is inherited by
every test inside it, with `LintLevelSource::Node.span` pointing at the module's
attribute. This fixture pins that behaviour down: one justified `allow` on a
module exempts every test in it, and is clean. That breadth is intentional, but
it is worth having a test that fails loudly if it ever changes.

Fixtures write a bare `#[allow(...)]` rather than the `cfg_attr(dylint_lib = ...)` form consumers use, because under `dylint_testing` the lint is always registered. Task 8's e2e fixture covers the `cfg_attr` form.

- [ ] **Step 2: Run to verify they fail**

```bash
cd lints && cargo test --package hyp_hegel --test ui
```

Expected: FAIL. `allow_unjustified.rs` currently produces no diagnostic at all (the allow silences everything), which is exactly the gap being closed.

- [ ] **Step 3: Declare the second lint**

Add to `lints/hyp_hegel/src/hegel_tests.rs`:

```rust
declare_lint! {
    /// ### What it does
    ///
    /// Checks for `allow(non_hegel_test)` that carries no `reason = "..."`.
    ///
    /// ### Why is this bad?
    ///
    /// Silencing the property-testing requirement should be a deliberate,
    /// explained decision. Requiring a written reason forces whoever adds the
    /// exemption — human or agent — to articulate why a property test does not
    /// apply, and leaves that argument in the code for the next reader.
    ///
    /// ### Example
    ///
    /// ```rust,ignore
    /// #[allow(non_hegel_test)]
    /// #[test]
    /// fn golden_wire_format() {}
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust,ignore
    /// #[allow(non_hegel_test, reason = "asserts an exact byte layout; no general property holds")]
    /// #[test]
    /// fn golden_wire_format() {}
    /// ```
    pub HEGEL_EXEMPTION_WITHOUT_JUSTIFICATION,
    Deny,
    "`allow(non_hegel_test)` without a stated reason"
}
```

Update the lint pass registration:

```rust
impl_lint_pass!(HegelTests => [NON_HEGEL_TEST, HEGEL_EXEMPTION_WITHOUT_JUSTIFICATION]);
```

- [ ] **Step 4: Branch on the lint level**

Replace the emission block in `check_crate` with:

```rust
        for &def_id in &self.test_fns {
            let span = cx.tcx.def_span(def_id);
            let is_hegel = expansion_chain_includes_hegel(cx, span)
                || def_id
                    .as_local()
                    .is_some_and(|local| body_calls_hegel(cx, local));
            if is_hegel {
                self.hegel_test_count += 1;
                continue;
            }

            let Some(local) = def_id.as_local() else {
                continue;
            };
            let hir_id = cx.tcx.local_def_id_to_hir_id(local);
            // `lint_level_spec_at_node` returns `StableLevelSpec`, whose `level`
            // field is deliberately private — read it through `.level()`. `src`
            // is public and destructures directly. Verified against this
            // nightly's `rustc_middle::lint` in the Task 2 spike; do not
            // substitute `lint_level_at_node`, which does not exist here.
            let spec = cx.tcx.lint_level_spec_at_node(NON_HEGEL_TEST, hir_id);

            match (spec.level(), spec.src) {
                // Exempted in source with a stated reason: accepted.
                (Level::Allow, LintLevelSource::Node { reason: Some(_), .. }) => {}

                // Exempted in source with no reason: report the attribute.
                // `span` covers just the lint name inside the attribute, not
                // the whole `#[allow(...)]`.
                (Level::Allow, LintLevelSource::Node { span: attr_span, .. }) => {
                    span_lint_and_help(
                        cx,
                        HEGEL_EXEMPTION_WITHOUT_JUSTIFICATION,
                        attr_span,
                        "`allow(non_hegel_test)` without a stated reason",
                        None,
                        "add `reason = \"...\"` explaining why a property test does not apply here",
                    );
                }

                // Allowed from the command line (`-A non_hegel_test`): a
                // deliberate operator decision, not a source-level exemption.
                (Level::Allow, _) => {}

                _ => {
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
```

Add the field `hegel_test_count: usize` to `HegelTests` (it is used in Task 7) and the imports:

```rust
use rustc_middle::lint::LintLevelSource;
use rustc_session::lint::Level;
```

- [ ] **Step 4b: Move emission to `check_item`**

`check_crate` keeps only the `self.test_fns = find_test_fns(cx)` scan. The
hegel check, the level query, the emission and the `hegel_test_count` increment
all move into:

```rust
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let def_id = item.owner_id.to_def_id();
        if !self.test_fns.contains(&def_id) {
            return;
        }
        // ... hegel check, level query, emission, as in Step 4 ...
    }
```

Query the level at `item.hir_id()`. `#[test]` is only legal on free functions,
so no test arrives as an `ImplItem` and `check_item` alone suffices.

Add two fixtures the rest of the suite is structurally blind to, since every
other fixture uses the default level:

- `ui/warn_level.rs` — a plain `#[test]` with `#[warn(non_hegel_test)]`. Its
  `.stderr` must say `warning:`, not `error:`.
- `ui/deny_level.rs` — `mod legacy` with an inner
  `#![allow(non_hegel_test, reason = "...")]`, one test inheriting it and one
  carrying `#[deny(non_hegel_test)]`. Exactly one error, for the opted-back-in
  test, proving the innermost attribute wins in both directions.

- [ ] **Step 5: Register the new lint**

In `lints/hyp_hegel/src/lib.rs`:

```rust
    lint_store.register_lints(&[
        hegel_tests::NON_HEGEL_TEST,
        hegel_tests::HEGEL_EXEMPTION_WITHOUT_JUSTIFICATION,
    ]);
```

- [ ] **Step 6: Run and bless**

```bash
cd lints
# Regenerate any .stderr that changed: the failing run prints "Actual stderr
# saved to <path>" for each fixture; read each one, then copy it into
# hyp_hegel/ui/. There is no bless mode -- see the Background section.
cargo test --package hyp_hegel --test ui 2>&1 | grep 'saved to'
# ...copy each into place, then confirm the whole package is green.
# Use the bare form, not `--test ui`: it also runs the doctests generated from
# the `declare_lint!` doc comments, which `--test ui` silently skips.
cargo test -p hyp_hegel
```

Expected: PASS. `allow_unjustified.stderr` contains one `hegel_exemption_without_justification` error pointing at the `#[allow]` attribute; `allow_justified.stderr` is empty. Verify both by reading them.

- [ ] **Step 7: Commit**

```bash
cd "$(git rev-parse --show-toplevel)"
git add lints/
git commit -m "feat: require a stated reason on hegel exemptions"
```

---

## Task 7: Flag crates that test but never use hegel

**Files:**
- Create: `lints/hyp_hegel/ui/no_hegel_tests.rs`
- Modify: `lints/hyp_hegel/src/hegel_tests.rs`, `lints/hyp_hegel/src/lib.rs`

- [ ] **Step 1: Write the failing UI fixture**

`lints/hyp_hegel/ui/no_hegel_tests.rs`:

```rust
#[allow(non_hegel_test, reason = "exercising the crate-level check")]
#[test]
fn only_example_based() {
    assert_eq!(1 + 1, 2);
}

fn main() {}
```

Every test here is exempted individually, so the per-test lint stays quiet — isolating the crate-level check.

- [ ] **Step 2: Run to verify it fails**

```bash
cd lints && cargo test --package hyp_hegel --test ui
```

Expected: FAIL — no diagnostic is produced, because the crate-level check does not exist yet.

- [ ] **Step 3: Declare the lint**

Add to `lints/hyp_hegel/src/hegel_tests.rs`:

```rust
declare_lint! {
    /// ### What it does
    ///
    /// Checks for crates that have a test harness but contain no hegel
    /// property tests at all.
    ///
    /// ### Why is this bad?
    ///
    /// A crate that has opted into testing but uses no property testing is
    /// missing the primary constraint this policy exists to enforce. A crate
    /// with no tests at all is a separate concern and is not reported here.
    ///
    /// ### Example
    ///
    /// ```rust,ignore
    /// #[test]
    /// fn one() {}
    /// #[test]
    /// fn two() {}
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust,ignore
    /// #[hegel::test]
    /// fn property(tc: hegel::TestCase) {}
    /// ```
    pub CRATE_WITHOUT_HEGEL_TESTS,
    Deny,
    "crate has tests but no hegel property tests"
}
```

Update: `impl_lint_pass!(HegelTests => [NON_HEGEL_TEST, HEGEL_EXEMPTION_WITHOUT_JUSTIFICATION, CRATE_WITHOUT_HEGEL_TESTS]);`

- [ ] **Step 4: Implement `check_crate_post`**

Add to the `impl LateLintPass for HegelTests` block:

```rust
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // A crate with no test harness at all is silent: "this crate has no
        // tests" is a different policy question and out of scope here.
        if self.test_fns.is_empty() || self.hegel_test_count > 0 {
            return;
        }

        let span = cx.tcx.def_span(rustc_hir::def_id::CRATE_DEF_ID);
        span_lint_and_help(
            cx,
            CRATE_WITHOUT_HEGEL_TESTS,
            span,
            "crate has tests but no hegel property tests",
            None,
            "add at least one `#[hegel::test]` property test to this crate",
        );
    }
```

- [ ] **Step 5: Register it**

In `lints/hyp_hegel/src/lib.rs`, add `hegel_tests::CRATE_WITHOUT_HEGEL_TESTS` to the `register_lints` slice.

- [ ] **Step 6: Run and bless**

```bash
cd lints
# Regenerate any .stderr that changed: the failing run prints "Actual stderr
# saved to <path>" for each fixture; read each one, then copy it into
# hyp_hegel/ui/. There is no bless mode -- see the Background section.
cargo test --package hyp_hegel --test ui 2>&1 | grep 'saved to'
# ...copy each into place, then confirm the whole package is green.
# Use the bare form, not `--test ui`: it also runs the doctests generated from
# the `declare_lint!` doc comments, which `--test ui` silently skips.
cargo test -p hyp_hegel
```

Expected: PASS. `no_hegel_tests.stderr` contains `crate_without_hegel_tests`. Critically, check that `hegel_test.stderr` and `builder_form.stderr` are still empty — those crates *do* have hegel tests and must not trip the crate-level check. If they now fire, `hegel_test_count` is not being incremented on the skip path (Task 6, Step 4).

- [ ] **Step 7: Run the full lint workspace checks**

```bash
cd lints && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

Expected: all clean.

- [ ] **Step 8: Commit**

```bash
cd "$(git rev-parse --show-toplevel)"
git add lints/
git commit -m "feat: flag crates with tests but no hegel property tests"
```

---

## Task 8: End-to-end consumer fixture

UI tests verify the pass logic and nothing about packaging. This task builds a real consumer workspace and proves `cargo dylint` actually loads and fires.

**Files:**
- Create: `fixtures/consumer/Cargo.toml` and four member crates, `scripts/e2e.sh`

- [ ] **Step 1: Create the consumer workspace**

`fixtures/consumer/Cargo.toml`:

```toml
[workspace]
members = ["good", "bad", "exempt_ok", "exempt_bad"]
resolver = "2"

[workspace.dependencies]
hegel = { package = "hegeltest", version = "0.14" }

[workspace.metadata.dylint]
libraries = [{ path = "../../lints/*" }]

[workspace.lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = ["cfg(dylint_lib, values(any()))"]
```

`path` rather than `git` is deliberate: at pull-request time the release tag does not exist yet, so this validates the working tree. Task 11 validates the `git`/`tag` path at release time.

- [ ] **Step 2: Create the four member crates**

Each needs `Cargo.toml` of this shape (substituting the name):

```toml
[package]
name = "good"
version = "0.1.0"
edition = "2021"
publish = false

[dev-dependencies]
hegel = { workspace = true }

[lints]
workspace = true
```

`fixtures/consumer/good/src/lib.rs`:

```rust
pub fn add(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

#[cfg(test)]
mod tests {
    use super::add;

    #[hegel::test]
    fn addition_commutes(tc: hegel::TestCase) {
        let a = tc.draw(hegel::generators::integers::<i64>());
        let b = tc.draw(hegel::generators::integers::<i64>());
        assert_eq!(add(a, b), add(b, a));
    }
}
```

`fixtures/consumer/bad/src/lib.rs`:

```rust
pub fn add(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn addition_works() {
        assert_eq!(add(2, 2), 4);
    }
}
```

`fixtures/consumer/exempt_ok/src/lib.rs` — note this uses the real `cfg_attr(dylint_lib = ...)` form that consumers must write, and includes one genuine hegel test so the crate-level lint stays quiet:

```rust
pub fn render(n: u8) -> String {
    format!("{n:03}")
}

#[cfg(test)]
mod tests {
    use super::render;

    #[cfg_attr(
        dylint_lib = "hyp_hegel",
        allow(
            non_hegel_test,
            reason = "asserts an exact zero-padded output format; no general property holds"
        )
    )]
    #[test]
    fn golden_format() {
        assert_eq!(render(7), "007");
    }

    #[hegel::test]
    fn always_three_chars(tc: hegel::TestCase) {
        let n = tc.draw(hegel::generators::integers::<u8>());
        assert_eq!(render(n).len(), 3);
    }
}
```

`fixtures/consumer/exempt_bad/src/lib.rs` — identical but with the `reason` removed:

```rust
pub fn render(n: u8) -> String {
    format!("{n:03}")
}

#[cfg(test)]
mod tests {
    use super::render;

    #[cfg_attr(dylint_lib = "hyp_hegel", allow(non_hegel_test))]
    #[test]
    fn golden_format() {
        assert_eq!(render(7), "007");
    }

    #[hegel::test]
    fn always_three_chars(tc: hegel::TestCase) {
        let n = tc.draw(hegel::generators::integers::<u8>());
        assert_eq!(render(n).len(), 3);
    }
}
```

- [ ] **Step 3: Write the e2e script**

`scripts/e2e.sh` — see the committed file for the current version. Three
things about it are load-bearing and were discovered the hard way:

**Capture the exit status on its own line.** The obvious form is wrong:

```bash
local out
out=$(cargo dylint ... 2>&1)
if [ $? -ne 0 ]; then    # tests `local`, not cargo dylint
```

`local` always succeeds, so `$?` is always 0 and every clean-expecting check
reports success unconditionally — including when the lint library fails to load
entirely. That is the exact silent-pass failure this script exists to prevent.

**Match on `--message-format=json`, not rendered text.** Grepping the human
output for a lint name produces false positives: the
`hegel_exemption_without_justification` diagnostic quotes `allow(non_hegel_test)`
in its own message, so a grep for `non_hegel_test` matches on a package where
that lint never fired. Match `"code":{"code":"<lint>"` instead, and re-run in
human format only when reporting a failure.

**Assert the full lint set per package, not one name.** `bad` legitimately fires
two lints; asserting only `non_hegel_test` would leave the crate-level lint
untested end to end, and an unexpected extra lint would go unnoticed.

```bash
chmod +x scripts/e2e.sh
```

- [ ] **Step 4: Run it to verify it passes**

```bash
./scripts/e2e.sh
```

Expected output, all four lines:

```
ok: good clean
ok: bad reported crate_without_hegel_tests non_hegel_test
ok: exempt_ok clean
ok: exempt_bad reported hegel_exemption_without_justification
```

`bad` reports two lints: it has tests and none are hegel, so the crate-level
lint fires alongside the per-test one. `exempt_bad` reports only the
justification lint, because its `always_three_chars` hegel test keeps the crate
count non-zero.

If `good` fails, the real crate's expansion chain is reporting a `[lib]` name not in `HEGEL_CRATE_NAMES`. The Task 2 spike observed `hegel_macros` for `#[hegel::test]`; check `docs/superpowers/notes/2026-09-15-api-spike.md` and reconcile rather than guessing.

- [ ] **Step 5: Verify the `--all-targets` trap is real**

```bash
cd fixtures/consumer && cargo dylint --all -- -p bad; echo "exit=$?"
```

Expected: `exit=0` — the lint sees nothing without `--all-targets`. This confirms why the composite action supplies it by default. Do not "fix" this.

- [ ] **Step 6: Commit**

```bash
cd "$(git rev-parse --show-toplevel)"
git add fixtures/ scripts/
git commit -m "test: add end-to-end consumer fixture"
```

---

## Task 9: CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the workflow**

`.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  DYLINT_VERSION: 6.0.4

jobs:
  lints:
    name: Lint library
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin
            ~/.cargo/registry
            ~/.cargo/git
            lints/target
          key: lints-${{ runner.os }}-${{ hashFiles('lints/Cargo.lock', 'lints/rust-toolchain.toml') }}

      - name: Install dylint-link
        run: cargo install --locked dylint-link@${{ env.DYLINT_VERSION }}

      - name: Format
        working-directory: lints
        run: cargo fmt --check

      - name: Clippy
        working-directory: lints
        run: cargo clippy --all-targets -- -D warnings

      - name: UI tests
        working-directory: lints
        run: cargo test

  e2e:
    name: Consumer smoke test
    runs-on: ubuntu-latest
    needs: lints
    steps:
      - uses: actions/checkout@v5

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin
            ~/.cargo/registry
            ~/.cargo/git
            ~/.dylint_drivers
          key: e2e-${{ runner.os }}-${{ hashFiles('lints/Cargo.lock', 'lints/rust-toolchain.toml') }}

      - name: Install dylint
        run: cargo install --locked cargo-dylint@${{ env.DYLINT_VERSION }} dylint-link@${{ env.DYLINT_VERSION }}

      - name: End-to-end assertions
        run: ./scripts/e2e.sh
```

The nightly toolchain is not installed explicitly — `rustup` reads `lints/rust-toolchain.toml` and fetches it, including the `rustc-dev` and `llvm-tools-preview` components, on first use in that directory.

- [ ] **Step 2: Verify locally before pushing**

```bash
cd lints && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd "$(git rev-parse --show-toplevel)" && ./scripts/e2e.sh
```

Expected: all pass. This is the same sequence CI runs.

- [ ] **Step 3: Commit and confirm CI is green**

```bash
git add .github/
git commit -m "ci: build and test the lint library"
git push
gh run watch
```

Expected: both jobs green. Do not proceed until they are.

---

## Task 10: Composite action

**Files:**
- Create: `action.yml`, `scripts/verify-pin.sh`

- [ ] **Step 1: Write the pin verification script**

`scripts/verify-pin.sh`:

```bash
#!/usr/bin/env bash
# Verify that the consuming workspace pins the same tag the action was called
# at. Drift between "which action version ran" and "which lint version it ran"
# is otherwise invisible and produces results that are hard to interpret.
set -euo pipefail

manifest="${1:?usage: verify-pin.sh <Cargo.toml> <expected-ref>}"
expected="${2:?usage: verify-pin.sh <Cargo.toml> <expected-ref>}"

if [ ! -f "$manifest" ]; then
  echo "verify-pin: no manifest at $manifest" >&2
  exit 1
fi

# Pull the tag out of the hyp-rust-policy-lints entry in
# workspace.metadata.dylint.libraries.
found=$(grep -A2 'hyp-rust-policy-lints' "$manifest" \
        | grep -o 'tag[[:space:]]*=[[:space:]]*"[^"]*"' \
        | head -n1 \
        | sed 's/.*"\(.*\)"/\1/' || true)

if [ -z "$found" ]; then
  echo "verify-pin: no hyp-rust-policy-lints tag found in $manifest" >&2
  echo "verify-pin: expected a libraries entry pinning tag = \"$expected\"" >&2
  exit 1
fi

if [ "$found" != "$expected" ]; then
  echo "verify-pin: version drift" >&2
  echo "  action called at: $expected" >&2
  echo "  Cargo.toml pins:  $found" >&2
  echo "Pin both to the same tag, or set verify-pin: false." >&2
  exit 1
fi

echo "verify-pin: ok ($found)"
```

```bash
chmod +x scripts/verify-pin.sh
```

- [ ] **Step 2: Write the composite action**

`action.yml` must be at the repository root — that is what makes `uses: hypotheosis/hyp-rust-policy-lints@v0.1.0` resolve.

```yaml
name: Hyp Rust Policy Lints
description: Run hypotheosis Rust policy lints via cargo-dylint
branding:
  icon: check-circle
  color: purple

inputs:
  working-directory:
    description: Workspace root to check
    required: false
    default: "."
  args:
    description: Arguments passed to cargo dylint
    required: false
    default: "--all -- --all-targets"
  cargo-dylint-version:
    description: cargo-dylint version to install
    required: false
    default: "6.0.4"
  verify-pin:
    description: Fail if the workspace pins a different tag than this action's ref
    required: false
    default: "true"

runs:
  using: composite
  steps:
    - name: Cache cargo and dylint
      uses: actions/cache@v4
      with:
        path: |
          ~/.cargo/bin
          ~/.cargo/registry
          ~/.cargo/git
          ~/.dylint_drivers
        key: hyp-policy-lints-${{ runner.os }}-${{ github.action_ref }}

    - name: Install cargo-dylint
      shell: bash
      run: |
        cargo install --locked \
          cargo-dylint@${{ inputs.cargo-dylint-version }} \
          dylint-link@${{ inputs.cargo-dylint-version }}

    - name: Verify pinned version
      if: inputs.verify-pin == 'true'
      shell: bash
      run: |
        "${{ github.action_path }}/scripts/verify-pin.sh" \
          "${{ inputs.working-directory }}/Cargo.toml" \
          "${{ github.action_ref }}"

    - name: Run policy lints
      shell: bash
      working-directory: ${{ inputs.working-directory }}
      run: cargo dylint ${{ inputs.args }}
```

Caching keyed on `github.action_ref` is the load-bearing part: without it, every consumer CI run recompiles a nightly `rustc_private` cdylib from scratch, costing minutes on every job. Keyed on the tag, it is a cold build once per lint release per repository.

- [ ] **Step 3: Test verify-pin locally, both directions**

```bash
cd "$(git rev-parse --show-toplevel)"
printf '[workspace.metadata.dylint]\nlibraries = [{ git = "https://github.com/hypotheosis/hyp-rust-policy-lints", tag = "v0.1.0", pattern = "lints/*" }]\n' > /tmp/pin-ok.toml
./scripts/verify-pin.sh /tmp/pin-ok.toml v0.1.0; echo "exit=$?"
./scripts/verify-pin.sh /tmp/pin-ok.toml v0.2.0; echo "exit=$?"
```

Expected: first prints `verify-pin: ok (v0.1.0)` and `exit=0`; second prints a drift error and `exit=1`.

- [ ] **Step 4: Commit**

```bash
git add action.yml scripts/verify-pin.sh
git commit -m "feat: add composite action for consumers"
```

---

## Task 11: Release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write the workflow**

`.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags: ["v*"]

env:
  CARGO_TERM_COLOR: always
  DYLINT_VERSION: 6.0.4

jobs:
  validate:
    name: Validate tag
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5

      - name: Install dylint
        run: cargo install --locked cargo-dylint@${{ env.DYLINT_VERSION }} dylint-link@${{ env.DYLINT_VERSION }}

      - name: Lint library checks
        working-directory: lints
        run: |
          cargo fmt --check
          cargo clippy --all-targets -- -D warnings
          cargo test

      - name: Path-based end-to-end
        run: ./scripts/e2e.sh

      # This is the only place the git + tag + pattern resolution consumers
      # actually depend on gets exercised. Pull-request CI structurally cannot
      # test it, because the tag does not exist until this moment.
      - name: Git-based end-to-end
        run: |
          set -euo pipefail
          TAG="${GITHUB_REF_NAME}"
          WORK=$(mktemp -d)
          mkdir -p "$WORK/bad/src"

          cat > "$WORK/Cargo.toml" <<EOF
          [workspace]
          members = ["bad"]
          resolver = "2"

          [workspace.metadata.dylint]
          libraries = [
            { git = "https://github.com/${GITHUB_REPOSITORY}", tag = "${TAG}", pattern = "lints/*" },
          ]

          [workspace.lints.rust.unexpected_cfgs]
          level = "warn"
          check-cfg = ["cfg(dylint_lib, values(any()))"]
          EOF

          cat > "$WORK/bad/Cargo.toml" <<EOF
          [package]
          name = "bad"
          version = "0.1.0"
          edition = "2021"
          publish = false

          [lints]
          workspace = true
          EOF

          cat > "$WORK/bad/src/lib.rs" <<EOF
          #[cfg(test)]
          mod tests {
              #[test]
              fn plain() {
                  assert_eq!(1 + 1, 2);
              }
          }
          EOF

          cd "$WORK"
          out=$(cargo dylint --all -- --all-targets 2>&1) && status=0 || status=$?
          if [ "$status" -eq 0 ]; then
            echo "FAIL: git-pinned library did not fire on a plain test"
            echo "$out"
            exit 1
          fi
          grep -q non_hegel_test <<<"$out" || {
            echo "FAIL: failed without mentioning non_hegel_test"
            echo "$out"
            exit 1
          }
          echo "ok: git-pinned library resolves and fires"

      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          generate_release_notes: true
          body: |
            ## Lints in this release

            - `non_hegel_test` — test does not use the hegel property-testing framework
            - `hegel_exemption_without_justification` — `allow(non_hegel_test)` without a stated reason
            - `crate_without_hegel_tests` — crate has tests but no hegel property tests

            ### Usage

            ```toml
            [workspace.metadata.dylint]
            libraries = [
              { git = "https://github.com/${{ github.repository }}", tag = "${{ github.ref_name }}", pattern = "lints/*" },
            ]

            [workspace.lints.rust.unexpected_cfgs]
            level = "warn"
            check-cfg = ["cfg(dylint_lib, values(any()))"]
            ```

            ```yaml
            - uses: actions/checkout@v5
            - uses: ${{ github.repository }}@${{ github.ref_name }}
            ```
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: validate and publish release tags"
git push
```

- [ ] **Step 3: Cut the first release**

```bash
git tag v0.1.0
git push origin v0.1.0
gh run watch
```

Expected: the `validate` job goes green and a GitHub Release appears. If the git-based step fails, **delete the tag before anyone pins it**:

```bash
git push --delete origin v0.1.0 && git tag -d v0.1.0
```

Then fix and retag.

---

## Task 12: Consumer documentation

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Write the README**

Replace `README.md` with:

````markdown
# hyp-rust-policy-lints

Custom [`cargo-dylint`](https://github.com/trailofbits/dylint) lints encoding
hypotheosis Rust engineering policy.

## Lints

| Lint | Level | Fires on |
|---|---|---|
| `non_hegel_test` | Deny | A test that does not use the hegel property-testing framework |
| `hegel_exemption_without_justification` | Deny | An `allow(non_hegel_test)` carrying no `reason = "..."` |
| `crate_without_hegel_tests` | Deny | A crate that has tests but no hegel property tests |

## Using it

Add to your workspace `Cargo.toml`:

```toml
[workspace.metadata.dylint]
libraries = [
  { git = "https://github.com/hypotheosis/hyp-rust-policy-lints", tag = "v0.1.0", pattern = "lints/*" },
]

[workspace.lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = ["cfg(dylint_lib, values(any()))"]
```

The `check-cfg` entry is required. dylint passes `--cfg=dylint_lib="hyp_hegel"`
when it runs, and without this declaration every exemption attribute produces an
`unexpected_cfgs` warning during ordinary `cargo build`.

In CI:

```yaml
- uses: actions/checkout@v5
- uses: hypotheosis/hyp-rust-policy-lints@v0.1.0
```

Locally:

```bash
cargo install --locked cargo-dylint dylint-link
cargo dylint --all -- --all-targets
```

`--all-targets` is required. Without it the lint loads, finds no tests, and
reports nothing — which looks exactly like passing.

## Exempting a test

Exemptions must state a reason. This is enforced: an `allow` without one is
itself an error.

```rust
#[cfg_attr(
    dylint_lib = "hyp_hegel",
    allow(
        non_hegel_test,
        reason = "asserts an exact serialized byte layout; no general property holds"
    )
)]
#[test]
fn golden_wire_format() {
    // ...
}
```

Good reasons to exempt a test, from hegel's own guidance: it checks exact output
format, it checks a specific error message, complex setup dominates, or no
meaningful property is apparent. Requires Rust 1.81+ for `reason` in lint
attributes.

## Versioning

Exact tags only — there is no moving `v0` tag. Pin the same tag in both
`libraries` and `uses:`; the action verifies they match and fails on drift.

Bumping the nightly in `lints/rust-toolchain.toml` is a release event, not a
routine dependency update: consumers compile the lint from source, so the
toolchain is part of the public contract. It must move in lockstep with the
`clippy_utils` rev in `lints/Cargo.toml`.

## Developing

```bash
cargo install --locked cargo-dylint dylint-link

cd lints && cargo test          # UI tests
./scripts/e2e.sh                # end-to-end consumer check
```

To regenerate expected UI output: run the test, and it prints `Actual stderr
saved to <path>` for each fixture that differs. Read that file, then copy it
over the corresponding `hyp_hegel/ui/*.stderr`. There is no bless mode —
`dylint_testing` does not plumb one through to `compiletest_rs`. Always read
the output before accepting it.

Adding a lint: create a new directory under `lints/`. The workspace
`members = ["*"]` glob and the consumer-side `pattern = "lints/*"` both pick it
up with no other change.
````

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: document consumer setup and exemption policy"
git push
```

---

## Verification checklist

Before considering this done, confirm each of these by running it:

- [ ] `cd lints && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` passes
- [ ] `./scripts/e2e.sh` prints all four `ok:` lines
- [ ] `cd fixtures/consumer && cargo dylint --all -- -p bad` exits 0 (confirming the `--all-targets` requirement is real and documented)
- [ ] `gh run list --workflow=ci.yml --limit 1` shows success
- [ ] The `v0.1.0` release exists and its `validate` job passed
- [ ] A scratch workspace elsewhere on disk, pinned to `tag = "v0.1.0"` with the two-line action snippet, fails on a plain `#[test]` and passes on a `#[hegel::test]`
