# Releases are cut from this branch and nowhere else.
release_branch := "main"

# Set to "false" to release without a green CI run for the exact commit.
require_ci := "true"

default:
    @just --list --unsorted

# ---------------------------------------------------------------- development

# Everything CI gates on, in the order CI runs it.
check: lint test e2e

# rustfmt and clippy, with the flags the `lints` CI job uses.
lint:
    cd lints && cargo fmt --check
    cd lints && cargo clippy --all-targets -- -D warnings

# Format in place.
fmt:
    cd lints && cargo fmt

# UI fixtures plus the declare_lint! doctests.
test:
    # Bare `cargo test`, never `--test ui`: the latter skips the doctests, and
    # those are what catch a ```rust fence that should have been ```rust,ignore.
    cd lints && cargo test

# Real cargo dylint over fixtures/consumer.
e2e:
    # Asserts the exact lint set per package, and first asserts the library
    # actually loaded -- one that fails to load produces a clean run that is
    # indistinguishable from a passing one.
    ./scripts/e2e.sh

# Prove the git pin works for an existing ref: just release-check rev <sha>
release-check key value:
    ./scripts/release-check.sh https://github.com/hypotheosis/hyp-rust-policy-lints {{key}} {{value}}

# -------------------------------------------------------------------- release

# Refuse to release anything that is not verifiably origin/<release_branch>.
release-preflight:
    #!/usr/bin/env bash
    set -euo pipefail

    # A tag here is the entire product: `tag = "vX.Y.Z"` selects the lint
    # library and `uses: ...@vX.Y.Z` selects the action, so pushing one ships
    # both to everyone at once. A tag pointing at a commit nobody else can
    # fetch -- an uncommitted edit, an unpushed commit, a stale local branch --
    # cannot be repaired by fixing the tree afterwards. It has to be deleted.

    branch="{{release_branch}}"

    current="$(git rev-parse --abbrev-ref HEAD)"
    if [ "$current" != "$branch" ]; then
      echo "release-preflight: on '$current', expected '$branch'." >&2
      echo "  Consumers pin tags, not branches. A tag cut here would point at a" >&2
      echo "  commit that is not on the release branch." >&2
      exit 1
    fi

    if [ -n "$(git status --porcelain)" ]; then
      echo "release-preflight: working tree is not clean:" >&2
      git status --short >&2
      echo "  The tag would be cut from a commit without these changes in it." >&2
      exit 1
    fi

    git fetch --quiet origin "$branch"

    local_sha="$(git rev-parse HEAD)"
    remote_sha="$(git rev-parse "origin/$branch")"
    if [ "$local_sha" != "$remote_sha" ]; then
      counts="$(git rev-list --left-right --count "HEAD...origin/$branch")"
      ahead="$(echo "$counts" | cut -f1)"
      behind="$(echo "$counts" | cut -f2)"
      echo "release-preflight: HEAD is not origin/$branch ($ahead ahead, $behind behind)." >&2
      echo "  local:  $local_sha" >&2
      echo "  remote: $remote_sha" >&2
      [ "$behind" -gt 0 ] && echo "  Pull first: releasing from behind drops whatever landed since." >&2
      [ "$ahead" -gt 0 ] && echo "  Push first: a tag on an unpushed commit resolves to nothing." >&2
      exit 1
    fi

    if [ "{{require_ci}}" = "true" ]; then
      conclusion="$(gh run list --branch "$branch" --commit "$local_sha" \
        --limit 1 --json conclusion --jq '.[0].conclusion' 2>/dev/null || true)"
      case "$conclusion" in
        success) ;;
        "")
          echo "release-preflight: no completed CI run for $local_sha." >&2
          echo "  Either none was triggered, or one is still in flight." >&2
          echo "  Re-run when it finishes, or pass require_ci=false to override." >&2
          exit 1 ;;
        *)
          echo "release-preflight: CI for $local_sha concluded '$conclusion'." >&2
          echo "  Pass require_ci=false to override." >&2
          exit 1 ;;
      esac
      echo "release-preflight: CI green for $local_sha"
    fi

    echo "release-preflight: ok -- $branch at $local_sha, clean and pushed"

# Bump, tag and push. LEVEL is patch | minor | major.
release level: release-preflight check
    #!/usr/bin/env bash
    set -euo pipefail

    # Local checks run first because a bad tag is public the moment it is
    # pushed, and the only remedy release.yml can offer is "delete it and cut
    # another". release.yml re-runs all of this on the runner regardless; doing
    # it here just means failing costs nothing.
    #
    # --no-publish: nothing goes to crates.io. Every package is
    #   `publish = false`; consumers get the library by git tag.
    # --no-verify: the packaging check builds each crate in isolation, which for
    #   a rustc_private cdylib means a full clippy_utils build against
    #   rustc-dev. `just check` already built and tested the real thing.
    cd lints && cargo release --no-publish --no-verify "{{level}}" --execute

    version="$(cargo metadata --no-deps --format-version 1 \
      --manifest-path lints/hyp_hegel/Cargo.toml \
      | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"]=="hyp_hegel"))')"

    echo
    echo "Pushed v${version}. release.yml is validating it now:"
    echo "  https://github.com/hypotheosis/hyp-rust-policy-lints/actions/workflows/release.yml"
    echo
    echo "If that run fails, the tag is public but unusable. Delete it before"
    echo "anyone pins it:"
    echo "  git push --delete origin v${version} && git tag -d v${version}"

# Bump the patch version, tag and push.
release-patch: (release "patch")

# Bump the minor version, tag and push.
release-minor: (release "minor")

# Bump the major version, tag and push.
release-major: (release "major")
