#!/usr/bin/env bash
# Verify that the consuming workspace pins the same tag the action was called
# at.
#
# Two versions are in play on every consumer run and nothing else ties them
# together: the ref in `uses: hypotheosis/hyp-rust-policy-lints@<ref>`, which
# decides the action's behaviour, and the `tag` in the workspace's
# `workspace.metadata.dylint.libraries` entry, which decides which lint library
# `cargo dylint` actually downloads and runs. Bump one and forget the other and
# CI keeps passing while enforcing a policy nobody chose. The failure is silent,
# which is why it is worth a script.
#
# Usage: verify-pin.sh <Cargo.toml> <expected-ref>
#
# Requires python3 >= 3.11 (for `tomllib`). Parsing TOML with grep loses to the
# first manifest that spreads a libraries entry over several lines or mentions
# the repository in a comment, and this check exists precisely to be trusted; a
# parser that can be fooled is worse than no check. Runners without python3 can
# set `verify-pin: false`.
set -euo pipefail

# Matched as a substring of the `git` URL, so it covers https, ssh and a
# trailing `.git` alike.
REPO_SLUG_FRAGMENT="hyp-rust-policy-lints"

if [ "$#" -ne 2 ]; then
  echo "usage: verify-pin.sh <Cargo.toml> <expected-ref>" >&2
  exit 2
fi

manifest=$1
expected=$2

if [ ! -f "$manifest" ]; then
  echo "verify-pin: no manifest at $manifest" >&2
  echo "verify-pin: is 'working-directory' pointing at the workspace root?" >&2
  exit 1
fi

# An empty expected ref means `github.action_ref` was empty, which happens when
# the action runs from a local path (`uses: ./`) rather than a released ref.
# There is then no action version to compare against, so the check cannot be
# performed -- and passing anyway would defeat the whole point: an unverifiable
# pin would report the same green tick as a verified one. Fail, and name the
# escape hatch.
if [ -z "$expected" ]; then
  cat >&2 <<'EOF'
verify-pin: cannot tell which version of this action is running.
  github.action_ref was empty, which is what happens when the action is used
  from a local checkout (uses: ./) instead of a released ref. There is no
  version to compare the workspace's pin against, so nothing was verified.
  Set verify-pin: false for local-checkout runs.
EOF
  exit 1
fi

python=""
for candidate in python3 python; do
  if command -v "$candidate" >/dev/null 2>&1 \
     && "$candidate" -c 'import tomllib' >/dev/null 2>&1; then
    python=$candidate
    break
  fi
done

if [ -z "$python" ]; then
  echo "verify-pin: needs python3 >= 3.11 (for tomllib) to parse $manifest" >&2
  echo "verify-pin: install python3 on the runner, or set verify-pin: false" >&2
  exit 1
fi

"$python" - "$manifest" "$expected" "$REPO_SLUG_FRAGMENT" <<'PY'
import sys
import tomllib

manifest_path, expected, slug = sys.argv[1:4]


def normalize(ref):
    # `github.action_ref` is a bare ref ("v0.1.0"), but a manifest may have been
    # written with the fully qualified form. Compare like with like.
    return ref[len("refs/tags/"):] if ref.startswith("refs/tags/") else ref


def die(*lines):
    for line in lines:
        print(line, file=sys.stderr)
    sys.exit(1)


try:
    with open(manifest_path, "rb") as fh:
        doc = tomllib.load(fh)
except (OSError, tomllib.TOMLDecodeError) as exc:
    die(f"verify-pin: could not parse {manifest_path}: {exc}")


def dig(root, *keys):
    node = root
    for key in keys:
        if not isinstance(node, dict):
            return None
        node = node.get(key)
    return node


# cargo-dylint 6.0.4 reads `workspace.metadata.dylint.libraries` and nothing
# else. `package.metadata.dylint` is not a supported spelling -- it is not a
# fallback, it is ignored, and the run that follows prints
# `Warning: No libraries were found.` and exits 0 having checked nothing.
#
# So that spelling must be an error here, not something to read anyway. A
# manifest that pins the library where dylint will never look enforces no
# policy at all, and reporting `verify-pin: ok` for it would be worse than not
# checking: it would turn "silently unenforced" into "actively confirmed
# correct". Same failure shape as the `cargo dylint list` precondition in
# scripts/e2e.sh, and there for the same reason.
libraries = dig(doc, "workspace", "metadata", "dylint", "libraries")

if libraries is None:
    if dig(doc, "package", "metadata", "dylint", "libraries") is not None:
        die(
            f"verify-pin: {manifest_path} pins the lint library under "
            "package.metadata.dylint",
            "  cargo-dylint reads workspace.metadata.dylint only. This entry "
            "is ignored: the run",
            "  would print 'Warning: No libraries were found.' and pass "
            "without checking anything.",
            "  Add an explicit [workspace] table -- a single-package "
            "repository can leave it empty --",
            "  and move the entry to [workspace.metadata.dylint].",
        )

    die(
        f"verify-pin: {manifest_path} has no "
        "workspace.metadata.dylint.libraries",
        "  cargo dylint would find no libraries here and pass without "
        "checking anything.",
    )

# dylint accepts a single table as well as an array of them.
if isinstance(libraries, dict):
    libraries = [libraries]
if not isinstance(libraries, list):
    die(f"verify-pin: libraries in {manifest_path} is not a table or array")

ours = [
    entry
    for entry in libraries
    if isinstance(entry, dict) and slug in str(entry.get("git", ""))
]

if not ours:
    pins = ", ".join(sorted({k for e in libraries if isinstance(e, dict)
                             for k in ("git", "path") if k in e})) or "none"
    die(
        f"verify-pin: no libraries entry in {manifest_path} whose git URL "
        f"names {slug}",
        f"  entries found pin by: {pins}",
        f'  expected an entry like {{ git = ".../{slug}", '
        f'tag = "{expected}", pattern = "lints/*" }}',
        "  A path pin cannot be verified against an action ref; use "
        "verify-pin: false for those.",
    )

want = normalize(expected)
mismatched = []
found = []

for entry in ours:
    tag = entry.get("tag")
    if tag is None:
        other = ", ".join(k for k in ("branch", "rev") if k in entry) or "nothing"
        die(
            f"verify-pin: the {slug} entry in {manifest_path} has no tag "
            f"(it pins {other})",
            f'  pin it with tag = "{expected}", or set verify-pin: false.',
        )
    found.append(tag)
    if normalize(str(tag)) != want:
        mismatched.append(tag)

if mismatched:
    die(
        "verify-pin: version drift",
        f"  action called at: {expected}",
        f"  {manifest_path} pins: {', '.join(mismatched)}",
        "  Pin both to the same tag, or set verify-pin: false.",
    )

print(f"verify-pin: ok ({', '.join(found)})")
PY
