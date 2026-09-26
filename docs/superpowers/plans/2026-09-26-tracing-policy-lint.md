# hyp_tracing Instrument Lint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a second dylint library, `lints/hyp_tracing`, that denies any non-test function lacking `#[tracing::instrument]`. It must accept justified `allow`s and support a justified per-workspace off switch in `dylint.toml`.

**Architecture:** One `LateLintPass` implements `check_crate`, which reads the config and skips `--test` builds, and `check_fn`, which reports uninstrumented functions. A function counts as instrumented when any expression or `let` in its body has a span whose expansion chain reaches a macro from the `tracing_attributes` crate. The level is resolved at the function's own `HirId`, and the unjustified-`allow` handling copies `hyp_hegel`'s. Both lint crates share one workspace version, so one tag means one thing.

**Tech Stack:** Rust nightly-2026-07-09 with `rustc_private`, dylint 6.0.4 (`dylint_linting` config API, `dylint_testing`), `clippy_utils` at rev `09382ed3c34e091d7705f96964e303b59978533c`, `serde`, and `tracing` 0.1 as a dev-dependency and in the e2e fixture.

**Spec:** `docs/superpowers/specs/2026-09-26-tracing-policy-lint-design.md`

---

## Background you need before starting

Read the "Background" section of `docs/superpowers/plans/2026-09-15-hegel-policy-lints.md` first. It explains dylint, why the repo has two workspaces, `dylint-link`, UI tests versus e2e tests, and why every `declare_lint!` example must be fenced ` ```rust,ignore `. Everything there applies here. The additions:

- **There is no bless mode.** When a UI test mismatches, it prints `Actual stderr saved to <path>`. Read that file and compare it to the expected text given in this plan. Copy it into place only when the *behaviour* it shows matches: the right lint, the right line, the right level. Column and caret widths may differ from this plan by a character; the line and the lint must not.
- **No `.stderr` means "assert silence".** Never commit an empty `.stderr`.
- **UI fixtures here are libraries.** The harness passes `--crate-type=lib`, so fixtures have no `fn main` (a `main` would itself be an uninstrumented function). Make fixture functions `pub`, or `dead_code` warnings will land in the stderr.
- **`dylint.toml` in UI tests.** `dylint_testing::ui::Test::dylint_toml(..)` sets `DYLINT_TOML` for one run, and runs are serialised by a mutex inside `dylint_testing`. So each config scenario gets its own fixture directory and its own `#[test]`.
- **Commands.** Run everything in `lints/` with bare `cargo test`, never `--test ui`, because the doctests matter. `just check` from the repo root runs everything CI runs.

## File map

| Path | Responsibility |
|---|---|
| `lints/Cargo.toml` | shared `[workspace.package] version`, `serde`/`tracing` workspace deps, `shared-version = true` |
| `lints/hyp_hegel/Cargo.toml` | `version.workspace = true` |
| `lints/hyp_tracing/Cargo.toml` | the new crate |
| `lints/hyp_tracing/src/lib.rs` | `register_lints`: `init_config`, the two lints, the pass |
| `lints/hyp_tracing/src/config.rs` | `[hyp_tracing]` schema, `Mode`, `load()` |
| `lints/hyp_tracing/src/instrument_detect.rs` | `body_is_instrumented()` |
| `lints/hyp_tracing/src/tracing_fns.rs` | lint declarations and the `LateLintPass` |
| `lints/hyp_tracing/tests/ui.rs` | one `#[test]` per fixture directory |
| `lints/hyp_tracing/ui*/` | fixtures (`ui`, `ui_test_build`, `ui_disabled`, `ui_disabled_no_reason`, `ui_disabled_blank_reason`, `ui_bad_config`) |
| `lints/hyp_tracing/ui/README.md` | fixture conventions |
| `fixtures/tracing_consumer/` | new e2e workspace |
| `fixtures/consumer/dylint.toml`, `fixtures/action_check/dylint.toml` | justified off switch |
| `scripts/e2e.sh`, `scripts/release-check.sh`, `scripts/release-notes.py` | new lints and workspace |
| `.github/workflows/ci.yml`, `.github/workflows/release.yml` | lint list and cache keys |
| `README.md` | the second policy |

---

### Task 1: Share one version across the lint workspace

**Files:**
- Modify: `lints/Cargo.toml`
- Modify: `lints/hyp_hegel/Cargo.toml:3`

- [ ] **Step 1: Move the version to the workspace**

In `lints/Cargo.toml`, add this after the `[workspace]` table and before `[workspace.dependencies]`:

```toml
# One version for every lint library. A tag `vX.Y.Z` selects all of lints/*
# at once -- consumers pin `pattern = "lints/*"` -- so the libraries cannot
# version independently: `v0.2.0` has to mean one thing.
[workspace.package]
version = "0.1.1"
```

In the same file, extend `[workspace.metadata.release]`:

```toml
[workspace.metadata.release]
tag-name = "v{{version}}"
# Every releasable crate moves together and one tag is cut. See
# [workspace.package] above for why.
shared-version = true
```

In `lints/hyp_hegel/Cargo.toml`, replace `version = "0.1.1"` with:

```toml
version.workspace = true
```

- [ ] **Step 2: Verify nothing moved**

Run: `cd lints && cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; [print(p["name"],p["version"]) for p in json.load(sys.stdin)["packages"]]'`

Expected:
```text
hyp_hegel 0.1.1
hegeltest 0.14.0
hegeltest-macros 0.14.0
```

Run: `cd lints && cargo test`
Expected: PASS. The UI tests and doctests are unchanged.

- [ ] **Step 3: Commit**

```bash
git add lints/Cargo.toml lints/hyp_hegel/Cargo.toml lints/Cargo.lock
git commit -m "build: share one version across the lint workspace"
```

(The release dry run happens in Task 12, once there are two crates to release together.)

---

### Task 2: Scaffold `hyp_tracing` with a do-nothing pass and a UI harness

**Files:**
- Modify: `lints/Cargo.toml` (`[workspace.dependencies]`)
- Create: `lints/hyp_tracing/Cargo.toml`
- Create: `lints/hyp_tracing/src/lib.rs`
- Create: `lints/hyp_tracing/src/tracing_fns.rs`
- Create: `lints/hyp_tracing/tests/ui.rs`
- Create: `lints/hyp_tracing/ui/instrumented_sync.rs`

- [ ] **Step 1: Add workspace dependencies**

In `lints/Cargo.toml` `[workspace.dependencies]`, add:

```toml
serde = { version = "1", features = ["derive"] }
# The real crate, not a stub: hyp_tracing's detection depends on exactly how
# tracing-attributes assigns spans, and a stub would only test the stub.
tracing = "0.1"
```

- [ ] **Step 2: Create `lints/hyp_tracing/Cargo.toml`**

```toml
[package]
name = "hyp_tracing"
version.workspace = true
description = "hypotheosis policy lints: require #[tracing::instrument] on every function"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
clippy_utils = { workspace = true }
dylint_linting = { workspace = true }
serde = { workspace = true }

[dev-dependencies]
dylint_testing = { workspace = true }
tracing = { workspace = true }

[features]
rlib = ["dylint_linting/constituent"]

[lints]
workspace = true

[package.metadata.rust-analyzer]
rustc_private = true
```

- [ ] **Step 3: Create `lints/hyp_tracing/src/lib.rs`**

```rust
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
```

Only the crates this task uses are declared: `#![warn(unused_extern_crates)]` flags the rest. Task 3 adds `rustc_hir`, `rustc_middle` and `rustc_span`.

- [ ] **Step 4: Create `lints/hyp_tracing/src/tracing_fns.rs`**

It holds both lint declarations (final text) and an empty pass:

