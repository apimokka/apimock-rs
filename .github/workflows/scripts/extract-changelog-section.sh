#!/usr/bin/env bash
# Extract one version's section from CHANGELOG.md for use as release notes.
# Usage: extract-changelog-section.sh <version> [changelog-path]
#
# Prints the section body (everything between "## [<version>] - ..." and
# the next "## [" heading, or end of file) to stdout, with leading and
# trailing blank lines trimmed. Exits non-zero with no stdout output if
# the version has no section - callers must treat that as a hard failure,
# not fall back to empty notes.

set -eu

VERSION="${1:?usage: extract-changelog-section.sh <version> [changelog-path]}"
CHANGELOG="${2:-CHANGELOG.md}"

if [ ! -f "$CHANGELOG" ]; then
    echo "::error::$CHANGELOG not found" >&2
    exit 1
fi

SECTION=$(awk -v version="$VERSION" '
    /^## \[/ {
        if (in_section) exit
        if ($0 == "## [" version "]" || index($0, "## [" version "] ") == 1) {
            in_section = 1
        }
        next
    }
    in_section { lines[++n] = $0 }
    END {
        first = 1
        while (first <= n && lines[first] ~ /^[[:space:]]*$/) first++
        last = n
        while (last >= first && lines[last] ~ /^[[:space:]]*$/) last--
        for (i = first; i <= last; i++) print lines[i]
    }
' "$CHANGELOG")

if [ -z "$SECTION" ]; then
    echo "::error::CHANGELOG.md has no section for version $VERSION (expected a heading like '## [$VERSION] - YYYY-MM-DD')" >&2
    exit 1
fi

printf '%s\n' "$SECTION"
