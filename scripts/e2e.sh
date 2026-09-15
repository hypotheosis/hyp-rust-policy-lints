#!/usr/bin/env bash
# End-to-end check: run the real cargo-dylint over the consumer fixture and
# assert each package produces the expected outcome.
#
# Asserting on the diagnostic codes rather than the exit code alone is
# deliberate: a library that fails to load produces a clean run that is
# otherwise indistinguishable from a passing one. To confirm the assertions
# are still load-bearing, point `libraries` in fixtures/consumer/Cargo.toml at
# a path that matches nothing and check that this script fails.
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

# Every lint hyp_hegel can emit. A lint absent from this list is invisible to
# the set comparison below, so a new lint must be added here.
ALL_LINTS=(
  non_hegel_test
  hegel_exemption_without_justification
  crate_without_hegel_tests
)

fail=0

# expect_lints <pkg> [expected-lint...]
#
# With no expected lints the package must pass. Otherwise it must fail, and
# the set of hyp_hegel lints it reports must equal the expected set exactly.
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
    problem="should have failed but passed (is the library loading?)"
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

expect_lints good
expect_lints bad        non_hegel_test crate_without_hegel_tests
expect_lints exempt_ok
expect_lints exempt_bad hegel_exemption_without_justification

exit "$fail"