```rust
use rustc_lint::LateLintPass;
use rustc_session::{declare_lint, impl_lint_pass};

declare_lint! {
    /// ### What it does
    ///
    /// Checks for functions that are not instrumented with
    /// `#[tracing::instrument]`. Every free function, inherent method,
    /// trait-impl method and trait default method with a body is in scope.
    /// `const fn`s, closures and anything compiled under `--test` are not.
    ///
    /// ### Why is this bad?
    ///
    /// A function without a span is invisible in a trace: its time is
    /// attributed to whichever caller happens to be instrumented, and its
    /// arguments are never recorded. Requiring the attribute everywhere makes
    /// trace coverage the default rather than something someone remembered.
    ///
    /// ### Example
    ///
    /// ```rust,ignore
    /// pub fn load(path: &Path) -> io::Result<Config> {
    ///     // ...
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust,ignore
    /// #[tracing::instrument]
    /// pub fn load(path: &Path) -> io::Result<Config> {
    ///     // ...
    /// }
    /// ```
    pub UNINSTRUMENTED_FN,
    Deny,
    "function is not instrumented with `#[tracing::instrument]`"
    // Deliberately *not* `report_in_external_macro`. A function generated by
    // another crate's macro has no source line where a consumer could write
    // `#[instrument]`, and builtin derives (`Debug`, `Clone`, ...) would fire on
    // every derived method. rustc cancels those diagnostics for us.
}

declare_lint! {
    /// ### What it does
    ///
    /// Checks for `allow(uninstrumented_fn)` that carries no
    /// `reason = "..."`, and for a `dylint.toml` that switches `hyp_tracing`
    /// off without one.
    ///
    /// ### Why is this bad?
    ///
    /// Leaving a function out of the trace should be a deliberate, explained
    /// decision. A written reason makes whoever adds the exemption say why,
    /// and leaves that argument in the code for the next reader.
    ///
    /// ### Example
    ///
    /// ```rust,ignore
    /// #[allow(uninstrumented_fn)]
    /// fn decode_byte(b: u8) -> u8 { b ^ 0x5a }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust,ignore
    /// #[allow(uninstrumented_fn, reason = "called per byte in the decoder hot loop")]
    /// fn decode_byte(b: u8) -> u8 { b ^ 0x5a }
    /// ```
    pub INSTRUMENT_EXEMPTION_WITHOUT_JUSTIFICATION,
    Deny,
    "`allow(uninstrumented_fn)` or a `dylint.toml` opt-out without a stated reason",
    // The span may be an `allow` attribute that a consumer's macro wrote.
    // Without this, rustc would cancel the diagnostic before it is rendered,
    // and the justification requirement would be unenforceable there.
    report_in_external_macro
}

#[derive(Default)]
pub struct TracingFns;

impl_lint_pass!(TracingFns => [UNINSTRUMENTED_FN, INSTRUMENT_EXEMPTION_WITHOUT_JUSTIFICATION]);

impl<'tcx> LateLintPass<'tcx> for TracingFns {}
```

- [ ] **Step 5: Create `lints/hyp_tracing/tests/ui.rs`**

```rust
use std::env::current_exe;
use std::fs::read_dir;
use std::path::{Path, PathBuf};

/// Default config, ordinary (non-test) compilation.
#[test]
fn ui() {
    run("ui", Build::Library, None);
}

/// How a fixture directory is compiled.
#[derive(Clone, Copy)]
enum Build {
    /// `--crate-type=lib`: the fixtures need no `fn main`, which would itself
    /// be an uninstrumented function and put noise in every `.stderr`.
    Library,
}

fn run(src_base: &str, build: Build, dylint_toml: Option<&str>) {
    let deps = deps_dir();
    let tracing = rlib(&deps, "tracing");

    let mut flags = vec![
        // `#[tracing::instrument]` in attribute position needs the 2018-or-later
        // extern prelude.
        "--edition=2024".to_owned(),
        // `dylint_testing` only recovers a fixture's `--extern`/`-L` flags for
        // *example* targets, so fixtures under `src_base` are linked by hand.
        // `-L dependency=` is what lets rustc find `tracing_attributes` and
        // `tracing_core`, which `tracing` depends on.
        "-L".to_owned(),
        format!("dependency={}", deps.display()),
        "--extern".to_owned(),
        format!("tracing={}", tracing.display()),
    ];
    match build {
        Build::Library => flags.push("--crate-type=lib".to_owned()),
    }

    let mut test = dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), src_base);
    test.rustc_flags(flags);
    if let Some(dylint_toml) = dylint_toml {
        test.dylint_toml(dylint_toml);
    }
    test.run();
}

/// The `target/debug/deps` directory holding this test binary and the
/// dev-dependencies built alongside it.
fn deps_dir() -> PathBuf {
    current_exe()
        .expect("could not determine test executable path")
        .parent()
        .expect("test executable has no parent directory")
        .to_path_buf()
}

/// The most recently built `lib<name>-<hash>.rlib` in `dir`.
///
/// Cargo leaves stale artifacts behind, so several hashes can coexist; the
/// newest is the one the current `Cargo.lock` produced. A copy of
/// `hyp_hegel/tests/ui.rs`'s helper: each lint crate is its own test binary
/// and there is no shared crate to put it in.
fn rlib(dir: &Path, name: &str) -> PathBuf {
    let prefix = format!("lib{name}-");
    let mut candidates = read_dir(dir)
        .unwrap_or_else(|e| panic!("could not read `{}`: {e}", dir.display()))
        .filter_map(Result::ok)
        .filter(|entry| {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            file_name.starts_with(&prefix) && file_name.ends_with(".rlib")
        })
        .map(|entry| {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .expect("could not stat candidate rlib");
            (modified, entry.path())
        })
        .collect::<Vec<_>>();

    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| {
            panic!(
                "no `{prefix}*.rlib` in `{}`; is the `tracing` dev-dependency built?",
                dir.display()
            )
        })
        .1
}
```

The prefix `libtracing-` does not match `libtracing_core-` or `libtracing_attributes-`, because those continue with `_`, not `-`.

- [ ] **Step 6: Create the first fixture, `lints/hyp_tracing/ui/instrumented_sync.rs`**

```rust
//! `#[tracing::instrument]` on a synchronous free function is accepted.

#[tracing::instrument]
pub fn instrumented(n: u32) -> u32 {
    n + 1
}
```

No `.stderr`: it must be silent.

- [ ] **Step 7: Build and run**

Run: `cd lints && cargo test -p hyp_tracing`
Expected: PASS. The `ui` test compiles `instrumented_sync.rs` and gets no diagnostics.

If compiletest rejects `--crate-type=lib`, which would show up as a "conflicting crate type" or E0601 "`main` function not found" error, stop and report it. The fallback is to drop the flag and end every fixture with `#[tracing::instrument] fn main() {}`. That touches every later task, so decide it now.

- [ ] **Step 8: Commit**

```bash
git add lints/Cargo.toml lints/Cargo.lock lints/hyp_tracing
git commit -m "feat(hyp_tracing): scaffold the library and its UI harness"
```

---

### Task 3: Detect `#[instrument]` and report uninstrumented functions

**Files:**
- Create: `lints/hyp_tracing/src/instrument_detect.rs`
- Modify: `lints/hyp_tracing/src/lib.rs`
- Modify: `lints/hyp_tracing/src/tracing_fns.rs`
- Create: fixtures under `lints/hyp_tracing/ui/` (listed below)

- [ ] **Step 1: Write the failing fixtures**

`ui/plain_fn.rs`:
```rust
pub fn plain() {}
```

`ui/plain_fn.stderr`:
```text
error: function is not instrumented with `#[tracing::instrument]`
  --> $DIR/plain_fn.rs:1:1
   |
LL | pub fn plain() {}
   | ^^^^^^^^^^^^^^
   |
   = help: add `#[tracing::instrument]`, or exempt it with `allow(uninstrumented_fn, reason = "...")`
   = note: `#[deny(uninstrumented_fn)]` on by default

error: aborting due to 1 previous error

```

`ui/plain_method.rs`:
```rust
pub struct Counter(u32);

impl Counter {
    pub fn get(&self) -> u32 {
        self.0
    }
}
```

`ui/plain_method.stderr`:
```text
error: function is not instrumented with `#[tracing::instrument]`
  --> $DIR/plain_method.rs:4:5
   |
