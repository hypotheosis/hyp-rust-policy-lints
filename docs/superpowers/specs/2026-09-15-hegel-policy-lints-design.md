# hyp-rust-policy-lints: cargo-dylint policy lints

**Date:** 2026-09-15
**Status:** Approved design, not yet implemented

## Purpose

Provide a versioned, importable library of custom `cargo-dylint` lints that
encode hypotheosis Rust engineering policy. Consumer repositories opt in with a
single `workspace.metadata.dylint` entry plus a one-step GitHub Action.

The proof-of-concept policy: **every test must use the hegel property-testing
framework**, and any exemption must carry a written justification.

## Non-goals

- Publishing lint crates to crates.io (`publish = false` throughout).
- Distributing prebuilt `.so` artifacts. Consumers compile from source; dylint
  caches the result.
- Any lint beyond the hegel policy in v0.1.0.

---

## 1. Repository layout

```
hyp-rust-policy-lints/
├── action.yml                    # root composite action (see §5.2)
├── lints/                        # self-contained workspace, nightly-pinned
│   ├── Cargo.toml                # [workspace] members = ["*"]
│   ├── Cargo.lock                # committed
│   ├── rust-toolchain.toml
│   ├── .cargo/config.toml
│   └── hyp_hegel/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs            # register_lints entry point
│       │   └── hegel_tests.rs    # the LateLintPass
│       └── ui/                   # dylint_testing fixtures
│           ├── hegel_test.rs / .stderr
│           ├── plain_test.rs / .stderr
│           ├── builder_form.rs / .stderr
│           ├── allow_unjustified.rs / .stderr
│           ├── allow_justified.rs / .stderr
│           └── no_hegel_tests.rs / .stderr
├── fixtures/consumer/            # separate workspace, stable toolchain
│   ├── Cargo.toml
│   ├── good/
│   ├── bad/
│   ├── exempt_ok/
│   └── exempt_bad/
├── docs/superpowers/specs/
└── .github/workflows/
    ├── ci.yml
    └── release.yml
```

### 1.1 Two independent workspaces

`lints/` and `fixtures/consumer/` are separate Cargo workspaces with separate
toolchains. There is no repo-root `Cargo.toml` and no repo-root `cargo build`.

This is forced by `rustc_private`: lint crates must build on the exact nightly
that matches the `clippy_utils` revision, while the consumer fixture must build
on stable to represent a real customer. CI therefore runs two jobs with two
different working directories.

### 1.2 `lints/rust-toolchain.toml`

```toml
[toolchain]
channel = "nightly-2026-07-09"
components = ["llvm-tools-preview", "rustc-dev"]
```

Matches the dylint 6.0.4 template. Bumping this channel is a **release event**,
not a routine dependency update: it changes the compiler every consumer pinned
to a subsequent tag will build against. See §7.

### 1.3 `lints/.cargo/config.toml`

```toml
[target.'cfg(all())']
rustflags = ["-C", "linker=dylint-link"]
```

`dylint-link` must be installed (`cargo install dylint-link`) before anything in
`lints/` will build.

### 1.4 `lints/Cargo.toml` (workspace root)

```toml
[workspace]
members = ["*"]
exclude = [".cargo"]

[workspace.dependencies]
clippy_utils = { git = "https://github.com/rust-lang/rust-clippy", rev = "09382ed3c34e091d7705f96964e303b59978533c" }
dylint_linting = "6.0"
dylint_testing = "6.0"

[workspace.lints.rust.unexpected_cfgs]
level = "deny"
check-cfg = ["cfg(dylint_lib, values(any()))"]
```

The `clippy_utils` rev is pinned to the same commit as the dylint 6.0.4 template
and must be bumped in lockstep with `rust-toolchain.toml`.

### 1.5 `lints/hyp_hegel/Cargo.toml`

```toml
[package]
name = "hyp_hegel"
version = "0.1.0"
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

The `rlib` feature follows dylint's own convention so that a future umbrella
crate can aggregate several lint crates without each one emitting its own
`register_lints` symbol.

### 1.6 Adding future lints

A new policy is a new directory under `lints/`. The workspace `members = ["*"]`
glob and the consumer-side `pattern = "lints/*"` both pick it up with no other
change. Unrelated policies get separate crates; closely related lints that share
a pass live in one crate.

---

## 2. Consumer integration contract

A consuming workspace adds:

```toml
[workspace.metadata.dylint]
libraries = [
  { git = "https://github.com/hypotheosis/hyp-rust-policy-lints",
    tag = "v0.1.0",
    pattern = "lints/*" },
]

