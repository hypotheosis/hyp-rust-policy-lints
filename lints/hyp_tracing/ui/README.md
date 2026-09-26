# UI fixtures

Each `.rs` file is compiled as a **standalone library crate** with the lint
library loaded, and its diagnostics are diffed against the `.stderr` beside it.
New files are auto-discovered.

The general conventions are [`hyp_hegel`'s](../../hyp_hegel/ui/README.md):
no `.stderr` means "assert silence", one fixture makes one assertion, there is
no bless mode, and every fixture is checked to be load-bearing by breaking what
it guards. What differs here:

**Fixtures are libraries.** `tests/ui.rs` passes `--crate-type=lib`, so there is
no `fn main` -- which would otherwise be an uninstrumented function in every
fixture's output. The one exception is `ui_test_build/`, compiled with `--test`
the way cargo compiles tests.

**The real `tracing` crate, no stub.** The detection depends on how
`tracing-attributes` assigns spans, so it is tested against the real thing.
Unlike `hyp_hegel`'s stub, `#[tracing::instrument]` keeps the other attributes
on its function, so `#[expect(...)]` next to it works.

**One directory per compilation mode or `dylint.toml`.** `rustc_flags` and
`dylint_toml` apply to a whole run, so each scenario has its own directory and
its own `#[test]` in `tests/ui.rs`:

| Directory | Build | `dylint.toml` |
|---|---|---|
| `ui/` | `--crate-type=lib` | empty (enforce) |
| `ui_test_build/` | `--test` | empty (enforce) |
| `ui_disabled/` | `--crate-type=lib` | `enabled = false` with a reason |
| `ui_disabled_no_reason/` | `--crate-type=lib` | `enabled = false`, no reason |
| `ui_disabled_blank_reason/` | `--crate-type=lib` | `enabled = false`, whitespace-only reason |
| `ui_enabled_with_reason/` | `--crate-type=lib` | `enabled = true` with a reason (still enforced) |
| `ui_bad_config/` | `--crate-type=lib` | misspelt key `enable = false` |

## Harness notes

- compiletest passes `-A unused` itself, so `dead_code` and `unused_*` warnings
  never appear in a `.stderr`, and a fixture about an unused lint cannot be
  expressed here.
- The harness passes an absolute `src_base`. compiletest replaces the src_base
  string with `$DIR` everywhere in the output, and with a relative `ui` it
  turned words like "builds" into `b$DIRlds`.
- Every run passes a `dylint.toml` (empty by default), so a developer's
  `DYLINT_TOML` or a stray `dylint.toml` cannot change results.
- `tracing` is found by picking the newest `libtracing-*.rlib` in the test
  binary's `deps` directory. This relies on every workspace build asking for the
  same `tracing` features, which is why `hyp_hegel` has an otherwise-unused
  `tracing` dev-dependency. If that breaks, every fixture fails with "cannot
  find attribute `instrument` in `tracing`"; `cargo clean -p tracing` fixes it.
- `dylint_testing` serialises runs with a mutex, so when one UI test fails the
  others in the binary can fail with a `PoisonError`. Run the failing test on
  its own (`cargo test -p hyp_tracing <name>`) to see its real result.