LL |     pub fn get(&self) -> u32 {
   |     ^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: add `#[tracing::instrument]`, or exempt it with `allow(uninstrumented_fn, reason = "...")`
   = note: `#[deny(uninstrumented_fn)]` on by default

error: aborting due to 1 previous error

```

`ui/nested_fn.rs`. It pins down that an instrumented *outer* function does not exempt an item nested inside it. The nested function's tokens come from the user, not the macro.
```rust
#[tracing::instrument]
pub fn outer() -> u32 {
    fn inner() -> u32 {
        1
    }
    inner()
}
```

`ui/nested_fn.stderr`:
```text
error: function is not instrumented with `#[tracing::instrument]`
  --> $DIR/nested_fn.rs:3:5
   |
LL |     fn inner() -> u32 {
   |     ^^^^^^^^^^^^^^^^^
   |
   = help: add `#[tracing::instrument]`, or exempt it with `allow(uninstrumented_fn, reason = "...")`
   = note: `#[deny(uninstrumented_fn)]` on by default

error: aborting due to 1 previous error

```

`ui/local_macro_fn.rs`. A crate's own `macro_rules!` is not an external macro, so a function it emits is still reported.
```rust
macro_rules! make_fn {
    ($name:ident) => {
        pub fn $name() {}
    };
}

make_fn!(generated);
```

`ui/local_macro_fn.stderr`. This is the expected shape. Confirm the primary span is line 3 inside the macro and that there is an "in this macro invocation" label on line 7:
```text
error: function is not instrumented with `#[tracing::instrument]`
  --> $DIR/local_macro_fn.rs:3:9
   |
LL |         pub fn $name() {}
   |         ^^^^^^^^^^^^^^
...
LL | make_fn!(generated);
   | ------------------- in this macro invocation
   |
   = help: add `#[tracing::instrument]`, or exempt it with `allow(uninstrumented_fn, reason = "...")`
   = note: `#[deny(uninstrumented_fn)]` on by default
   = note: this error originates in the macro `make_fn` (in Nightly builds, run with -Z macro-backtrace for more info)

error: aborting due to 1 previous error

```

Silent fixtures (no `.stderr`):

`ui/instrumented_async.rs`:
```rust
//! `#[instrument]` on an `async fn`. The body is lowered to a coroutine, and
//! the detection must look inside it.

#[tracing::instrument]
pub async fn instrumented(n: u32) -> u32 {
    n + 1
}
```

`ui/instrumented_method.rs`:
```rust
pub struct Counter(u32);

impl Counter {
    #[tracing::instrument(skip(self))]
    pub fn get(&self) -> u32 {
        self.0
    }
}
```

`ui/instrumented_trait_default.rs`:
```rust
pub trait Greet {
    #[tracing::instrument(skip(self))]
    fn greet(&self) -> u32 {
        1
    }
}
```

`ui/instrumented_trait_impl.rs`:
```rust
pub trait Greet {
    fn greet(&self) -> u32;
}

pub struct English;

impl Greet for English {
    #[tracing::instrument(skip(self))]
    fn greet(&self) -> u32 {
        1
    }
}
```

`ui/const_fn.rs`:
```rust
//! `#[instrument]` cannot be applied to a `const fn`, so the lint must not
//! demand it.

pub const fn answer() -> u32 {
    42
}
```

`ui/closure_only.rs`:
```rust
//! A closure is not a function a consumer can annotate. This one sits in a
//! static, outside any function, so nothing else here could be reported.

pub static INCREMENT: fn(u32) -> u32 = |n| n + 1;
```

`ui/trait_decl_only.rs`:
```rust
//! A trait method with no body has nothing to instrument.

pub trait Shape {
    fn area(&self) -> f64;
}
```

`ui/derive_only.rs`:
```rust
//! Derived methods come from an external macro expansion. They have no source
//! line to annotate, and rustc cancels the diagnostic because
//! `uninstrumented_fn` does not opt into `report_in_external_macro`.

#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: i32,
}
```

- [ ] **Step 2: Run to see the failing fixtures fail**

Run: `cd lints && cargo test -p hyp_tracing`
Expected: FAIL. `plain_fn`, `plain_method`, `nested_fn` and `local_macro_fn` report a missing diagnostic because the pass does nothing yet. The silent fixtures pass.

- [ ] **Step 3: Create `lints/hyp_tracing/src/instrument_detect.rs`**

```rust
use rustc_hir::intravisit::{Visitor, walk_expr, walk_local};
use rustc_hir::{Body, Expr, LetStmt};
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
/// syntax context. Walking `def_span`'s expansion chain, as `hyp_hegel` does
/// for `#[hegel::test]`, finds nothing. The body is different: the
/// `let __tracing_attr_span; ...` prologue (sync) and the
/// `let __tracing_instrument_future = ...` wrapper (async) are emitted in the
/// attribute's own expansion context. So the body is searched for any
/// expression or `let` whose span passes through that expansion.
///
/// For an instrumented function the first thing visited already matches, so
/// the common case costs nothing. Only a function that will be *reported*
/// pays for a full walk of its body.
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
    // An `async fn`'s body is a coroutine closure, which is a separate body
    // that the default `nested_filter::None` would skip. `OnlyBodies` enters
    // it, but not nested *items*: a `fn` declared inside an instrumented
    // function is its own function and is checked on its own.
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

    fn visit_local(&mut self, local: &'tcx LetStmt<'tcx>) {
        if self.found {
            return;
        }
        if from_instrument(self.cx, local.span) {
            self.found = true;
            return;
        }
        walk_local(self, local);
    }
}
```

- [ ] **Step 4: Register the module and its rustc crates**

In `lints/hyp_tracing/src/lib.rs`, add `mod instrument_detect;` above `mod tracing_fns;`, and make the extern-crate block:

```rust
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
```

- [ ] **Step 5: Implement `check_fn` without level handling**

In `lints/hyp_tracing/src/tracing_fns.rs`, replace the imports and the empty `impl` with:

```rust
use crate::instrument_detect::body_is_instrumented;
use clippy_utils::diagnostics::span_lint_hir_and_then;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, FnDecl};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;
```

```rust
impl<'tcx> LateLintPass<'tcx> for TracingFns {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'_>,
        body: &'tcx Body<'_>,
        _span: Span,
        def_id: LocalDefId,
    ) {
        // Closures are not something a consumer can annotate, and a `const fn`
        // cannot take `#[instrument]` at all. An `async fn` arrives here twice:
        // once as the function (`ItemFn`/`Method`) and once as its coroutine
        // (`Closure`), and only the first is checked.
        if matches!(kind, FnKind::Closure) || cx.tcx.is_const_fn(def_id.to_def_id()) {
            return;
        }

        // Re-fetched through `tcx` for the `'tcx` lifetime on the `Body`'s
        // contents, which `body_is_instrumented`'s visitor needs.
        if body_is_instrumented(cx, cx.tcx.hir_body(body.id())) {
            return;
        }

        let hir_id = cx.tcx.local_def_id_to_hir_id(def_id);
        span_lint_hir_and_then(
            cx,
            UNINSTRUMENTED_FN,
            hir_id,
            cx.tcx.def_span(def_id),
            "function is not instrumented with `#[tracing::instrument]`",
            |diag| {
                diag.help(
                    "add `#[tracing::instrument]`, or exempt it with `allow(uninstrumented_fn, reason = \"...\")`",
                );
            },
        );
    }
}
```

If the compiler rejects the `check_fn` signature, copy the exact parameter types from the `LateLintPass::check_fn` error message. The body stays the same.

- [ ] **Step 6: Run the tests**

Run: `cd lints && cargo test -p hyp_tracing`
Expected: PASS. All the fixtures above match.

If `instrumented_async`, `instrumented_trait_default` or `instrumented_trait_impl` now *fires*, the detection is wrong for that shape. Inspect it with `cargo +nightly-2026-07-09 rustc -- -Zunpretty=expanded,hygiene` on a scratch crate, the same way the spec's §4 spike did, and adjust `body_is_instrumented`. Do not change the fixture.

- [ ] **Step 7: Commit**

```bash
git add lints/hyp_tracing
git commit -m "feat(hyp_tracing): report functions without #[tracing::instrument]"
```

---

### Task 4: Honour levels and require a reason on `allow`

**Files:**
- Modify: `lints/hyp_tracing/src/tracing_fns.rs`
- Create: fixtures under `lints/hyp_tracing/ui/` (listed below)

- [ ] **Step 1: Write the fixtures**

Silent (no `.stderr`):

`ui/allow_justified.rs`:
```rust
#[allow(uninstrumented_fn, reason = "called once per byte in the decoder hot loop")]
pub fn plain() {}
```

`ui/allow_module_level.rs`:
```rust
#[allow(uninstrumented_fn, reason = "generated lookup tables; a span per lookup would be noise")]
pub mod tables {
    pub fn first() {}
    pub fn second() {}
}
```

`ui/allow_impl_level.rs`:
```rust
pub struct Point {
    x: i32,
}