[workspace.lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = ["cfg(dylint_lib, values(any()))"]
```

and invokes:

```
cargo dylint --all -- --all-targets
```

### 2.1 `--all-targets` is mandatory

The lint cannot see any test unless the crate is compiled with the test harness
enabled. Without `--all-targets` (or `--tests`) the lint loads, finds zero test
functions, and silently reports nothing. The composite action supplies this by
default so consumers cannot get it wrong.

### 2.2 The `check-cfg` entry is mandatory

dylint passes `--cfg=dylint_lib="hyp_hegel"` when running. Consumer exemption
attributes are written as `cfg_attr(dylint_lib = "hyp_hegel", ...)`, which
rustc's `unexpected_cfgs` lint will flag during ordinary `cargo build` unless
`check-cfg` declares it.

### 2.3 Minimum consumer Rust version

1.81, for `reason = "..."` in lint attributes (§3.3).

---

## 3. Lint design

One `LateLintPass` in `hyp_hegel` registering three lints, all **Deny** by
default.

| Lint | Level | Fires on |
|---|---|---|
| `non_hegel_test` | Deny | A test function whose expansion chain does not involve the `hegel` crate |
| `hegel_exemption_without_justification` | Deny | An `allow(non_hegel_test)` carrying no `reason = "..."` |
| `crate_without_hegel_tests` | Deny | Crate has a test harness but zero hegel tests |

Because there are three lints in one crate, `declare_late_lint!` is not usable.
`src/lib.rs` hand-writes the entry point:

```rust
#![feature(rustc_private)]

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod hegel_tests;

#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[
        hegel_tests::NON_HEGEL_TEST,
        hegel_tests::HEGEL_EXEMPTION_WITHOUT_JUSTIFICATION,
        hegel_tests::CRATE_WITHOUT_HEGEL_TESTS,
    ]);
    lint_store.register_late_lint_pass(Box::new(|_| {
        Box::<hegel_tests::HegelTests>::default()
    }));
}
```

### 3.1 Identifying test functions

By the time a late pass runs, `#[test]` no longer exists as an attribute. The
test harness has lowered each test into a generated `const` of type
`test::TestDescAndFn` whose `testfn` field holds
`StaticTestFn(|| assert_test_result(actual_fn()))`.

`check_crate` therefore walks the crate's items once and collects the `DefId` of
each such `actual_fn` into `test_fns: Vec<DefId>`. `check_item` then asks
whether the item's `DefId` is in that set. This mirrors dylint's own
`non_thread_safe_call_in_test`, which solves the identical problem.

Practical consequence: `--all-targets` must be in effect (§2.1), and UI tests
must pass `.rustc_flags(["--test"])`.

### 3.2 Identifying hegel usage

A test counts as hegel-based if **either** condition holds:

1. **Expansion chain.** Walking `span.ctxt()` upward through `outer_expn_data()`
   yields an `ExpnData` whose `macro_def_id` belongs to a crate named `hegel`.
   This catches `#[hegel::test]` and `#[hegel::state_machine]`, because the
   builtin `#[test]` expansion they produce is nested inside the hegel
   proc-macro expansion.

2. **Body scan fallback.** The test function's body contains a call whose
   resolved `DefId` belongs to the `hegel` crate. This catches the documented
   builder form:

   ```rust
   #[test]
   fn t() { Hegel::new(|tc| { /* ... */ }).run(); }
   ```

   which has no hegel expansion at all and would otherwise be a false positive.

Crate identification is by name, resolved through
`cx.tcx.crate_name(def_id.krate)`. Note that the hegel crate is published as
`hegeltest` and conventionally renamed on import
(`hegel = { package = "hegeltest" }`), because its proc-macros hard-code the
`hegel::` path. The lint matches on the crate name as the compiler sees it;
**the implementation must verify empirically whether that resolves to `hegel`
or `hegeltest`** and match both if necessary.

### 3.3 Justification enforcement

`#[allow(non_hegel_test)]` suppresses `non_hegel_test` by construction, so the
pass cannot rely on emitting and being silenced. Instead, for every non-hegel
test the pass explicitly queries the lint level before emitting:

```
level = cx.tcx.lint_level_at_node(NON_HEGEL_TEST, hir_id)
```

and branches on the result:

| Level | `LintLevelSource::Node` reason | Action |
|---|---|---|
| not Allow | — | emit `non_hegel_test` on the test |
| Allow | `Some(_)` | silent — exemption accepted |
| Allow | `None` | emit `hegel_exemption_without_justification` at the attribute span |

`LintLevelSource::Node` carries `reason: Option<Symbol>`, which is populated
from `reason = "..."`. Because the second lint is a *different* lint, an
`allow(non_hegel_test)` does not suppress it.

