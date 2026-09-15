#!/usr/bin/env bash
# End-to-end check: run the real cargo-dylint over the consumer fixture and
# assert each package produces the expected outcome.
#
# Asserting on the diagnostic codes rather than the exit code alone is
# deliberate: a library that fails to load produces a clean run that is
# otherwise indistinguishable from a passing one. `cargo dylint` degrades
# silently to a plain `cargo check` when it finds no libraries, so a
# clean-expecting package cannot by itself tell "the lint ran and found
# nothing" from "no lint ran at all". `require_library_loaded` below is what
# makes that distinction; do not remove it on the grounds that the
# failure-expecting packages would catch it anyway. They only would while the
# fixture mix happens to contain some, which is an accident, not a guarantee.
#
# Each package is checked against the *complete* set of lints it should
# produce, not just one. A package firing an extra lint is as much a
# regression as one firing too few: `bad` must report both `non_hegel_test`
# and `crate_without_hegel_tests`, and asserting only the first would hide the
# day the crate-level lint stops firing.
#
# Detection reads `--message-format=json` rather than grepping the rendered
# text, because a lint's name appears in other lints' prose. The
# `hegel_exemption_without_justification` diagnostic quotes
# `allow(non_hegel_test)` in its own message, so a text grep reports
# `non_hegel_test` for a package where it never fired.
set -uo pipefail

cd "$(dirname "$0")/../fixtures/consumer" || exit 1

# The lints these fixtures actually exercise. Used only as the precondition's
# checklist -- the set comparison itself runs against ALL_LINTS, which is
# derived from the driver, so a lint missing from this list cannot silently
# disable a comparison.
REQUIRED_LINTS=(
  non_hegel_test
  hegel_exemption_without_justification
  crate_without_hegel_tests
)

# Every lint the loaded libraries can emit. Filled in by
# require_library_loaded from `cargo dylint list`, so a lint added to the
# library is compared against automatically: a package that starts firing a
# brand-new lint fails here instead of being invisible, which is what a
# hand-maintained array could not give us.
ALL_LINTS=()

fail=0

contains() {
  local needle="$1" item
  shift
  for item in "$@"; do
    [ "$item" = "$needle" ] && return 0
  done
  return 1
}

# Assert the lint library loaded, and derive ALL_LINTS from the same output.
#
# This must run before any per-package check. On failure the script stops:
# every subsequent result would be meaningless, and a wall of per-package
# failures would bury the one line that explains them.
require_library_loaded() {
  local out missing=() lint

  # `cargo dylint list` exits 0 even when it finds no libraries at all -- it
  # prints `Warning: No libraries were found` on stderr and nothing on stdout
  # -- so the exit status proves nothing and the output has to be read.
  # stdout alone is taken, because the build chatter on stderr mentions the
  # library by name and would satisfy a careless grep.
  out=$(cargo dylint list --all 2>/dev/null)

  # Output is a library name in column 0 followed by its lints, each indented
  # and starting with the lint name:
  #
  #     hyp_hegel
  #         crate_without_hegel_tests    deny    crate has tests but no ...
  mapfile -t ALL_LINTS < <(awk '/^[[:space:]]+[a-z_]/ { print $1 }' <<<"$out" | sort -u)

  for lint in "${REQUIRED_LINTS[@]}"; do
    contains "$lint" "${ALL_LINTS[@]+"${ALL_LINTS[@]}"}" || missing+=("$lint")
  done

  if [ ${#missing[@]} -gt 0 ]; then
    echo "FAIL: lint library did not load"
    echo "  'cargo dylint list --all' did not report: ${missing[*]}"
    echo "  Every check below would pass vacuously, so none was run."
    echo "  Check workspace.metadata.dylint.libraries in fixtures/consumer/Cargo.toml."
    echo "--- cargo dylint list --all ---"
    cargo dylint list --all 2>&1
    exit 1
  fi

  echo "ok: lint library loaded (${ALL_LINTS[*]})"
}

# expect_lints <pkg> [expected-lint...]
#
# With no expected lints the package must pass. Otherwise it must fail, and
# the set of lints it reports must equal the expected set exactly.
expect_lints() {
  local pkg="$1"
  shift
  local expected=("$@")
  local json status stderr

  stderr=$(mktemp)
  # Capture the status on its own line: `$?` after `local json=$(...)` is the
  # status of `local`, not of the command, and would always be 0 -- which
  # would make a clean-expecting check pass unconditionally, exactly the
  # silent-pass failure mode this script exists to rule out.
  json=$(cargo dylint --all -- --all-targets --message-format=json -p "$pkg" 2>"$stderr")
  status=$?

  local actual=() lint
  for lint in "${ALL_LINTS[@]}"; do
    if grep -qF "\"code\":{\"code\":\"$lint\"" <<<"$json"; then
      actual+=("$lint")
    fi
  done

  local want have
  want=$(printf '%s\n' "${expected[@]+"${expected[@]}"}" | sort | tr '\n' ' ')
  have=$(printf '%s\n' "${actual[@]+"${actual[@]}"}" | sort | tr '\n' ' ')

  local problem=""
  if [ ${#expected[@]} -eq 0 ] && [ "$status" -ne 0 ]; then
    problem="should have passed but exited $status"
  elif [ ${#expected[@]} -gt 0 ] && [ "$status" -eq 0 ]; then
    problem="should have failed but passed"
  elif [ "$want" != "$have" ]; then
    problem="reported lints [$have], expected [$want]"
  fi

  if [ -n "$problem" ]; then
    echo "FAIL: $pkg $problem"
    cat "$stderr"
    # Re-run in the default format so the diagnostics are readable; the check
    # above is already done, and the build is warm, so this is cheap and only
    # happens on failure.
    cargo dylint --all -- --all-targets -p "$pkg" 2>&1
    fail=1
  elif [ ${#expected[@]} -eq 0 ]; then
    echo "ok: $pkg clean"
  else
    echo "ok: $pkg reported ${have% }"
  fi

  rm -f "$stderr"
}

require_library_loaded

expect_lints good
expect_lints bad          non_hegel_test crate_without_hegel_tests
expect_lints exempt_ok
expect_lints exempt_bad   hegel_exemption_without_justification
expect_lints exempt_crate

exit "$fail"