#[allow(uninstrumented_fn, reason = "trivial accessors on a hot path")]
impl Point {
    pub fn x(&self) -> i32 {
        self.x
    }
}
```

`ui/allow_crate_level.rs`:
```rust
#![allow(uninstrumented_fn, reason = "migration in progress: instrumenting module by module")]

pub fn plain() {}
```

`ui/expect_level.rs`:
```rust
#[expect(uninstrumented_fn)]
pub fn plain() {}
```

Reporting:

`ui/allow_unjustified.rs`:
```rust
#[allow(uninstrumented_fn)]
pub fn plain() {}
```

`ui/allow_unjustified.stderr`:
```text
error: `allow(uninstrumented_fn)` without a stated reason
  --> $DIR/allow_unjustified.rs:1:9
   |
LL | #[allow(uninstrumented_fn)]
   |         ^^^^^^^^^^^^^^^^^
   |
   = help: add `reason = "..."` explaining why this function is not instrumented
   = note: `#[deny(instrument_exemption_without_justification)]` on by default

error: aborting due to 1 previous error

```

`ui/allow_empty_reason.rs`:
```rust
#[allow(uninstrumented_fn, reason = "   ")]
pub fn plain() {}
```

`ui/allow_empty_reason.stderr`:
```text
error: `allow(uninstrumented_fn)` without a stated reason
  --> $DIR/allow_empty_reason.rs:1:9
   |
LL | #[allow(uninstrumented_fn, reason = "   ")]
   |         ^^^^^^^^^^^^^^^^^
   |
   = help: add `reason = "..."` explaining why this function is not instrumented
   = note: `#[deny(instrument_exemption_without_justification)]` on by default

error: aborting due to 1 previous error

```

`ui/warn_level.rs`:
```rust
#[warn(uninstrumented_fn)]
pub fn plain() {}
```

`ui/warn_level.stderr`:
```text
warning: function is not instrumented with `#[tracing::instrument]`
  --> $DIR/warn_level.rs:2:1
   |
LL | pub fn plain() {}
   | ^^^^^^^^^^^^^^
   |
   = help: add `#[tracing::instrument]`, or exempt it with `allow(uninstrumented_fn, reason = "...")`
note: the lint level is defined here
  --> $DIR/warn_level.rs:1:8
   |
LL | #[warn(uninstrumented_fn)]
   |        ^^^^^^^^^^^^^^^^^

warning: 1 warning emitted

```

`ui/deny_level.rs`. It pins the level being resolved at the function, not the crate: the crate says `warn`, and the function's own `deny` has to win.
```rust
#![warn(uninstrumented_fn)]

#[deny(uninstrumented_fn)]
pub fn plain() {}
```

`ui/deny_level.stderr`:
```text
error: function is not instrumented with `#[tracing::instrument]`
  --> $DIR/deny_level.rs:4:1
   |
LL | pub fn plain() {}
   | ^^^^^^^^^^^^^^
   |
   = help: add `#[tracing::instrument]`, or exempt it with `allow(uninstrumented_fn, reason = "...")`
note: the lint level is defined here
  --> $DIR/deny_level.rs:3:8
   |
LL | #[deny(uninstrumented_fn)]
   |        ^^^^^^^^^^^^^^^^^

error: aborting due to 1 previous error

```

`ui/expect_unfulfilled.rs`:
```rust
#[expect(uninstrumented_fn)]
#[tracing::instrument]
pub fn instrumented() {}
```

`ui/expect_unfulfilled.stderr`:
```text
warning: this lint expectation is unfulfilled
  --> $DIR/expect_unfulfilled.rs:1:10
   |
LL | #[expect(uninstrumented_fn)]
   |          ^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unfulfilled_lint_expectations)]` on by default

warning: 1 warning emitted

```

- [ ] **Step 2: Run to see what fails**

Run: `cd lints && cargo test -p hyp_tracing`
Expected: FAIL on `allow_unjustified` and `allow_empty_reason`. An `allow` currently silences the lint outright, so the justification diagnostic never appears. The others may already pass, because `span_lint_hir_and_then` resolves the level at `hir_id`. They are still required: they are what stops a later change from breaking that.

- [ ] **Step 3: Implement the level decision table**

In `tracing_fns.rs`, add these imports:

```rust
use rustc_middle::lint::LintLevelSource;
use rustc_session::lint::Level;
```

Then replace the final `span_lint_hir_and_then(...)` call in `check_fn`, from `let hir_id = ...` to the end of the function, with:

```rust
        let hir_id = cx.tcx.local_def_id_to_hir_id(def_id);

        // An `allow` suppresses `uninstrumented_fn` by construction, so the
        // justified and unjustified cases look the same from the emission
        // path alone. The level has to be inspected directly. Same table as
        // `hyp_hegel`'s `non_hegel_test`; see `hegel_tests.rs` for the API
        // notes on `lint_level_spec_at_node`.
        let spec = cx.tcx.lint_level_spec_at_node(UNINSTRUMENTED_FN, hir_id);

        match (spec.level(), spec.src) {
            // Exempted in source with a stated reason: accepted. Only the
            // *presence* of a non-blank reason is checked, never its quality.
            (
                Level::Allow,
                LintLevelSource::Node {
                    reason: Some(reason),
                    ..
                },
            ) if !reason.as_str().trim().is_empty() => {}

            // Exempted in source with no reason, or a blank one: report the
            // attribute. `span` covers just the lint name inside it.
            (
                Level::Allow,
                LintLevelSource::Node {
                    span: attr_span, ..
                },
            ) => {
                span_lint_hir_and_then(
                    cx,
                    INSTRUMENT_EXEMPTION_WITHOUT_JUSTIFICATION,
                    hir_id,
                    attr_span,
                    "`allow(uninstrumented_fn)` without a stated reason",
                    |diag| {
                        diag.help(
                            "add `reason = \"...\"` explaining why this function is not instrumented",
                        );
                    },
                );
            }

            // Allowed from the command line (`-A uninstrumented_fn`): a
            // deliberate operator decision, not a source-level exemption.
            (Level::Allow, _) => {}

            // Everything else -- the `Deny` default, or an explicit `warn`,
            // `deny` or `expect` -- is emitted at the function's own `HirId`,
            // so rustc resolves the level against the function and its
            // parents, not against whichever node the pass is visiting.
            _ => {
                span_lint_hir_and_then(
                    cx,
                    UNINSTRUMENTED_FN,
                    hir_id,
                    cx.tcx.def_span(def_id),
                    "function is not instrumented with `#[tracing::instrument]`",
                    |diag| {
                        diag.help(
                            "add `#[tracing::instrument]`, or exempt it with `allow(uninstrumented_fn, reason = \"...\")`",
                        );
                    },
                );
            }
        }
```

- [ ] **Step 4: Run the tests**

Run: `cd lints && cargo test -p hyp_tracing`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lints/hyp_tracing
git commit -m "feat(hyp_tracing): require a reason on allow(uninstrumented_fn)"
```

---

### Task 5: Skip `--test` compilations

**Files:**
- Modify: `lints/hyp_tracing/src/tracing_fns.rs`
- Modify: `lints/hyp_tracing/tests/ui.rs`
- Create: `lints/hyp_tracing/ui_test_build/test_helpers.rs`

- [ ] **Step 1: Write the fixture and its harness entry**

`ui_test_build/test_helpers.rs` has no `.stderr`, so it must be silent:
```rust
//! Compiled with `--test`. Neither the test nor the helper it calls is
//! production code, and neither needs a span.

fn helper() -> u32 {
    1
}

#[test]
fn uses_helper() {
    assert_eq!(helper(), 1);
}
```

In `tests/ui.rs`, add a variant to `Build`:

