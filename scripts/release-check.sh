#!/usr/bin/env bash
# Release-time validation of the real `git` + `pattern` consumer path.
#
# Everything else in this repository pins the lint library by `path`:
# fixtures/consumer, fixtures/action_check and therefore scripts/e2e.sh and the
# whole of ci.yml. That is not laziness -- at pull-request time the release tag
# does not exist yet, so there is nothing for a `tag` pin to resolve to. The
# consequence is that the form every consumer actually writes
#
#     { git = "https://github.com/hypotheosis/hyp-rust-policy-lints",
#       tag = "vX.Y.Z", pattern = "lints/*" }
#
# is never exercised by pull-request CI. This script is the only place it is.
# It builds a throwaway consumer workspace in a temp directory, pins the
# library through the given git ref, and asserts that the lints both *load* and
# *fire* through that path.
#
# Usage: release-check.sh <git-url> <tag|rev|branch> <value>
#
#   release-check.sh https://github.com/hypotheosis/hyp-rust-policy-lints tag v0.1.0
#   release-check.sh https://github.com/hypotheosis/hyp-rust-policy-lints rev 90047ea...
#
# The pin key is a separate argument rather than something inferred from the
# value. cargo-dylint spells a git pin as exactly one of `tag =`, `rev =` or
# `branch =` -- the same three keys cargo uses for a git dependency -- and they
# are not interchangeable: `rev = "v0.1.0"` and `tag = "<sha>"` both fail to
# resolve. Guessing from the shape of the value would be guessing (a 40-hex
# string is a plausible tag name, and a branch name is indistinguishable from a
# tag name), so the caller says which it means. The release workflow passes
# `tag`; a maintainer validating an unreleased revision passes `rev` or
# `branch`, which exercises the identical git-fetch-clone-build-glob path and
# leaves only the literal key name untested.
#
# The assertions mirror scripts/e2e.sh, and for the same reasons documented at
# length there: `cargo dylint` degrades to a plain `cargo check` and exits 0
# when it finds no libraries, so a clean run and a run where nothing loaded are
# indistinguishable from the exit code alone. Hence the explicit
# `cargo dylint list` precondition, and hence a fixture that must *fail*
# alongside one that must pass.
set -uo pipefail

# The lints the probe workspace exercises. Used as the precondition's
# checklist; the per-package set comparison runs against ALL_LINTS, which is
# derived from the driver, so a lint added to the library is compared against
# automatically.
REQUIRED_LINTS=(
  non_hegel_test
  hegel_exemption_without_justification
  crate_without_hegel_tests
)

ALL_LINTS=()
fail=0

usage() {
  cat >&2 <<'EOF'
usage: release-check.sh <git-url> <tag|rev|branch> <value>

  release-check.sh https://github.com/hypotheosis/hyp-rust-policy-lints tag v0.1.0
  release-check.sh https://github.com/hypotheosis/hyp-rust-policy-lints rev 1a2b3c4
EOF
}

if [ "$#" -ne 3 ]; then
  usage
  exit 2
fi

url=$1
key=$2
value=$3

case "$key" in
  tag | rev | branch) ;;
  *)
    echo "release-check: unknown pin key '$key'" >&2
    usage
    exit 2
    ;;
esac

if [ -z "$value" ]; then
  echo "release-check: empty value for '$key'" >&2
  exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "release-check: cargo is not on PATH" >&2
  exit 2
fi

if ! cargo dylint --version >/dev/null 2>&1; then
  echo "release-check: 'cargo dylint' is not available" >&2
  echo "release-check: cargo install --locked cargo-dylint dylint-link" >&2
  exit 2
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/hyp-release-check.XXXXXXXX") || exit 2

# The temp directory is kept on failure: the build output and the resolved git
# checkout under it are the evidence you need to tell "the tag does not
# resolve" from "the library does not build on this toolchain".
cleanup() {
  if [ "$fail" -eq 0 ]; then
    rm -rf "$work"
  else
    echo "release-check: probe workspace kept at $work"
  fi
}
trap cleanup EXIT

echo "release-check: probing $url ($key = $value)"
echo "release-check: workspace $work"

# The probe workspace. Two packages, no third-party dependencies at all --
# deliberately not even hegel. Pulling hegeltest from crates.io would add a
# second way for this check to go red that has nothing to do with the git pin,
# and the one thing being tested here is the pin. A hegel-using package is not
# needed to prove the lints fire: a plain `#[test]` is already a violation.
mkdir -p "$work/violating/src" "$work/exempted/src"

# `pattern = "lints/*"` is the glob consumers write, and it is part of what is
# under test: it has to match the lints/ directory *inside the fetched
# checkout*. `[workspace.lints]` with the check-cfg entry is the other half of
# the documented consumer setup, and the exempted package below would warn
# without it.
cat > "$work/Cargo.toml" <<EOF
[workspace]
members = ["violating", "exempted"]
resolver = "2"

[workspace.metadata.dylint]
libraries = [
  { git = "$url", $key = "$value", pattern = "lints/*" },
]

