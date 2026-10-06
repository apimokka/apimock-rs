#!/usr/bin/env bash
# Wait for one or more npm packages to become resolvable at a specific
# version before proceeding. `npm publish` can return success before
# the registry's own index has propagated the new version everywhere a
# subsequent `npm view`/`npm install` looks - npm says so itself, in
# the publish output: "Your package is being processed and may take a
# few minutes to become available." This polls until every named
# package answers at the target version, or fails naming whichever
# package(s) never did.
#
# Usage: wait-for-npm-propagation.sh <version> <package>...
#   NPM_PROPAGATION_BUDGET_SECONDS - override the total budget (testing)
#
# ## Why the budget is 10 minutes, and adaptive
#
# This used to be six attempts 15s apart - a flat 90s. That budget lost
# the race twice: 6.0.0 and 6.2.0 both left a partially published
# release, each repaired afterwards with `gh run rerun --failed`. At
# 6.2.1 it won, but only at attempt 4 of 6, and the core package then
# took a further ~2.5 minutes to appear - a delay that would have
# failed this script had it been the thing being waited on.
#
# A flat budget has to choose between the common case and the tail. So
# this backs off instead: 5s, 10s, 20s, then 30s intervals to a 600s
# total. The first re-check is *sooner* than the old 15s, so the fast
# path got faster, while the tail stopped being a coin flip.
#
# The 90s figure was previously described here as a shared convention
# with release-publish.yaml's verify-published / verify-crates-io jobs.
# That is no longer true of this script. Those are post-publish
# *verification* waits - a different job, deliberately left alone
# rather than changed in passing; ROADMAP carries the note that the
# core npm package is not verified at all.
#
# ## Scope
#
# RFC 066 § 2 reserves the publish path to the owner, and RFC 066
# Amendment 2 narrows that to exclude "a wait, retry or poll that
# changes nothing about what publishes, in what order, or with what
# credentials". This script is exactly that carve-out: it changes only
# how long we are willing to wait.
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

BUDGET="${NPM_PROPAGATION_BUDGET_SECONDS:-600}"
INTERVAL=5
MAX_INTERVAL=30

START=$(date +%s)
attempt=0

while :; do
    attempt=$((attempt + 1))

    MISSING=()
    for pkg in "${PACKAGES[@]}"; do
        if ! npm view "${pkg}@${VERSION}" version > /dev/null 2>&1; then
            MISSING+=("$pkg")
        fi
    done

    ELAPSED=$(( $(date +%s) - START ))

    if [ "${#MISSING[@]}" -eq 0 ]; then
        echo "All packages resolved at ${VERSION} after ${ELAPSED}s (${attempt} attempt(s)): ${PACKAGES[*]}"
        exit 0
    fi

    # Would the next sleep take us past the budget? Then this was the
    # last attempt, and we report against the budget we actually spent.
    if [ "$(( ELAPSED + INTERVAL ))" -ge "$BUDGET" ]; then
        echo "::error::package(s) never resolved at ${VERSION} within ${BUDGET}s (${attempt} attempts, ${ELAPSED}s elapsed): ${MISSING[*]}"
        echo "::error::npm may still be processing the publish. Check the registry directly before re-cutting anything; if the package has since landed, re-run the failed jobs rather than tagging again."
        exit 1
    fi

    echo "attempt ${attempt} (${ELAPSED}s elapsed, budget ${BUDGET}s): still waiting on: ${MISSING[*]}, retrying in ${INTERVAL}s..."
    sleep "$INTERVAL"

    if [ "$INTERVAL" -lt "$MAX_INTERVAL" ]; then
        INTERVAL=$(( INTERVAL * 2 ))
        [ "$INTERVAL" -gt "$MAX_INTERVAL" ] && INTERVAL="$MAX_INTERVAL"
    fi
done