```rust
    /// `--test`: how cargo compiles unit and integration tests. Not combined
    /// with `--crate-type`, which a test harness overrides.
    TestHarness,
```

Add its arm to the `match build` in `run`:

```rust
        Build::TestHarness => flags.push("--test".to_owned()),
```

Add the test:

```rust
/// Everything under `--test` is out of scope.
#[test]
fn ui_test_build() {
    run("ui_test_build", Build::TestHarness, None);
}
```

- [ ] **Step 2: Run to see it fail**

Run: `cd lints && cargo test -p hyp_tracing ui_test_build`
Expected: FAIL. `helper` and `uses_helper` are both reported with `uninstrumented_fn`.

- [ ] **Step 3: Implement the skip**

In `tracing_fns.rs`, turn `TracingFns` into a stateful pass:

```rust
#[derive(Default)]
pub struct TracingFns {
    /// Whether `check_fn` reports anything for the crate being compiled.
    /// Decided once, in `check_crate`.
    enforce: bool,
}
```

Add `check_crate` to the `impl`, above `check_fn`:

```rust
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        // A `--test` compilation is test code: `#[test]` functions,
        // `#[cfg(test)]` helpers, integration-test crates. None of it needs a
        // span. Production code is still checked, in the ordinary lib/bin
        // compilation that `--all-targets` also performs -- and in the one a
        // run *without* `--all-targets` performs, so unlike `hyp_hegel`, this
        // library does not depend on that flag.
        self.enforce = !cx.tcx.sess.opts.test;
    }
```

Add this at the top of `check_fn`:

```rust
        if !self.enforce {
            return;
        }
```

- [ ] **Step 4: Run the tests**

Run: `cd lints && cargo test -p hyp_tracing`
Expected: PASS for `ui` and `ui_test_build`.

- [ ] **Step 5: Commit**

```bash
git add lints/hyp_tracing
git commit -m "feat(hyp_tracing): leave --test compilations alone"
```

---

### Task 6: The `dylint.toml` switch

**Files:**
- Create: `lints/hyp_tracing/src/config.rs`
- Modify: `lints/hyp_tracing/src/lib.rs`
- Modify: `lints/hyp_tracing/src/tracing_fns.rs`
- Modify: `lints/hyp_tracing/tests/ui.rs`
- Create: `lints/hyp_tracing/ui_disabled/plain.rs`
- Create: `lints/hyp_tracing/ui_disabled_no_reason/plain.rs` + `.stderr`
- Create: `lints/hyp_tracing/ui_disabled_blank_reason/plain.rs` + `.stderr`
- Create: `lints/hyp_tracing/ui_bad_config/plain.rs` + `.stderr`

- [ ] **Step 1: Write the fixtures**

Every one of the four directories gets the same `plain.rs`:
```rust
pub fn plain() {}
```

`ui_disabled/` has no `.stderr` and must be silent.

`ui_disabled_no_reason/plain.stderr`:
```text
error: `hyp_tracing` is disabled in `dylint.toml` without a stated reason
  --> $DIR/plain.rs:1:1
   |
LL | pub fn plain() {}
   | ^
   |
   = help: add `reason = "..."` under `[hyp_tracing]` explaining why this workspace does not instrument its functions
   = note: `#[deny(instrument_exemption_without_justification)]` on by default

error: aborting due to 1 previous error

```

`ui_disabled_blank_reason/plain.stderr` has identical content.

`ui_bad_config/plain.stderr`. This is the expected shape. The exact wording of the toml error comes from serde. Confirm that it names the unknown field `enable`, and that no `uninstrumented_fn` error appears alongside it:
```text
error: could not read `[hyp_tracing]` from `dylint.toml`: unknown field `enable`, expected `enabled` or `reason`

error: aborting due to 1 previous error

```

In `tests/ui.rs`, add:

```rust
/// `enabled = false` with a reason: the whole workspace is exempt.
#[test]
fn ui_disabled() {
    run(
        "ui_disabled",
        Build::Library,
        Some("[hyp_tracing]\nenabled = false\nreason = \"CLI tool with no tracing subscriber\"\n"),
    );
}

/// `enabled = false` without a reason: one justification error per crate.
#[test]
fn ui_disabled_no_reason() {
    run(
        "ui_disabled_no_reason",
        Build::Library,
        Some("[hyp_tracing]\nenabled = false\n"),
    );
}

/// A whitespace-only reason is no reason.
#[test]
fn ui_disabled_blank_reason() {
    run(
        "ui_disabled_blank_reason",
        Build::Library,
        Some("[hyp_tracing]\nenabled = false\nreason = \"   \"\n"),
    );
}

/// A misspelt key must fail loudly, not silently leave the policy on.
#[test]
fn ui_bad_config() {
    run(
        "ui_bad_config",
        Build::Library,
        Some("[hyp_tracing]\nenable = false\n"),
    );
}
```

- [ ] **Step 2: Run to see them fail**

Run: `cd lints && cargo test -p hyp_tracing`
Expected: all four new tests FAIL, because each reports `uninstrumented_fn` on `plain`. `ui` and `ui_test_build` still pass.

- [ ] **Step 3: Create `lints/hyp_tracing/src/config.rs`**

```rust
use serde::Deserialize;

/// This library's table in the consuming workspace's `dylint.toml`.
pub const TABLE: &str = "hyp_tracing";

/// `[hyp_tracing]` in `dylint.toml`.
///
/// ```toml
/// [hyp_tracing]
/// enabled = false
/// reason = "CLI tool with no tracing subscriber"
/// ```
///
/// Unknown keys are rejected. `enable = false` is a typo that would otherwise
/// deserialize to the default and leave the policy silently on -- or, for a
/// misspelt `reason`, silently unjustified.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "enabled_by_default")]
    enabled: bool,
    #[serde(default)]
    reason: Option<String>,
}

fn enabled_by_default() -> bool {
    true
}

impl Default for Config {
    /// No `[hyp_tracing]` table: enforce.
    fn default() -> Self {
        Self {
            enabled: true,
            reason: None,
        }
    }
}

/// What the pass does for the crate being compiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Enforce,
    /// Switched off with a non-blank reason.
    Off,
    /// Switched off without a reason, or with a blank one. The pass reports
    /// the missing justification once and nothing else.
    OffWithoutReason,
}

impl Config {
    pub fn mode(&self) -> Mode {
        if self.enabled {
            return Mode::Enforce;
        }
        // As for `allow(...)`, only the presence of a non-blank reason is
        // checked, never its quality.
        match &self.reason {
            Some(reason) if !reason.trim().is_empty() => Mode::Off,
            _ => Mode::OffWithoutReason,
        }
    }
}

/// Read `[hyp_tracing]` from the workspace's `dylint.toml`.
///
/// `dylint_linting::config`, not `config_or_default`: the latter panics on a
/// bad value, which a consumer would see as a compiler crash rather than a
/// configuration error.
pub fn load() -> Result<Config, dylint_linting::ConfigError> {
    dylint_linting::config::<Config>(TABLE).map(Option::unwrap_or_default)
}
```

- [ ] **Step 4: Initialise dylint's config in `register_lints`**

In `lib.rs`, add `mod config;`. Rename `_sess` to `sess` and make the first statement:

```rust
    // Reads the consuming workspace's `dylint.toml` (or `DYLINT_TOML`), so
    // `config::load` has something to read. Idempotent.
    dylint_linting::init_config(sess);
