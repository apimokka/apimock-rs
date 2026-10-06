#!/usr/bin/env bash
# Assert that a release's CHANGELOG section is *true* for the claims that
# have a machine-readable referent (RFC 084). Reads everything from the
# tagged commit, so a run against any tag reproduces what the release
# job saw.
#
# Usage: assert-changelog-claims.sh <version>
#   The tag <version> must exist locally (fetch tags, and history for the
#   previous tag: actions/checkout with fetch-depth: 0 and fetch-tags).
#   REPO defaults to $GITHUB_REPOSITORY, then apimokka/apimock-rs.
#
# Claims checked (and ONLY these; prose is never judged, RFC 084 § 4):
#   (a) an API claim matches the public-api baselines:
#       - a section asserting "byte-identical" / "no public API change" /
#         "no API change" requires an empty baseline diff against the
#         previous tag, and the baselines must exist at BOTH tags;
#       - baselines that moved between the two tags require a
#         ### Added / ### Changed / ### Removed heading or a migration
#         guide link in the section.
#   (b) every advisory identifier resolves; a sentence claiming versions
#       are unaffected requires a cited advisory record in the section.
#   (c) a "<crate> X → Y" (or "->") claim naming a crate in Cargo.lock has
#       Y equal to a version that lockfile resolves.
#
# Policy (RFC 084 § 2, § 3):
#   - Previous tag = the greatest semver tag strictly lower than <version>.
#     Chosen over "the tag before it in history": it is stable under
#     backports (a 6.1.1 released after 6.2.0 does not change 6.2.1's
#     predecessor). Missing or unreachable -> failure, never a skip.
#   - Network errors fail CLOSED. Only a definitive "not found" (HTTP 404,
#     or an empty CVE search) counts as an unresolvable identifier.
#   - "Cites evidence" for an unaffectedness sentence means: the sentence
#     names a version or advisory ID, AND the section contains at least one
#     link to an advisory record (rustsec.org or a GitHub security advisory).
#     The job cannot judge the analysis; it requires the claim to point at
#     the record where the analysis lives.
#
# Output: one PASS/FAIL line per assertion. With SUMMARY_FILE set, also
# writes the Tier and claims summary (RFC 081 § 2, RFC 084 § 5) to that file.

set -euo pipefail

VERSION="${1:?usage: assert-changelog-claims.sh <version>}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXTRACT="$SCRIPT_DIR/extract-changelog-section.sh"
REPO="${GITHUB_REPOSITORY:-${REPO:-apimokka/apimock-rs}}"
RESULTS_TMP=""

FAILURES=0
TMP="$(mktemp -d)"
RESULTS_TMP="$TMP/results.tsv"
: > "$RESULTS_TMP"
trap 'rm -rf "$TMP"' EXIT

record() { # record <PASS|FAIL|NOTE> <claim> <detail>
    local status="$1" claim="$2" detail="$3"
    printf '%-5s %-28s %s\n' "$status" "$claim" "$detail"
    printf '%s\t%s\t%s\n' "$status" "$claim" "$detail" >> "$RESULTS_TMP"
    if [ "$status" = "FAIL" ]; then
        FAILURES=$((FAILURES + 1))
    fi
}

fail_closed() { # a network or API failure: never a skip
    record FAIL "$1" "$2 (failing closed: this is not a pass)"
}

# ── Inputs ──────────────────────────────────────────────────────────────

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "::error::'$VERSION' is not a plain X.Y.Z version" >&2
    exit 1
fi

if ! TAG_COMMIT=$(git rev-parse -q --verify "refs/tags/$VERSION^{commit}"); then
    echo "::error::tag $VERSION not found locally - fetch tags (fetch-tags: true)" >&2
    exit 1
fi

# Previous tag: greatest semver tag strictly lower than VERSION.
PREV=""
while IFS= read -r candidate; do
    [ "$candidate" = "$VERSION" ] && continue
    lower=$(printf '%s\n%s\n' "$candidate" "$VERSION" | sort -V | head -1)
    if [ "$lower" = "$candidate" ]; then
        PREV="$candidate"
    fi
