# UI fixtures

Each `.rs` file here is compiled as a **standalone crate** with the lint library
loaded, and its diagnostics are diffed against the `.stderr` file beside it.
New files are auto-discovered — nothing to register.

## Conventions

**No `.stderr` means "assert no diagnostic."** Silence is an assertion, and a
stronger one than matching an error block. Prefer it where the fixture's point
is that something is *accepted*. Do not add an empty `.stderr` — delete the file.

**One fixture, one assertion.** If a fixture can fail for two unrelated reasons,
a failure no longer tells you which. `deny_level.rs` violates this knowingly; it
is the exception, not the pattern.

**Say what the fixture pins, in the fixture.** Several here exist to guard one
specific line of the pass — `no_tests_at_all.rs` is the only thing constraining
the `test_fns.is_empty()` guard, and `mixed_tests.rs` the only thing constraining
the increment-on-hegel-skip path. Without a comment saying so, that guard looks
like dead code to whoever considers deleting it.

**Use the builder form when an attribute must survive.** The stub macro rebuilds
the annotated item from a string, so it **silently discards every attribute on
its input**. A fixture combining `#[expect(...)]` with `#[hegel::test]` compiles,
passes, and proves nothing. Write a plain `#[test]` calling
`hegel::Hegel::new(|tc| ...).run()` instead — no proc macro, nothing to eat the
attribute. See `hegel_stub_macros/src/lib.rs`.

**Keep anything containing `fn `, `{` or `}` below the annotated function.** The
stub finds the name after the first `"fn "` and the body between the first `{`
and the last `}`. A doc comment above the function shifts those match points and
corrupts the expansion.

**A fixture whose subject is the per-test lint needs a hegel test.** Otherwise
`crate_without_hegel_tests` also fires and the fixture asserts two things. The
nine that carry an identical `keeps_crate_lint_quiet` block do so for this
reason; the duplication is deliberate, since each fixture is its own crate and
there is nothing to share it through.

## Regenerating a `.stderr`

There is no bless mode — `dylint_testing` does not plumb one through to
`compiletest_rs`, so `BLESS=1` is silently ignored. Run the test; it prints
`Actual stderr saved to <path>` for each mismatch. **Read that file**, then copy
it into place.

Reading it is not a formality. Several bugs during this library's construction
produced a fully self-consistent set of expectations for wrong behaviour — an
inverted crate-name match, a diagnostic rustc was cancelling before it rendered,
a fixture whose attribute never reached the compiler. Each would have been
frozen in by an unexamined copy.

## Checking a fixture is load-bearing

Break the thing it guards and confirm *this* fixture fails. A fixture that
passes under the mutation it supposedly covers is testing nothing, and the suite
cannot tell you that on its own.