```

- [ ] **Step 5: Use the config in `check_crate`**

In `tracing_fns.rs`, add these imports:

```rust
use crate::config::{self, Mode};
use rustc_hir::CRATE_HIR_ID;
use rustc_hir::def_id::CRATE_DEF_ID;
```

Replace `check_crate` with:

```rust
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        self.enforce = false;

        // A `--test` compilation is test code: `#[test]` functions,
        // `#[cfg(test)]` helpers, integration-test crates. None of it needs a
        // span. Production code is still checked, in the ordinary lib/bin
        // compilation that `--all-targets` also performs -- and in the one a
        // run *without* `--all-targets` performs, so unlike `hyp_hegel`, this
        // library does not depend on that flag.
        if cx.tcx.sess.opts.test {
            return;
        }

        let config = match config::load() {
            Ok(config) => config,
            Err(error) => {
                // Neither "enforce" nor "off" is a safe guess for a switch
                // nobody can read: the build fails and says why.
                cx.tcx.dcx().err(format!(
                    "could not read `[{}]` from `dylint.toml`: {error}",
                    config::TABLE
                ));
                return;
            }
        };

        match config.mode() {
            Mode::Enforce => self.enforce = true,
            Mode::Off => {}
            // Reported once, at the crate root (the span
            // `crate_without_hegel_tests` uses), and `uninstrumented_fn` stays
            // off. The configuration is what needs fixing, and an error per
            // function would bury it.
            Mode::OffWithoutReason => {
                let crate_span = cx.tcx.def_span(CRATE_DEF_ID);
                let span = cx.tcx.sess.source_map().start_point(crate_span);
                span_lint_hir_and_then(
                    cx,
                    INSTRUMENT_EXEMPTION_WITHOUT_JUSTIFICATION,
                    CRATE_HIR_ID,
                    span,
                    "`hyp_tracing` is disabled in `dylint.toml` without a stated reason",
                    |diag| {
                        diag.help(
                            "add `reason = \"...\"` under `[hyp_tracing]` explaining why this workspace does not instrument its functions",
                        );
                    },
                );
            }
        }
    }
```

If `dcx().err(..)`'s `ErrorGuaranteed` return value triggers `unused_must_use`, bind it as `let _guar = ...;`.

- [ ] **Step 6: Run the tests**

Run: `cd lints && cargo test -p hyp_tracing`
Expected: PASS for all six UI tests. Before accepting `ui_bad_config/plain.stderr`, read its saved actual output. It must name `enable` and contain nothing about `uninstrumented_fn`.

- [ ] **Step 7: Commit**

```bash
git add lints/hyp_tracing
git commit -m "feat(hyp_tracing): justified per-workspace off switch in dylint.toml"
```

---

### Task 7: Prove each fixture is load-bearing

`lints/hyp_hegel/ui/README.md` requires that each fixture be checked against the break it guards. Do each mutation below, run `cd lints && cargo test -p hyp_tracing`, confirm that the **named** fixture fails, then revert with `git checkout lints/hyp_tracing/src`. Do not commit any mutation.

- [ ] **Step 1:** In `body_is_instrumented`, return `true` unconditionally. Expect failures in `plain_fn`, `plain_method`, `nested_fn`, `local_macro_fn`.
- [ ] **Step 2:** Return `false` unconditionally. Expect failures in `instrumented_sync`, `instrumented_async`, `instrumented_method`, `instrumented_trait_default`, `instrumented_trait_impl`, and `expect_unfulfilled` (the expectation becomes fulfilled).
- [ ] **Step 3:** Change the `nested_filter` to `nested_filter::None` (also delete `maybe_tcx` if the compiler says it no longer matches the trait). Expect `instrumented_async` to fail. If it does *not* fail, the async prologue is visible without entering the coroutine. Record that in the `NestedFilter` comment rather than leaving an unproven claim there.
- [ ] **Step 4:** Delete `matches!(kind, FnKind::Closure) ||`. Expect `closure_only` to fail.
- [ ] **Step 5:** Delete `|| cx.tcx.is_const_fn(...)`. Expect `const_fn` to fail.
- [ ] **Step 6:** Delete the `sess.opts.test` early return. Expect `ui_test_build` to fail.
- [ ] **Step 7:** Emit `UNINSTRUMENTED_FN` at `CRATE_HIR_ID` instead of `hir_id` in the fall-through arm. Expect `deny_level` and `warn_level` to fail.
- [ ] **Step 8:** Drop `.trim()` in the `allow` arm. Expect `allow_empty_reason` to fail.
- [ ] **Step 9:** Drop `.trim()` in `Config::mode`. Expect `ui_disabled_blank_reason` to fail.
- [ ] **Step 10:** Map `Mode::OffWithoutReason` to `{}`. Expect `ui_disabled_no_reason` to fail.
- [ ] **Step 11:** Remove `#[serde(deny_unknown_fields)]`. Expect `ui_bad_config` to fail.

If any named fixture passes under its mutation, it is not guarding what it claims. Fix the fixture, re-run the mutation, and commit the fixture fix:

```bash
git add lints/hyp_tracing
git commit -m "test(hyp_tracing): tighten <fixture> so it guards <what>"
```

---

### Task 8: Fixture README and lint-workspace checks

**Files:**
- Create: `lints/hyp_tracing/ui/README.md`

- [ ] **Step 1: Write `lints/hyp_tracing/ui/README.md`**

```markdown
# UI fixtures

Each `.rs` file is compiled as a **standalone library crate** with the lint
library loaded, and its diagnostics are diffed against the `.stderr` beside it.
New files are auto-discovered.

The general conventions are in [`hyp_hegel`'s fixture README](../../hyp_hegel/ui/README.md):
no `.stderr` means "assert silence", one fixture makes one assertion, there is
no bless mode, and every fixture is checked to be load-bearing by breaking what
it guards. What differs here:

**Fixtures are libraries.** `tests/ui.rs` passes `--crate-type=lib`, so there is
no `fn main` -- which would otherwise be an uninstrumented function in every
fixture's output. Make fixture functions `pub`, or `dead_code` warnings will
appear in the `.stderr`.

**The real `tracing` crate, no stub.** The detection depends on how
`tracing-attributes` assigns spans, so it is tested against the real thing.
Unlike `hyp_hegel`'s stub, `#[tracing::instrument]` keeps the other attributes
on its function, so `#[expect(...)]` next to it works.

**One directory per compilation mode or `dylint.toml`.** `rustc_flags` and
`dylint_toml` apply to a whole run, so each scenario has its own directory and
its own `#[test]` in `tests/ui.rs`:

| Directory | Build | `dylint.toml` |
|---|---|---|
| `ui/` | library | none |
| `ui_test_build/` | `--test` | none |
| `ui_disabled/` | library | `enabled = false`, with a reason |
| `ui_disabled_no_reason/` | library | `enabled = false` |
| `ui_disabled_blank_reason/` | library | `enabled = false`, `reason = "   "` |
| `ui_bad_config/` | library | `enable = false` (a typo) |
```

- [ ] **Step 2: Run the lint workspace's full CI gate**

Run: `cd lints && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: all three succeed. `cargo test` includes both crates' UI tests and the `declare_lint!` doctests. If clippy flags anything in `hyp_tracing`, fix it in the source rather than allowing it.

- [ ] **Step 3: Commit**

```bash
git add lints/hyp_tracing
git commit -m "docs(hyp_tracing): fixture conventions"
```

---

### Task 9: End-to-end fixture workspace and off switches for the existing fixtures

**Files:**
- Create: `fixtures/tracing_consumer/Cargo.toml`, `Cargo.lock`
- Create: `fixtures/tracing_consumer/{good,bad,exempt_ok,exempt_bad}/Cargo.toml` and `src/lib.rs`
- Create: `fixtures/consumer/dylint.toml`
- Create: `fixtures/action_check/dylint.toml`
- Modify: `scripts/e2e.sh`

- [ ] **Step 1: Create `fixtures/tracing_consumer/Cargo.toml`**

```toml
# End-to-end fixture for hyp_tracing: real `cargo dylint` over real crates using
# the real `tracing`. The hyp_hegel counterpart is fixtures/consumer. They are
# separate workspaces because `dylint.toml` is per workspace, and
# fixtures/consumer switches hyp_tracing off.
#
# None of these packages has tests, so hyp_hegel's lints stay silent here: a
# crate with no test harness is out of its scope.

[workspace]
members = ["good", "bad", "exempt_ok", "exempt_bad"]
resolver = "2"

[workspace.dependencies]
tracing = "0.1"

# `path` rather than `git`/`tag`, for the reason given in fixtures/consumer.
[workspace.metadata.dylint]
libraries = [{ path = "../../lints/*" }]

[workspace.lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = ["cfg(dylint_lib, values(any()))"]
```

- [ ] **Step 2: Create the four packages**

Each `Cargo.toml` looks like this, with `name` set to the package and the `[dependencies]` table left out for `bad`:

```toml
[package]
name = "good"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
tracing = { workspace = true }

[lints]
workspace = true
```