done < <(git tag --list | grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' | sort -V)

if [ -z "$PREV" ]; then
    echo "::error::no semver tag lower than $VERSION is visible - either none exists, or tags/history were not fetched (fetch-depth: 0, fetch-tags: true)" >&2
    exit 1
fi
if ! PREV_COMMIT=$(git rev-parse -q --verify "refs/tags/$PREV^{commit}") \
    || ! git cat-file -e "$PREV_COMMIT^{commit}" 2>/dev/null; then
    echo "::error::previous tag $PREV is not reachable - fetch history (fetch-depth: 0)" >&2
    exit 1
fi

echo "Asserting CHANGELOG claims for $VERSION (tag $(git rev-parse --short "$TAG_COMMIT")), previous tag $PREV"
echo

git show "$TAG_COMMIT:CHANGELOG.md" > "$TMP/CHANGELOG.md"
bash "$EXTRACT" "$VERSION" "$TMP/CHANGELOG.md" > "$TMP/section.md"
git show "$TAG_COMMIT:Cargo.lock" > "$TMP/Cargo.lock"
SECTION="$TMP/section.md"

# ── (a) API claim vs baselines ──────────────────────────────────────────

# Every workspace crate that carries a manifest at the tag must have a baseline.
CRATES=$(git ls-tree -d --name-only "$TAG_COMMIT" crates/ | sed 's|^crates/||; s|/$||' | sort)
for c in $CRATES; do
    git cat-file -e "$TAG_COMMIT:crates/$c/Cargo.toml" 2>/dev/null || continue
    for ref in "$PREV_COMMIT" "$TAG_COMMIT"; do
        label=$([ "$ref" = "$TAG_COMMIT" ] && echo "$VERSION" || echo "$PREV")
        if ! git cat-file -e "$ref:crates/$c/public-api.txt" 2>/dev/null; then
            BASELINE_MISSING="${BASELINE_MISSING:-} $c@$label"
        fi
    done
done

NO_API_PHRASE=$(grep -i -E 'byte-identical|no public API change|no API change' "$SECTION" || true)
if [ -n "$NO_API_PHRASE" ]; then
    if [ -n "${BASELINE_MISSING:-}" ]; then
        record FAIL "api-claim" "section asserts no API change, but baselines are absent:${BASELINE_MISSING}. Absence is not a pass (RFC 081 Amendment 1 A)."
    else
        DIFF=$(git diff "$PREV_COMMIT" "$TAG_COMMIT" -- 'crates/*/public-api.txt')
        if [ -n "$DIFF" ]; then
            record FAIL "api-claim" "section asserts no API change, but $PREV..$VERSION moved the baselines:"
            printf '%s\n' "$DIFF" | head -40
        else
            record PASS "api-claim" "section asserts no API change; baselines byte-identical $PREV..$VERSION, present at both"
        fi
    fi
fi

MOVED=$(git diff --name-only "$PREV_COMMIT" "$TAG_COMMIT" -- 'crates/*/public-api.txt')
if [ -n "$MOVED" ]; then
    DECLARED=$(grep -E '^### (Added|Changed|Removed)$' "$SECTION" || true)
    MIGRATION=$(grep -E 'migrating-to-' "$SECTION" || true)
    if [ -n "$DECLARED" ] || [ -n "$MIGRATION" ]; then
        record PASS "api-declared" "baselines moved $PREV..$VERSION and the section declares it (Added/Changed/Removed or a migration link)"
    else
        record FAIL "api-declared" "baselines moved $PREV..$VERSION but the section has no ### Added/Changed/Removed heading and no migration-guide link"
        printf '%s\n' "$MOVED" | sed 's/^/        moved: /'
    fi
else
    record PASS "api-declared" "baselines unchanged $PREV..$VERSION (or absent at both)"
fi

# ── (b) advisory identifiers, and unaffectedness claims ─────────────────

IDS=$(grep -oE 'RUSTSEC-[0-9]{4}-[0-9]{4}|GHSA-[a-z0-9-]+|CVE-[0-9]{4}-[0-9]+' "$SECTION" \
    | sed -E 's/-+$//' | sort -u || true)

resolve_rustsec() {
    local id="$1" code
    code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 30 --retry 2 \
        "https://rustsec.org/advisories/$id") || { echo "unreachable"; return; }
    case "$code" in
        200) echo "resolved" ;;
        404) echo "not-found" ;;
        *)   echo "unreachable:$code" ;;
    esac
}

auth_header=()
if [ -n "${GITHUB_TOKEN:-}" ]; then
    auth_header=(-H "Authorization: Bearer $GITHUB_TOKEN")
fi