The accepted exemption form is:

```rust
#[cfg_attr(dylint_lib = "hyp_hegel",
           allow(non_hegel_test,
                 reason = "asserts an exact serialized byte layout; no general property holds"))]
#[test]
fn golden_wire_format() { /* ... */ }
```

The intent is that silencing this lint requires articulating *why* a property
test does not apply, which is a reasoning step rather than a mechanical one.

### 3.4 Crate-level check

`check_crate_post` emits `crate_without_hegel_tests` when:

```
test_fns.len() > 0 && hegel_test_count == 0
```

A crate with no test harness at all stays silent — "this crate has no tests" is
a different policy question and is out of scope here. A crate that has opted
into testing but uses no property testing is in violation.

The diagnostic is emitted at the crate root span with a note naming the
`#[hegel::test]` attribute as the remedy.

### 3.5 Lint documentation

Each lint carries the standard dylint doc-comment sections (`### What it does`,
`### Why is this bad?`, `### Example` / `Use instead`), which surface in
`cargo dylint list` and in the generated README.

---

## 4. Testing strategy

Two layers, because they fail in different ways.

### 4.1 UI tests (`lints/hyp_hegel/ui/`)

Driven by `dylint_testing`:

```rust
#[test]
fn ui() {
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui")
        .rustc_flags(["--test"])
        .run();
}
```

Each fixture is a `.rs` file paired with an expected `.stderr`:

| Fixture | Expectation |
|---|---|
| `hegel_test.rs` | `#[hegel::test]` — clean |
| `plain_test.rs` | plain `#[test]` — `non_hegel_test` |
| `builder_form.rs` | `#[test]` + `Hegel::new(..).run()` — clean (guards §3.2 fallback) |
| `allow_unjustified.rs` | allow without reason — `hegel_exemption_without_justification` |
| `allow_justified.rs` | allow with reason — clean |
| `no_hegel_tests.rs` | tests present, none hegel — `crate_without_hegel_tests` |

Fixtures that reference hegel need a minimal stub providing the
`#[hegel::test]` macro and `Hegel` type, so UI tests do not depend on fetching
the real crate.

### 4.2 End-to-end consumer smoke test (`fixtures/consumer/`)

UI tests verify the pass logic but exercise none of the packaging: the glob
pattern, the cdylib entry point, the `dylint_lib` cfg, and `--all-targets` are
all invisible to them. The fixture workspace covers that gap by wiring the lints
in exactly as a customer would:

```toml
[workspace.metadata.dylint]
libraries = [{ path = "../../lints/*" }]
```

`path` rather than `git` because at PR time the tag does not exist yet; this
validates the working tree. The `git`/`tag` path is validated separately at
release time (§5.3).

Assertions, all run under `cargo dylint --all -- --all-targets`:

| Package | Expected |
|---|---|
| `good` | exit 0 — all tests are `#[hegel::test]` |
| `bad` | non-zero exit **and** stderr contains `non_hegel_test` |
| `exempt_ok` | exit 0 — one non-hegel test behind a justified allow |
| `exempt_bad` | non-zero exit **and** stderr contains `hegel_exemption_without_justification` |

Each is a distinct package, so a single `cargo dylint` invocation per package
yields one unambiguous expected outcome.

Matching on the diagnostic name, not merely the exit code, is deliberate: a
library that fails to load produces a clean run that is otherwise
indistinguishable from a passing one.

---

## 5. GitHub Actions

### 5.1 `ci.yml` — push and pull_request

Two jobs.

**`lints`** — working directory `lints/`, toolchain resolved from
`rust-toolchain.toml`:

1. `cargo install --locked dylint-link`
2. `cargo fmt --check`
3. `cargo clippy --all-targets -- -D warnings`
4. `cargo test` (runs the UI fixtures)

**`e2e`** — `needs: lints`, working directory `fixtures/consumer/`, stable
toolchain:

1. `cargo install --locked cargo-dylint dylint-link`
2. Run the §4.2 assertion matrix.

Both jobs cache `~/.cargo` and `~/.local/share/dylint`.

### 5.2 `action.yml` — root composite action

Consumers invoke the policy check as a single step:

```yaml
- uses: actions/checkout@v5
- uses: hypotheosis/hyp-rust-policy-lints@v0.1.0
```

`uses: owner/repo@ref` resolves an `action.yml` at the repository root. A
reusable workflow could only be addressed by its full
`.github/workflows/<file>.yml@ref` path, and running a check against
already-checked-out code is step-shaped rather than job-shaped, so a composite
action is both shorter and structurally correct. No Marketplace publication is
required for `uses:` to resolve.