[workspace.lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = ["cfg(dylint_lib, values(any()))"]
EOF

cat > "$work/violating/Cargo.toml" <<'EOF'
[package]
name = "violating"
version = "0.1.0"
edition = "2021"
publish = false

[lints]
workspace = true
EOF

# Must fire non_hegel_test (a plain #[test]) and crate_without_hegel_tests (a
# test harness with no hegel property test in it).
cat > "$work/violating/src/lib.rs" <<'EOF'
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
EOF

cat > "$work/exempted/Cargo.toml" <<'EOF'
[package]
name = "exempted"
version = "0.1.0"
edition = "2021"
publish = false

[lints]
workspace = true
EOF

# Must be clean. A check that only ever asserts "lints fired" would stay green
# if the library started reporting every package unconditionally, and it also
# would not prove that the `cfg_attr(dylint_lib = ...)` exemption path survives
# the git-fetched build. Both exemptions are needed: allow(non_hegel_test) does
# not suppress crate_without_hegel_tests.
cat > "$work/exempted/src/lib.rs" <<'EOF'
#![cfg_attr(
    dylint_lib = "hyp_hegel",
    allow(
        non_hegel_test,
        reason = "release-check probe: fixed table lookup, an enumerated set rather than a domain to sample"
    )
)]
#![cfg_attr(
    dylint_lib = "hyp_hegel",
    allow(
        crate_without_hegel_tests,
        reason = "release-check probe: fixed table lookup, an enumerated set rather than a domain to sample"
    )
)]

pub fn alpha3(alpha2: &str) -> Option<&'static str> {
    match alpha2 {
        "GB" => Some("GBR"),
        "NZ" => Some("NZL"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::alpha3;

    #[test]
    fn known_codes_expand() {
        assert_eq!(alpha3("GB"), Some("GBR"));
    }
}
EOF

cd "$work" || exit 2

contains() {
  local needle="$1" item
  shift
  for item in "$@"; do
    [ "$item" = "$needle" ] && return 0
  done
  return 1
}

# Assert the library loaded through the git pin, and derive ALL_LINTS from the
# same output. This is the step that proves the ref resolved, the checkout
# built and `pattern = "lints/*"` matched; everything after it would pass
# vacuously otherwise. On failure the script stops.
require_library_loaded() {
  local out err missing=() lint

  err=$(mktemp)
  # This is the call that does the fetching and the (cold, several-minute)
  # build of the rustc_private cdylib, so its stderr is worth keeping: a ref
  # that does not resolve and a library that does not compile both surface
  # only there. `cargo dylint list` exits 0 even when it finds nothing, so the
  # status proves nothing and stdout has to be read.
  out=$(cargo dylint list --all 2>"$err")

  mapfile -t ALL_LINTS < <(awk '/^[[:space:]]+[a-z_]/ { print $1 }' <<<"$out" | sort -u)

  for lint in "${REQUIRED_LINTS[@]}"; do
    contains "$lint" "${ALL_LINTS[@]+"${ALL_LINTS[@]}"}" || missing+=("$lint")
  done

  if [ ${#missing[@]} -gt 0 ]; then
    fail=1
    echo "FAIL: the lint library did not load through the $key pin"
    echo "  pin:  { git = \"$url\", $key = \"$value\", pattern = \"lints/*\" }"
    echo "  'cargo dylint list --all' did not report: ${missing[*]}"
    echo "  Nothing below was checked: every assertion would have passed vacuously."
    echo "  Likely causes: the ref does not resolve in the remote, the"
    echo "  pattern does not match lints/ in the fetched checkout, or the"
    echo "  library failed to build on its pinned nightly."
    echo "--- cargo dylint list --all (stdout) ---"
    echo "$out"
    echo "--- cargo dylint list --all (stderr) ---"
    cat "$err"
    rm -f "$err"
    exit 1
  fi

  rm -f "$err"
  echo "ok: lint library loaded through the $key pin (${ALL_LINTS[*]})"
}

# expect_lints <pkg> [expected-lint...]
#
# With no expected lints the package must pass. Otherwise it must fail and the
# set of lints it reports must equal the expected set exactly. Lifted from
# scripts/e2e.sh, including the reason for capturing the status on its own
# line: `$?` after `local json=$(...)` is the status of `local`, not of the
# command, so a clean-expecting check would pass unconditionally.
expect_lints() {
  local pkg="$1"
  shift
  local expected=("$@")
  local json status stderr

  stderr=$(mktemp)
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

expect_lints violating non_hegel_test crate_without_hegel_tests
expect_lints exempted

if [ "$fail" -ne 0 ]; then
  echo
  echo "release-check: FAILED for $url ($key = $value)"
  if [ "$key" = "tag" ]; then
    echo "  This tag is not usable by consumers. Delete it before anyone pins it:"
    echo "      git push --delete origin $value && git tag -d $value"
    echo "  Do not publish a release for it."
  fi
  exit 1
fi

echo
echo "release-check: ok -- $url ($key = $value) loads and enforces the policy"