resolve_github() { # resolve_github <GHSA-…|CVE-…>
    local id="$1" code body
    case "$id" in
        GHSA-*)
            code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 30 --retry 2 \
                -H "Accept: application/vnd.github+json" "${auth_header[@]}" \
                "https://api.github.com/advisories/$id") || { echo "unreachable"; return; }
            [ "$code" = "200" ] && { echo "resolved"; return; }
            [ "$code" = "404" ] || { echo "unreachable:$code"; return; }
            # Repository-scoped advisories (not in the global database) live
            # under the repo. 6.0.0's GHSA-72g6-wgrg-vhm7 resolves only here.
            code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 30 --retry 2 \
                -H "Accept: application/vnd.github+json" "${auth_header[@]}" \
                "https://api.github.com/repos/$REPO/security-advisories/$id") || { echo "unreachable"; return; }
            case "$code" in
                200) echo "resolved-repo" ;;
                404) echo "not-found" ;;
                *)   echo "unreachable:$code" ;;
            esac
            ;;
        CVE-*)
            body=$(curl -s --max-time 30 --retry 2 \
                -H "Accept: application/vnd.github+json" "${auth_header[@]}" \
                -w '\n%{http_code}' "https://api.github.com/advisories?cve_id=$id") \
                || { echo "unreachable"; return; }
            code="${body##*$'\n'}"
            body="${body%$'\n'*}"
            [ "$code" = "200" ] || { echo "unreachable:$code"; return; }
            if [ "$(printf '%s' "$body" | tr -d '[:space:]')" = "[]" ]; then
                echo "not-found"
            else
                echo "resolved"
            fi
            ;;
    esac
}

for id in $IDS; do
    case "$id" in
        RUSTSEC-*) outcome=$(resolve_rustsec "$id") ;;
        *)         outcome=$(resolve_github "$id") ;;
    esac
    case "$outcome" in
        resolved)      record PASS "advisory-id" "$id resolves (global advisory database)" ;;
        resolved-repo) record PASS "advisory-id" "$id resolves (repository security advisory)" ;;
        not-found)     record FAIL "advisory-id" "$id does not resolve (not found)" ;;
        unreachable*)  fail_closed "advisory-id" "$id: could not verify ($outcome)" ;;
    esac
done
if [ -z "$IDS" ]; then
    record NOTE "advisory-id" "no advisory identifiers in the section"
fi

# Paragraphs: a new one starts at a blank line, a bullet, or a heading.
# Sentences: split at ". ", "! ", "? " within a paragraph.
awk '
    /^[[:space:]]*$/ || /^[[:space:]]*- / || /^#/ { if (buf != "") print buf; buf = ""; if (/^[[:space:]]*- /) buf = $0; next }
    { buf = (buf == "" ? $0 : buf " " $0) }
    END { if (buf != "") print buf }
' "$SECTION" | sed -E 's/([.!?])[[:space:]]+/\1\n/g' > "$TMP/sentences.txt"

UNAFF_CLAIMS=0
while IFS= read -r sentence; do
    if printf '%s' "$sentence" | grep -qi -E 'unaffected|not affected'; then
        if printf '%s' "$sentence" | grep -qE '[0-9]+\.[0-9]+|RUSTSEC-|GHSA-|CVE-'; then
            UNAFF_CLAIMS=$((UNAFF_CLAIMS + 1))
            record NOTE "unaffected-claim" "found: $(printf '%s' "$sentence" | cut -c1-110)"
        fi
    fi
done < "$TMP/sentences.txt"

if [ "$UNAFF_CLAIMS" -gt 0 ]; then
    if grep -qE 'rustsec\.org/advisories/|github\.com/[^/]+/[^/]+/security/advisories/' "$SECTION"; then
        record PASS "unaffected-evidence" "$UNAFF_CLAIMS unaffectedness claim(s) cite an advisory record in the section"
    else
        record FAIL "unaffected-evidence" "$UNAFF_CLAIMS unaffectedness claim(s) about versions with no advisory record cited in the section"
    fi
fi

# ── (c) dependency version claims vs the lockfile ───────────────────────