Inputs:

| Input | Default | Purpose |
|---|---|---|
| `working-directory` | `.` | Workspace root to check |
| `args` | `--all -- --all-targets` | Passed to `cargo dylint` |
| `cargo-dylint-version` | `6.0.4` | Pinned installer version |
| `verify-pin` | `true` | Enforce §5.2.1 |

Steps: install `cargo-dylint` and `dylint-link` with `--locked`; restore caches
for `~/.cargo/bin`, the registry, and `~/.local/share/dylint`, keyed on the
action ref; optionally verify the pin; run `cargo dylint ${{ inputs.args }}`.

Caching keyed on the action ref is the load-bearing part. Without it every
consumer CI run recompiles a nightly `rustc_private` cdylib from scratch, which
costs minutes on every job. Keyed on the tag, it is a cold build once per lint
release per repository.

#### 5.2.1 Pin verification

The action reads the consuming workspace's `Cargo.toml`, extracts the `tag`
from the `hyp-rust-policy-lints` entry in `workspace.metadata.dylint.libraries`,
and fails if it does not equal the ref the action itself was called at.

Drift between "which action version ran" and "which lint version it ran" is
otherwise invisible and produces results that are very hard to interpret. Since
the project uses exact tags only (§5.3), strict equality is the correct check.

Set `verify-pin: false` to opt out — appropriate for a repository deliberately
testing an unreleased lint revision.

### 5.3 `release.yml` — on tag push matching `v*`

1. Re-run the full §5.1 matrix against the tagged commit.
2. **Validate the real consumer path.** Generate a throwaway workspace in a temp
   directory whose `Cargo.toml` contains
   `{ git = "https://github.com/hypotheosis/hyp-rust-policy-lints", tag = "<new tag>", pattern = "lints/*" }`,
   then run the §4.2 good/bad assertions through it.
3. On success, cut a GitHub Release listing the lints in the library.

Step 2 is the only place the `git` + `tag` + `pattern` resolution — precisely
what consumers depend on — is actually exercised. PR CI structurally cannot
test it, because the tag does not exist until the release. A failure here means
deleting the tag before anyone pins it.

### 5.4 Tag strategy

**Exact tags only.** No moving `v0` or `v1` tag is maintained, despite that
being conventional for GitHub Actions.

A moving tag would mean the action and the lint library could sit at different
versions, which defeats §5.2.1, and a moving tag on the `libraries` entry would
reintroduce exactly the failure mode that tag-pinning was chosen to avoid: a new
lint breaking every consumer's CI with no change on their side. One tag governs
both halves of the integration.

---

## 6. Error handling and failure modes

| Failure | Surface | Mitigation |
|---|---|---|
| `dylint-link` not installed | Link error building any lint crate | Installed by CI and by the composite action |
| `--all-targets` omitted | Lint silently reports nothing | Supplied by the action's default `args` |
| `check-cfg` missing in consumer | `unexpected_cfgs` warnings on exemption attributes | Documented in README; consumer-side requirement |
| Library fails to load | Clean run indistinguishable from passing | E2E asserts on diagnostic text, not exit code alone |
| Action/library version drift | Confusing results | `verify-pin` (§5.2.1) |
| Nightly bump breaks `clippy_utils` | Every consumer on that tag fails to build | Tag pinning; bump is a release event (§7) |

---

## 7. Operational note: toolchain bumps are releases

`cargo dylint` is not `cargo clippy`. Consumers compile a nightly
`rustc_private` cdylib on first use of a tag, so the nightly channel in
`lints/rust-toolchain.toml` is part of the public contract. A nightly that
breaks `clippy_utils` breaks every consumer pinned to a tag built against it.

Tag pinning means breakage never arrives unannounced, but it also means
`rust-toolchain.toml` and the `clippy_utils` rev must move together, be
validated by `release.yml`, and ship as a deliberate version bump rather than a
routine dependency update.

---

## 8. Open implementation questions

These are deliberately deferred to implementation, where they can be resolved
empirically rather than guessed:

1. **Crate name resolution for hegel** (§3.2) — whether `cx.tcx.crate_name`
   yields `hegel` or `hegeltest` given the conventional rename. Match both if
   ambiguous.
2. **`LintLevelSource::Node` field shape** (§3.3) on nightly-2026-07-09 — the
   API is internal and version-sensitive; confirm the `reason` field before
   building on it.
3. **Body-scan fallback depth** (§3.2) — whether a direct call scan suffices or
   the builder form needs deeper traversal to catch `Hegel::new` behind a helper.
