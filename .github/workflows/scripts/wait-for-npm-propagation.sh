#!/usr/bin/env bash
# Wait for one or more npm packages to become resolvable at a specific
# version before proceeding. `npm publish` can return success before
# the registry's own index has propagated the new version everywhere a
# subsequent `npm view`/`npm install` looks - this polls until every
# named package answers at the target version, or fails naming
# whichever package(s) never did.
#
# Usage: wait-for-npm-propagation.sh <version> <package>...
#
# Six attempts, 15s apart (90s total) - the same budget
# release-publish.yaml's verify-published/verify-crates-io jobs already
# use for the same kind of registry-propagation wait, so there is one
# convention in this project for "how long do we wait for a registry",
# not two.
#
# Extracted into its own script, rather than left inline in the
# workflow, specifically so it can be run directly against the real
# npm registry - a wait that never waits is untested, and
# release-publish.yaml itself only ever runs on a real Release being
# published.

set -e

if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <version> <package>..." >&2
    exit 1
fi

VERSION="$1"
shift
PACKAGES=("$@")

for attempt in 1 2 3 4 5 6; do
    MISSING=()
    for pkg in "${PACKAGES[@]}"; do
        if ! npm view "${pkg}@${VERSION}" version > /dev/null 2>&1; then
            MISSING+=("$pkg")
        fi
    done

    if [ "${#MISSING[@]}" -eq 0 ]; then
        echo "All packages resolved at ${VERSION}: ${PACKAGES[*]}"
        exit 0
    fi

    if [ "$attempt" -eq 6 ]; then
        echo "::error::package(s) never resolved at ${VERSION} after 6 attempts over 90s: ${MISSING[*]}"
        exit 1
    fi

    echo "attempt ${attempt}: still waiting on: ${MISSING[*]}, retrying in 15s..."
    sleep 15
done
