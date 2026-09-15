#!/usr/bin/env python3
"""Render the GitHub Release notes for a tag, from README.md.

Usage: release-notes.py <tag> [readme]   # writes markdown to stdout

The lint table and the two setup snippets are lifted out of README.md rather
than written out again here. A release note that restates the setup in its own
words is a second source of truth that nobody diffs against the first: the day
the `pattern` glob or the `check-cfg` entry changes, the README gets updated and
the release notes quietly keep telling adopters to write the old thing. Taking
the real text means the notes cannot drift, and the extraction is strict --
every anchor below must be found, or this exits non-zero -- so a README
reshuffle fails the release loudly instead of publishing empty sections.

The literal placeholder tag the README is written against is substituted for
the tag being released, so the snippet an adopter copies out of the release is
pinned to that release.
"""

import re
import sys
from pathlib import Path

# The tag README.md's setup snippets are written against. Substituted for the
# tag being released wherever it appears inside an extracted snippet.
PLACEHOLDER_TAG = "v0.1.0"


def die(message: str) -> None:
    print(f"release-notes: {message}", file=sys.stderr)
    print(
        "release-notes: README.md no longer has the shape these notes are "
        "built from; fix the anchors in scripts/release-notes.py",
        file=sys.stderr,
    )
    sys.exit(1)


def section(readme: str, heading: str) -> str:
    """The body of one markdown section, up to the next heading of any level."""
    match = re.search(
        rf"^{re.escape(heading)}[^\n]*\n(.*?)(?=^#{{1,6}} )",
        readme,
        re.MULTILINE | re.DOTALL,
    )
    if not match:
        die(f"no section headed {heading!r}")
    return match.group(1)


def table(body: str, heading: str) -> str:
    """The first contiguous run of table rows in a section."""
    rows = re.search(r"^(\|.*(?:\n\|.*)*)", body.lstrip("\n"), re.MULTILINE)
    if not rows:
        die(f"no markdown table under {heading!r}")
    return rows.group(1)


def fence(body: str, lang: str, heading: str) -> str:
    """The first fenced code block of the given language in a section."""
    match = re.search(rf"^```{lang}\n(.*?)^```", body, re.MULTILINE | re.DOTALL)
    if not match:
        die(f"no ```{lang} block under {heading!r}")
    return match.group(1).rstrip("\n")


def main() -> None:
    if not 2 <= len(sys.argv) <= 3:
        print("usage: release-notes.py <tag> [readme]", file=sys.stderr)
        sys.exit(2)

    tag = sys.argv[1]
    if not tag:
        print("release-notes: empty tag", file=sys.stderr)
        sys.exit(2)

    readme_path = Path(sys.argv[2]) if len(sys.argv) == 3 else (
        Path(__file__).resolve().parent.parent / "README.md"
    )
    try:
        readme = readme_path.read_text(encoding="utf-8")
    except OSError as exc:
        die(f"could not read {readme_path}: {exc}")

    lints_heading = "## The lints"
    lints = table(section(readme, lints_heading), lints_heading)

    setup_heading = "### 1. Workspace `Cargo.toml`"
    manifest = fence(section(readme, setup_heading), "toml", setup_heading)

    ci_heading = "### 2. CI"
    ci = fence(section(readme, ci_heading), "yaml", ci_heading)

    # Only the snippets are rewritten. The lint table carries no version.
    if PLACEHOLDER_TAG not in manifest or PLACEHOLDER_TAG not in ci:
        die(
            f"a setup snippet does not mention {PLACEHOLDER_TAG}, so there is "
            "nothing to substitute the released tag for"
        )
    manifest = manifest.replace(PLACEHOLDER_TAG, tag)
    ci = ci.replace(PLACEHOLDER_TAG, tag)

    print(
        f"""## Lints

All three are **Deny** by default and live in the `hyp_hegel` library.

{lints}

## Setup

Workspace `Cargo.toml`:

```toml
{manifest}
```

CI:

```yaml
{ci}
```

Pin the action ref and the `tag` to the same release. They are two independent
versions of one integration, nothing else ties them together, and the action's
`verify-pin` step exists to fail when they drift. There is no moving `v0` tag.

The first run on a new tag compiles a nightly `rustc_private` cdylib from
source -- minutes, not seconds. It is cached per tag thereafter.

## Validation

This tag was validated before these notes were published: the full CI suite ran
against the tagged commit, and `scripts/release-check.sh` then built a
throwaway consumer workspace pinning
`{{ git = "...", tag = "{tag}", pattern = "lints/*" }}` and asserted that the
lints load and fire through that path -- the real `git`/`tag` resolution
adopters depend on, which pull-request CI cannot reach because the tag does not
exist yet.

See the [README at this tag](https://github.com/hypotheosis/hyp-rust-policy-lints/blob/{tag}/README.md)
for exemptions, `--all-targets`, and the rest of what will bite you."""
    )


if __name__ == "__main__":
    main()