`good/src/lib.rs`:
```rust
//! Every function instrumented: sync and async, free and associated, and a
//! trait default method. Nothing here should ever produce a diagnostic.

#[tracing::instrument]
pub fn add(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

#[tracing::instrument]
pub async fn add_later(a: i64, b: i64) -> i64 {
    add(a, b)
}

pub trait Describe {
    #[tracing::instrument(skip(self))]
    fn describe(&self) -> String {
        String::from("widget")
    }
}

pub struct Widget;

impl Widget {
    #[tracing::instrument(skip(self))]
    pub fn id(&self) -> u32 {
        7
    }
}

impl Describe for Widget {}
```

`bad/src/lib.rs`:
```rust
pub fn add(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}
```

`exempt_ok/src/lib.rs`:
```rust
#[cfg_attr(
    dylint_lib = "hyp_tracing",
    allow(
        uninstrumented_fn,
        reason = "called once per byte by the decoder; a span per call would dominate its cost"
    )
)]
pub fn scramble(b: u8) -> u8 {
    b ^ 0x5a
}
```

`exempt_bad/src/lib.rs`:
```rust
#[cfg_attr(dylint_lib = "hyp_tracing", allow(uninstrumented_fn))]
pub fn scramble(b: u8) -> u8 {
    b ^ 0x5a
}
```

`exempt_ok` and `exempt_bad` don't use `tracing`, so leave `[dependencies]` out of their manifests as well.

- [ ] **Step 3: Generate the lockfile**

Run: `cd fixtures/tracing_consumer && cargo generate-lockfile`
Expected: `Cargo.lock` is created and resolves `tracing 0.1.x`.

- [ ] **Step 4: Switch hyp_tracing off in the existing fixtures**

`fixtures/consumer/dylint.toml`:
```toml
# This workspace exercises hyp_hegel, and its packages' functions are
# deliberately uninstrumented. Switching hyp_tracing off here keeps each
# package's expected lint set about hyp_hegel alone -- and is also the
# end-to-end check that the switch works through real `cargo dylint`: if it
# stopped working, `good` would start failing with `uninstrumented_fn`.
# hyp_tracing itself is exercised by fixtures/tracing_consumer.
[hyp_tracing]
enabled = false
reason = "hyp_hegel fixture workspace; hyp_tracing is exercised by fixtures/tracing_consumer"
```

`fixtures/action_check/dylint.toml`:
```toml
# The action job needs a workspace that passes. This one is about hyp_hegel,
# and its function is uninstrumented.
[hyp_tracing]
enabled = false
reason = "action smoke-test workspace; hyp_tracing is exercised by fixtures/tracing_consumer"
```

- [ ] **Step 5: Extend `scripts/e2e.sh`**

1. Replace the line `cd "$(dirname "$0")/../fixtures/consumer" || exit 1` with:

   ```bash
   root="$(cd "$(dirname "$0")/.." && pwd)" || exit 1
   ```

2. Extend `REQUIRED_LINTS`:

   ```bash
   REQUIRED_LINTS=(
     non_hegel_test
     hegel_exemption_without_justification
     crate_without_hegel_tests
     uninstrumented_fn
     instrument_exemption_without_justification
   )
   ```

3. In `require_library_loaded`, change `echo "  Check workspace.metadata.dylint.libraries in fixtures/consumer/Cargo.toml."` to:

   ```bash
       echo "  Check workspace.metadata.dylint.libraries in $PWD/Cargo.toml."
   ```

4. Replace the block from `require_library_loaded` (the call, not the function) through `exit "$fail"` with:

   ```bash
   # hyp_hegel. hyp_tracing is switched off by fixtures/consumer/dylint.toml,
   # so every package here is also the end-to-end check of that switch.
   cd "$root/fixtures/consumer" || exit 1
   require_library_loaded

   expect_lints good
   expect_lints bad          non_hegel_test crate_without_hegel_tests
   expect_lints exempt_ok
   expect_lints exempt_bad   hegel_exemption_without_justification
   expect_lints exempt_crate

   # hyp_tracing.
   cd "$root/fixtures/tracing_consumer" || exit 1
   require_library_loaded

   expect_lints good
   expect_lints bad          uninstrumented_fn
   expect_lints exempt_ok
   expect_lints exempt_bad   instrument_exemption_without_justification

   exit "$fail"
   ```

5. In the header comment, change "run the real cargo-dylint over the consumer fixture" to "run the real cargo-dylint over the consumer fixtures (fixtures/consumer for hyp_hegel, fixtures/tracing_consumer for hyp_tracing)".

- [ ] **Step 6: Run the e2e**

Run: `./scripts/e2e.sh`
Expected:
```text
ok: lint library loaded (...)
ok: good clean
ok: bad reported crate_without_hegel_tests non_hegel_test
ok: exempt_ok clean
ok: exempt_bad reported hegel_exemption_without_justification
ok: exempt_crate clean
ok: lint library loaded (...)
ok: good clean
ok: bad reported uninstrumented_fn
ok: exempt_ok clean
ok: exempt_bad reported instrument_exemption_without_justification
```
and exit status 0.

- [ ] **Step 7: Prove the consumer off switch is load-bearing**

Temporarily rename `fixtures/consumer/dylint.toml` and re-run `./scripts/e2e.sh`. Expect the consumer packages to FAIL with an extra `uninstrumented_fn`. Restore the file.

- [ ] **Step 8: Commit**

```bash
git add fixtures/tracing_consumer fixtures/consumer/dylint.toml fixtures/action_check/dylint.toml scripts/e2e.sh
git commit -m "test(hyp_tracing): end-to-end fixture workspace; switch it off in the hegel fixtures"
```

---

### Task 10: CI, release check and release notes

**Files:**
- Modify: `.github/workflows/ci.yml` (lines ~64, ~162, ~328)
- Modify: `.github/workflows/release.yml` (line ~101)
- Modify: `scripts/release-check.sh`
- Modify: `scripts/release-notes.py:106`

- [ ] **Step 1: Cache keys include the new lockfile**

In `ci.yml`, both occurrences, and in `release.yml`, change:

```yaml
key: cargo-registry-${{ runner.os }}-${{ hashFiles('lints/Cargo.lock', 'fixtures/consumer/Cargo.lock') }}
```

to:

```yaml
key: cargo-registry-${{ runner.os }}-${{ hashFiles('lints/Cargo.lock', 'fixtures/consumer/Cargo.lock', 'fixtures/tracing_consumer/Cargo.lock') }}
```

- [ ] **Step 2: The action job's "library actually loaded" loop**

In `ci.yml`, change:

```bash
for lint in non_hegel_test hegel_exemption_without_justification crate_without_hegel_tests; do
```

to:

```bash
for lint in non_hegel_test hegel_exemption_without_justification crate_without_hegel_tests uninstrumented_fn instrument_exemption_without_justification; do
```

- [ ] **Step 3: `scripts/release-check.sh`**

1. Add `uninstrumented_fn` and `instrument_exemption_without_justification` to `REQUIRED_LINTS`.
2. Change the comment above the `violating/src/lib.rs` heredoc to: `# Must fire non_hegel_test (a plain #[test]), crate_without_hegel_tests (a test harness with no hegel property test in it) and uninstrumented_fn (add has no #[tracing::instrument]) -- the last proving hyp_tracing loads through the git pin too.`
3. In the `exempted/src/lib.rs` heredoc, add a third crate-level exemption after the two existing ones:

   ```rust
   #![cfg_attr(
       dylint_lib = "hyp_tracing",
       allow(
           uninstrumented_fn,
           reason = "release-check probe: a dependency-free workspace, so tracing is not available"
       )
   )]
   ```

   Update its preceding comment's last sentence to: `All three exemptions are needed: allow(non_hegel_test) does not suppress crate_without_hegel_tests, and neither touches uninstrumented_fn.`
4. Change `expect_lints violating non_hegel_test crate_without_hegel_tests` to:

   ```bash
   expect_lints violating non_hegel_test crate_without_hegel_tests uninstrumented_fn
   ```

- [ ] **Step 4: `scripts/release-notes.py`**

Change line 106 from:

```
All three are **Deny** by default and live in the `hyp_hegel` library.
```

to:

