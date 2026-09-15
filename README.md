# hyp-rust-policy-lints

A [`cargo-dylint`](https://github.com/trailofbits/dylint) lint library that
encodes hypotheosis Rust engineering policy. One policy ships today: **every
test must use the [hegel](https://crates.io/crates/hegeltest) property-testing
framework**, and every exemption must carry a written justification.

Consumers wire it in with one `Cargo.toml` stanza and one CI step. Nothing is
published to crates.io and no prebuilt artifacts are distributed — your CI
compiles the lint library from source on first use of a tag, and caches it.

## The lints

All three are **Deny** by default, live in the `hyp_hegel` library, and are
produced by a single late lint pass. The names and levels below are what
`cargo dylint list --all` reports:

| Lint | Level | Fires on |
|---|---|---|
| `non_hegel_test` | deny | A test function that neither expands from a hegel macro nor calls into hegel in its body |
| `hegel_exemption_without_justification` | deny | An `allow(non_hegel_test)` with no `reason = "..."`, or an empty/whitespace-only one |
| `crate_without_hegel_tests` | deny | A crate that has a test harness but not one hegel property test. A crate with no tests at all is **not** reported |

A test counts as a hegel test if either its expansion chain involves the hegel
macro crate (`#[hegel::test]`) or its body calls into hegel — the builder form,
`hegel::Hegel::new(|tc| { ... }).run()` under a plain `#[test]`. Renaming the
dependency in your `Cargo.toml` does not affect detection: the match is on the
defining crate's `[lib]` name, not its package name or your alias.

## Setup

### 1. Workspace `Cargo.toml`

```toml
[workspace.metadata.dylint]
libraries = [
  { git = "https://github.com/hypotheosis/hyp-rust-policy-lints", tag = "v0.1.0", pattern = "lints/*" },
]

[workspace.lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = ["cfg(dylint_lib, values(any()))"]
```

Substitute the release tag you are adopting for `v0.1.0`; pick it from the
repository's Releases page. Exact tags only — see
[Pinning](#pinning-and-verify-pin).

Each member crate that will write exemptions needs `[lints] workspace = true`
so it inherits the `check-cfg` entry. The `check-cfg` entry is **required**,
not optional; see [What will bite you](#what-will-bite-you).

A single-package repository needs an explicit empty `[workspace]` table above
the metadata. `cargo-dylint` 6.0.4 reads `workspace.metadata.dylint` only —
verified: the same entry written as `package.metadata.dylint` produces
`Warning: No libraries were found.` and a vacuous pass.

### 2. CI

```yaml
- uses: actions/checkout@v5
- uses: hypotheosis/hyp-rust-policy-lints@v0.1.0
```

The action installs `cargo-dylint` and `dylint-link`, caches the cargo
registry, the tools and the compiled lint library, checks that your pin and the
action ref agree, and runs `cargo dylint --all -- --all-targets`.

Inputs, with their real defaults:

| Input | Default | Purpose |
|---|---|---|
| `working-directory` | `.` | Workspace root to check, relative to the checkout |
| `args` | `--all -- --all-targets` | Arguments to `cargo dylint`; everything after `--` goes to the underlying `cargo check` |
| `cargo-dylint-version` | `6.0.4` | Version of `cargo-dylint` and `dylint-link` to install |
| `verify-pin` | `true` | Fail if the workspace pin and the action ref disagree |

If you override `args`, **keep `--all-targets`**.

### Running it by hand

```bash
cargo install --locked cargo-dylint dylint-link
cargo dylint --all -- --all-targets
```

## Exemptions

Silence the per-test lint on one test with a justified `allow`, wrapped in
`cfg_attr(dylint_lib = "hyp_hegel", ...)`:

```rust
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
```

The `cfg_attr` wrapper is what keeps the attribute invisible outside dylint.
`dylint_lib` is set only by the dylint driver, so a bare
`#[allow(non_hegel_test)]` produces `warning: unknown lint: non_hegel_test`
under an ordinary `cargo check`.

The `allow` is found by walking up the HIR parent chain, so it can sit on the
test function, on an enclosing `mod`, or on the crate root. One justified
`allow` on `mod tests` therefore exempts every test in that module — breadth
worth being deliberate about.

### Exempting a whole crate

`allow(non_hegel_test)` does not suppress `crate_without_hegel_tests`. A crate
that has genuinely decided against property testing says so with a second,
crate-level `allow`:

```rust
#![cfg_attr(
    dylint_lib = "hyp_hegel",
    allow(
        non_hegel_test,
        reason = "fixed regulatory identifiers checked against a published table; the inputs are an enumerated set, not a domain to sample"
    )
)]
#![cfg_attr(
    dylint_lib = "hyp_hegel",
    allow(
        crate_without_hegel_tests,
        reason = "fixed regulatory identifiers checked against a published table; the inputs are an enumerated set, not a domain to sample"
    )
)]
```

### What the justification check does and does not do

Only the **presence** of a non-empty reason is checked. Quality is not, and
cannot be: `reason = "n/a"` passes. The empty and whitespace-only cases are
rejected because they are the one degenerate form a lint can detect
mechanically, and `reason = ""` satisfies the letter of the policy while
defeating its purpose. Everything beyond that is a code-review question.

## What will bite you

**`--all-targets` is mandatory.** The lints can only see tests once the test
harness has been compiled. Without `--all-targets` (or `--tests`) the library
loads, finds zero test functions, reports nothing, and the run passes having
checked nothing. Verified against the `bad` fixture, which fires two lints with
the flag and exits 0 without it. The action supplies it by default; a hand-run
`cargo dylint` does not.

**`cargo dylint` exits 0 when no library loads.** If the `libraries` metadata
is missing or the tag does not resolve, `cargo dylint` prints
`Warning: No libraries were found.` on stderr, degrades to a plain
`cargo check`, and exits 0. A green CI step is not by itself evidence the
policy ran. To assert otherwise, check that the lints are listed:

```bash
cargo dylint list --all
```

**The `check-cfg` entry is required.** Without it, every
`cfg_attr(dylint_lib = "hyp_hegel", ...)` exemption produces
`warning: unexpected cfg condition name: dylint_lib` on ordinary
`cargo build` / `cargo check`. Verified by removing the entry from the consumer
fixture.

**Rust 1.81 or later**, for `reason = "..."` in lint attributes. This is a
floor on the *consuming* workspace's toolchain; the lint library builds on its
own pinned nightly, independently of yours.

**A fully exempted crate still fires `crate_without_hegel_tests`.** Exempting
every test individually, each with a justification, does not exempt the crate.
That is deliberate: it keeps "this crate does no property testing" visible as
one fact at the crate root, rather than letting it accumulate one justified
exemption at a time until nobody notices. Silence it explicitly as shown
above.

**Tests from external attribute macros are in scope.** `#[tokio::test]`,
`#[rstest]`, `#[async_std::test]` and friends generate tests whose spans lie
inside another crate's expansion, and rustc cancels such diagnostics unless a
lint opts in. `non_hegel_test` and `hegel_exemption_without_justification` both
set `report_in_external_macro` for exactly this reason — these are the tests
the policy most needs to reach. Verified against a local attribute macro that
emits `#[test]`.

**The first run on a new tag is slow.** Your CI compiles a nightly
`rustc_private` cdylib against `rustc-dev` — minutes, not seconds. The action
caches the built library and the dylint driver keyed on the action ref, so it
is a cold build once per lint release per repository. GitHub scopes cache reads
to the current branch plus its base, so the first run on your default branch
after a bump is the one that pays.

**`python3 >= 3.11` must be on the runner** for the pin check, which parses
your `Cargo.toml` with `tomllib`. Present on all GitHub-hosted images. If yours
lacks it, set `verify-pin: false`.

## Pinning and `verify-pin`

**Exact tags only. There is no moving `v0` tag**, the GitHub Actions convention
notwithstanding. One tag governs both halves of the integration — the action's
behaviour and the lint library it runs — and a moving tag would let a new lint
break your CI with no change on your side.

Two versions are therefore in play on every run: the ref in
`uses: hypotheosis/hyp-rust-policy-lints@<ref>` and the `tag` in your
`libraries` entry. Nothing else ties them together, and drift is silent — CI
keeps passing while enforcing a policy nobody chose. The action's `verify-pin`
step compares them and fails loudly:

```
verify-pin: version drift
  action called at: v0.2.0
  <workspace>/Cargo.toml pins: v0.1.0
  Pin both to the same tag, or set verify-pin: false.
```

It also fails when the manifest has no `dylint.libraries` at all, when the
entry has no `tag`, when the entry pins by `path`, and when `github.action_ref`
is empty (which is what `uses: ./` produces). Each of those is unverifiable
rather than verified, and passing anyway would give the same green tick as a
real check.

Set `verify-pin: false` when you pin the library by `path`, run the action from
a local checkout, or are deliberately testing an unreleased revision.

## Contributing

Two independent Cargo workspaces, two toolchains, no repo-root `Cargo.toml`.
`lints/` builds on the nightly pinned in `lints/rust-toolchain.toml` (rustup
installs it, `rustc-dev` included, on the first `cargo` invocation inside
`lints/`); `fixtures/consumer/` builds on stable, as a real consumer would.

```bash
cargo install --locked cargo-dylint dylint-link   # dylint-link is the linker for lints/
cd lints && cargo test                            # UI fixtures + doctests
./scripts/e2e.sh                                  # real cargo dylint over fixtures/consumer
```

Run bare `cargo test`, not `cargo test --test ui`: the workspace test also
compiles the `declare_lint!` doc comments as doctests, which is what catches a
` ```rust ` fence that should have been ` ```rust,ignore `.

`scripts/e2e.sh` asserts the **complete** set of lints each fixture package
produces, not just one, and separately asserts that the library loaded — a
clean run and a run where nothing loaded are otherwise indistinguishable.

### UI fixtures

Each `.rs` under `lints/hyp_hegel/ui/` is compiled as a standalone crate and
diffed against the `.stderr` beside it. New files are auto-discovered. **No
`.stderr` means "assert no diagnostic"** — do not add an empty one.

**There is no bless mode.** `dylint_testing` does not plumb one through to
`compiletest_rs`, so `BLESS=1` is silently ignored. Run the test; on a mismatch
it prints `Actual stderr saved to <path>`. Read that file, confirm the
behaviour it describes is the behaviour you want, then copy it into place.
Reading it is not a formality — several bugs during this library's construction
produced a fully self-consistent set of expectations for wrong behaviour.

The fixture conventions — and the traps in the hegel stub macro, which
rebuilds its input from a string and so silently discards every attribute —
are in [`lints/hyp_hegel/ui/README.md`](lints/hyp_hegel/ui/README.md).

### Adding a lint

A new policy is a new directory under `lints/`. The workspace's
`members = ["*"]` and the consumer-side `pattern = "lints/*"` both pick it up
with no other change. Closely related lints that share a pass belong in one
crate; unrelated policies get their own.

## Operational note: toolchain bumps are releases

`cargo dylint` is not `cargo clippy`. Consumers compile a nightly
`rustc_private` cdylib on first use of a tag, so the channel in
`lints/rust-toolchain.toml` is part of the public contract, and it must move in
lockstep with the `clippy_utils` rev in `lints/Cargo.toml` — the two are only
compatible in matched pairs.

A nightly bump therefore ships as a deliberate version bump, validated at
release time against the real `git`/`tag` consumer path, not as a routine
dependency update. Tag pinning means the
breakage never arrives unannounced; it does not mean it is cheap.