# shellcheck disable=SC2016  # the backticks are literal Markdown, not command substitution
ARROW_CLAIMS=$(grep -oE '`?[A-Za-z][A-Za-z0-9_-]*`?( (moved|bumped|updated|upgraded|raised|lowered))? [0-9]+\.[0-9]+\.[0-9]+ (→|->) [0-9]+\.[0-9]+\.[0-9]+' "$SECTION" || true)
DEP_CHECKED=0
while IFS= read -r claim; do
    [ -z "$claim" ] && continue
    # shellcheck disable=SC2016  # literal Markdown backtick in the pattern
    name=$(printf '%s' "$claim" | sed -E 's/^`?([A-Za-z][A-Za-z0-9_-]*)`?.*/\1/')
    target=$(printf '%s' "$claim" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | tail -1)
    lock_versions=$(awk -v n="$name" '
        $0 == "name = \"" n "\"" { getline; if ($1 == "version") { gsub(/"/, "", $3); print $3 } }
    ' "$TMP/Cargo.lock")
    if [ -z "$lock_versions" ]; then
        record NOTE "dependency-claim" "'$claim': '$name' is not in Cargo.lock; not a crate claim, not checked"
        continue
    fi
    DEP_CHECKED=$((DEP_CHECKED + 1))
    if printf '%s\n' "$lock_versions" | grep -qx "$target"; then
        record PASS "dependency-claim" "$name -> $target matches Cargo.lock ($(printf '%s' "$lock_versions" | tr '\n' ' ' | sed 's/ $//'))"
    else
        record FAIL "dependency-claim" "$name -> $target, but Cargo.lock resolves $(printf '%s' "$lock_versions" | tr '\n' ' ' | sed 's/ $//')"
    fi
done <<< "$ARROW_CLAIMS"

# ── Tier summary (RFC 081 § 2 conditions; RFC 084 § 5) ──────────────────
# Reports the four tier conditions and the claim results. It states what was
# enforced and what remains trusted prose, so a summary cannot read as more
# verified than it is. Written to SUMMARY_FILE when set; never changes the
# exit status.

if [ -n "${SUMMARY_FILE:-}" ]; then
    MAJOR_NOW="${VERSION%%.*}"
    MAJOR_PREV="${PREV%%.*}"
    if grep -qE '^### Security$' "$SECTION"; then
        C1="FAILS - the section has a ### Security heading"
    else
        C1="holds - no ### Security heading"
    fi
    if [ -n "${BASELINE_MISSING:-}" ]; then
        C2="NOT APPLICABLE (baselines absent:${BASELINE_MISSING}) - resolves to Tier B"
    elif [ -n "$MOVED" ]; then
        C2="FAILS - baselines moved $PREV..$VERSION"
    else
        C2="holds - baselines unchanged $PREV..$VERSION and present at both"
    fi
    C3="NOT MACHINE-CHECKED - a judgement on whether any default newly refuses a previously-accepted request; the architect decides"
    if [ "$MAJOR_NOW" = "$MAJOR_PREV" ]; then
        C4="holds - major $MAJOR_NOW unchanged from $PREV"
    else
        C4="FAILS - major changed from $MAJOR_PREV to $MAJOR_NOW"
    fi
    case "$C1$C2$C4" in
        *FAILS*|*NOT\ APPLICABLE*) TIER="Tier B" ;;
        *) TIER="Tier A candidate - condition 3 still needs the architect's judgement, so CI cannot confirm Tier A" ;;
    esac
    {
        echo "## Tier and CHANGELOG claims for $VERSION"
        echo
        echo "**Tier:** $TIER"
        echo
        echo "### RFC 081 § 2 conditions"
        echo
        echo "1. No \`### Security\` section - $C1"
        echo "2. No public-api baseline changed, and the baselines exist at both tags - $C2"
        echo "3. $C3"
        echo "4. Major component unchanged - $C4"
        echo
        echo "### Claims asserted by CI (RFC 084 § 3), compared against $PREV"
        echo
        echo "| result | claim | detail |"
        echo "|---|---|---|"
        while IFS=$'\t' read -r status claim detail; do
            echo "| $status | $claim | $(printf '%s' "$detail" | tr '|' '/') |"
        done < "$RESULTS_TMP"
        echo
        echo "### Not judged by CI"
        echo
        echo "Every other sentence in the section: whether a feature description is fair, the wording,"
        echo "the length, and the third tier condition above. Those stay human (RFC 084 § 4)."
        if [ -n "${GITHUB_RUN_ID:-}" ]; then
            echo
            echo "Build-phase run: ${GITHUB_SERVER_URL:-https://github.com}/${GITHUB_REPOSITORY:-$REPO}/actions/runs/$GITHUB_RUN_ID"
        fi
    } > "$SUMMARY_FILE"
fi

# ── Result ──────────────────────────────────────────────────────────────

echo
if [ "$FAILURES" -gt 0 ]; then
    echo "::error::$FAILURES CHANGELOG claim(s) for $VERSION failed the assertions above. Correct the CHANGELOG entry and re-tag."
    exit 1
fi
echo "All machine-checkable CHANGELOG claims for $VERSION hold. Prose was not judged (RFC 084 § 4)."