```
Every lint is **Deny** by default. The Library column says which library each
one lives in; `pattern = "lints/*"` loads them all.
```

(The table itself is lifted from README.md, which Task 11 updates.)

- [ ] **Step 5: Validate what can be validated locally**

Run: `bash -n scripts/release-check.sh && bash -n scripts/e2e.sh`
Expected: no output.

Run: `python3 -c 'import yaml,sys; [yaml.safe_load(open(f)) for f in sys.argv[1:]]' .github/workflows/ci.yml .github/workflows/release.yml`
Expected: no output. If PyYAML is missing, skip this command and say so in the task report.

Run: `./scripts/release-check.sh https://github.com/hypotheosis/hyp-rust-policy-lints rev "$(git rev-parse @)"` **only if this branch has been pushed.** Otherwise note in the task report that it has not been run. It fetches from GitHub.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/ci.yml .github/workflows/release.yml scripts/release-check.sh scripts/release-notes.py
git commit -m "ci: assert hyp_tracing loads and fires, in CI and at release"
```

---

### Task 11: README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Intro**

Replace the first paragraph's "One policy ships today: **every test must use the hegel property-testing framework**, and every exemption must carry a written justification." with:

```markdown
Two policies ship today, as two libraries:

- `hyp_hegel`: **every test must use the [hegel](https://crates.io/crates/hegeltest)
  property-testing framework**.
- `hyp_tracing`: **every non-test function must carry
  [`#[tracing::instrument]`](https://docs.rs/tracing/latest/tracing/attr.instrument.html)**.

Every exemption, in either, must carry a written justification.
```

- [ ] **Step 2: `## The lints`**

Replace the section's first paragraph and table with the following. `scripts/release-notes.py` lifts the *first* table in this section, so the combined table must come first:

```markdown
Every lint is **Deny** by default. `pattern = "lints/*"` loads both libraries.
The names and levels below are what `cargo dylint list --all` reports:

| Library | Lint | Level | Fires on |
|---|---|---|---|
| `hyp_hegel` | `non_hegel_test` | deny | A test function that neither expands from a hegel macro nor calls into hegel in its body |
| `hyp_hegel` | `hegel_exemption_without_justification` | deny | An `allow(non_hegel_test)` with no `reason = "..."`, or an empty/whitespace-only one |
| `hyp_hegel` | `crate_without_hegel_tests` | deny | A crate that has a test harness but not one hegel property test. A crate with no tests at all is **not** reported |
| `hyp_tracing` | `uninstrumented_fn` | deny | A non-test function with a body that is not annotated `#[tracing::instrument]` |
| `hyp_tracing` | `instrument_exemption_without_justification` | deny | An `allow(uninstrumented_fn)` with no `reason = "..."` or a blank one, or a `dylint.toml` that switches `hyp_tracing` off without one |

### `hyp_hegel`
```

Keep the existing paragraph ("A test counts as a hegel test if ...") under that new `### hyp_hegel` heading. After it, add:

```markdown
### `hyp_tracing`

In scope: free functions (including ones nested in another function's body),
inherent methods, trait-impl methods, and trait methods with a default body --
including `main`. Out of scope: `const fn` (the attribute cannot apply),
closures, trait methods without a body, and **everything compiled under
`--test`** -- test functions, `#[cfg(test)]` helpers and integration tests.
Production code is checked in its ordinary compilation, so this library, unlike
`hyp_hegel`, does not need `--all-targets`.

A function counts as instrumented when its body was generated by
`#[tracing::instrument]`, whatever arguments it takes (`skip`, `fields`, `err`,
`level`, ...). Detection matches the attribute's defining crate by `[lib]` name
(`tracing_attributes`), so renaming the `tracing` dependency does not affect it.
A span opened by hand (`info_span!(..).entered()`, `.instrument(span)`) does
**not** count; such a function needs the attribute or an `allow`.
```

- [ ] **Step 3: `## Exemptions`**

Insert a `### Exempting from hyp_hegel` heading directly under `## Exemptions`, so the existing hegel content sits beneath it. Demote its two existing subheadings, `### Exempting a whole crate` and `### What the justification check does and does not do`, to `####`. Then, before `## What will bite you`, add:

````markdown
### Exempting from hyp_tracing

The same pattern, with `hyp_tracing` as the `dylint_lib`:

```rust
#[cfg_attr(
    dylint_lib = "hyp_tracing",
    allow(uninstrumented_fn, reason = "called per byte in the decoder hot loop")
)]
fn decode_byte(b: u8) -> u8 {
    b ^ 0x5a
}
```

It can sit on the function, an enclosing `impl` or `mod`, or the crate root.
The crate-root form is the intended adoption path for an existing codebase:
exempt the crate with a reason, then instrument and remove it module by module.

#### Switching `hyp_tracing` off for a whole workspace

A workspace that does not use tracing at all -- a small CLI, say -- turns the
library off in a `dylint.toml` beside its root `Cargo.toml`:

```toml
[hyp_tracing]
enabled = false
reason = "single-shot CLI with no tracing subscriber; spans would go nowhere"
```

`hyp_hegel` is unaffected. The reason is required: `enabled = false` without
one, or with a blank one, fails every crate once with
`instrument_exemption_without_justification`. Unknown keys are an error, so a
typo such as `enable = false` fails the build rather than leaving the policy
silently on.

Not loading the library at all -- `pattern = "lints/hyp_hegel"` instead of
`lints/*` -- also works, but then that workspace stops receiving any library
added in a later release, and nothing records why.
````

- [ ] **Step 4: `## What will bite you`**

Change the opening of the `--all-targets` item from "**`--all-targets` is mandatory.** The lints can only see tests ..." to "**`--all-targets` is mandatory for `hyp_hegel`.** Its lints can only see tests ...". Then add these items at the end of the list, before `## Pinning`:

```markdown
**`hyp_tracing` fires on nearly every function of an existing codebase.**
Adopting a release that includes it is a migration. Start with a justified
crate-level `allow` (see [Exempting from hyp_tracing](#exempting-from-hyp_tracing)) or the `dylint.toml`
switch, and remove it as you go.

**`main` is a function too.** It is in scope like any other. Instrument it, or
exempt it -- typically because it installs the subscriber, so a span opened
before that point is recorded nowhere.

**Functions generated by other attribute macros are not checked.** rustc
cancels diagnostics inside another crate's macro expansion, and
`uninstrumented_fn` deliberately does not opt out (it would otherwise fire on
every derived `Debug`/`Clone` method). So methods rewritten by `#[async_trait]`,
or a `#[tokio::main]` `main`, are not reported, whether instrumented or not.
```

- [ ] **Step 5: `### Adding a lint`**

Append to that section:

```markdown
A new library takes `version.workspace = true`: every crate under `lints/`
shares one version, because one tag selects them all, and cargo-release is
configured with `shared-version = true` to bump them together.
```

- [ ] **Step 6: Check the release notes still render**

Run: `python3 scripts/release-notes.py v9.9.9 > /dev/null && python3 scripts/release-notes.py v9.9.9 | sed -n '1,15p'`
Expected: exit 0, and the output starts with the `## Lints` heading, the new "Every lint is **Deny**" sentence and the five-row table.

- [ ] **Step 7: Commit**

```bash
git add README.md
git commit -m "docs: document the hyp_tracing policy"
```

---

### Task 12: Full verification

- [ ] **Step 1: Everything CI gates on**

Run: `just check`
Expected: `lint`, `test` and `e2e` all succeed. `e2e` prints the eleven `ok:` lines from Task 9 Step 6.

- [ ] **Step 2: Release dry run**

Run: `cd lints && cargo release --no-publish --no-verify patch`. Without `--execute` this is a dry run and changes nothing.
Expected: it plans to bump **both** `hyp_hegel` and `hyp_tracing` to `0.1.2` and to create a **single** tag `v0.1.2`. It does not touch `hegeltest` or `hegeltest-macros`. It may refuse because of the branch or uncommitted changes; if so, read why. A policy refusal is fine and should be reported. A config error is not, and needs fixing.

- [ ] **Step 3: Tidy**

Run: `git status`
Expected: clean. There should be no stray `.stderr` files from "Actual stderr saved to" runs; those are written to `target/`, not the source tree.
