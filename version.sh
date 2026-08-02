#!/bin/sh
#
# version.sh — Update the version across the Cargo workspace and the npm
# packages in one operation, and verify every target it claims to update.
#
# Targets (RFC 032):
#   1. Root Cargo.toml     [workspace.package].version   (section-aware —
#                          external [workspace.dependencies] version pins,
#                          e.g. tokio/hyper/rustls, are left untouched)
#   2. Root Cargo.toml     [workspace.dependencies] internal-crate pins
#                          (apimock-config/-routing/-server) — major
#                          component kept in step with the new version, so
#                          a major bump doesn't silently break resolution
#   3. Cargo.lock          refreshed via `cargo fetch` and re-verified —
#                          this crate's own four workspace-member entries
#                          must show the new version afterward
#   4. npm/*/package.json  .version
#   5. npm/package.json    .version, and every
#                          .optionalDependencies["@apimock-rs/bin-*"] pin
#
# Member crate manifests (crates/*/Cargo.toml) are NOT touched:
# `version.workspace = true` already inherits from the root correctly —
# rewriting them was always wrong.
#
# No package-lock.json handling: none exists anywhere in this repository
# (the release workflow generates them at publish time, after this script
# has already run) — dead, untestable code was removed rather than kept
# "just in case". If one is ever committed, add support then, with a real
# file to verify against.
#
# Required tools: cargo, jq, awk, grep

set -eu

REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$REPO_ROOT"

ROOT_MANIFEST="Cargo.toml"

# ---------- help ----------
show_help() {
    cat <<EOF
Usage: ${0##*/} [OPTIONS]

Options:
  -l, --list                List the workspace crates' version and every
                             npm package's version.
  -u, --update VERSION      Set the workspace manifest, the internal
                             crate pins' major component, Cargo.lock, and
                             every npm package (including
                             optionalDependencies pins) to VERSION.
  -d, --dry-run             Used with --update: report exactly what
                             would change, without modifying anything.
  -h, --help                Show this help and exit.

Examples:
  ${0##*/} --list
  ${0##*/} --update 1.2.3
  ${0##*/} --update 1.2.3 --dry-run
EOF
    exit 0
}

# ---------- arg parsing ----------
LIST_MODE=0; UPDATE_MODE=0; DRY_RUN=0; NEW_VERSION=; NO_OPTION=1

while [ $# -gt 0 ]; do
    case "$1" in
        -l|--list)    LIST_MODE=1; NO_OPTION=0; shift ;;
        -u|--update)  UPDATE_MODE=1; NO_OPTION=0; NEW_VERSION=$2; shift 2 ;;
        -d|--dry-run) DRY_RUN=1; NO_OPTION=0; shift ;;
        -h|--help)    show_help ;;
        *) printf 'Unknown option: %s\n' "$1" >&2; exit 1 ;;
    esac
done
[ "$NO_OPTION" -eq 1 ] && show_help

# ---------- tool check ----------
for cmd in cargo jq awk grep; do
    command -v "$cmd" >/dev/null 2>&1 || { printf 'Error: %s not found.\n' "$cmd" >&2; exit 1; }
done

# ---------- npm targets (fixed layout, not derived from cargo metadata —
# that derivation is what silently broke this script after the 5.1.1
# workspace split) ----------
NPM_PLATFORM_PACKAGES="npm/darwin-arm64/package.json npm/linux-x64-gnu/package.json npm/win32-x64-msvc/package.json"
NPM_ROOT_PACKAGE="npm/package.json"

# ---------- read the current [workspace.package].version, section-aware ----------
# Only matches a `version = "..."` line while inside the
# [workspace.package] section. A line starting with "version" inside any
# other section (there currently is none, but this must not assume that)
# is never touched.
read_workspace_version() {
    awk '
        /^\[workspace\.package\]/ { in_section=1; next }
        /^\[/                     { in_section=0 }
        in_section && /^[[:space:]]*version[[:space:]]*=/ {
            line=$0
            sub(/^[[:space:]]*version[[:space:]]*=[[:space:]]*"/, "", line)
            sub(/".*$/, "", line)
            print line
            exit
        }
    ' "$ROOT_MANIFEST"
}

# ---------- major component of a semver string ("6.0.0" -> "6") ----------
major_version_of() {
    printf '%s\n' "$1" | awk -F'.' '{ print $1 }'
}

# ---------- read the internal-crate pins' version requirement, section-
# aware: only lines inside [workspace.dependencies] whose value carries a
# `path = "crates/...` (the mark of an internal workspace member, never
# present on an external dependency like tokio/hyper/rustls) are read. ----------
read_internal_crate_pin_versions() {
    awk '
        /^\[workspace\.dependencies\]/ { in_section=1; next }
        /^\[/                          { in_section=0 }
        in_section && /path[[:space:]]*=[[:space:]]*"crates\// {
            line=$0
            match(line, /version[[:space:]]*=[[:space:]]*"[^"]*"/)
            v=substr(line, RSTART, RLENGTH)
            sub(/^version[[:space:]]*=[[:space:]]*"/, "", v)
            sub(/"$/, "", v)
            print v
        }
    ' "$ROOT_MANIFEST"
}

# ---------- FAILED_TARGETS accumulates self-verification failures ----------
FAILED_TARGETS=""

fail_target() {
    printf 'Error: %s was not updated as expected (%s)\n' "$1" "$2" >&2
    FAILED_TARGETS="${FAILED_TARGETS}${FAILED_TARGETS:+, }$1"
}

# ---------- update the root manifest: [workspace.package].version, AND
# the internal-crate pins' major component, in one section-aware pass.
# Every external [workspace.dependencies] version pin (tokio, hyper,
# rustls, ...) is left byte-for-byte unchanged — only lines that are (a)
# inside [workspace.package] and start with `version =`, or (b) inside
# [workspace.dependencies] and carry a `path = "crates/...` value, are
# touched. ----------
#
# Why the internal pins need this too: apimock-config/-routing/-server
# are pinned at a major-only requirement ("5", i.e. Cargo's implicit
# ^5). That's correct and deliberately untouched across minor/patch
# bumps (5.14.0 -> 5.15.0: "5" still matches, nothing to do). Across a
# major bump (5.x -> 6.0.0) it stops matching and `cargo fetch` fails to
# resolve — so the major component must move in step with the workspace
# version, the same "versions that must move together" property this
# script already keeps for npm's optionalDependencies pins.
update_root_manifest_version() {
    ver=$1
    major=$(major_version_of "$ver")

    if [ "$DRY_RUN" -eq 1 ]; then
        printf '  (dry-run) would update %s [workspace.package].version -> %s\n' "$ROOT_MANIFEST" "$ver"
        printf '  (dry-run) would update %s internal crate pins -> "%s"\n' "$ROOT_MANIFEST" "$major"
        return
    fi

    tmp=$(mktemp) || exit 1
    awk -v nv="$ver" -v major="$major" '
        /^\[/ {
            if ($0 ~ /^\[workspace\.package\]/)      { section = "package" }
            else if ($0 ~ /^\[workspace\.dependencies\]/) { section = "deps" }
            else                                     { section = "" }
            print
            next
        }
        section == "package" && !found_version && /^[[:space:]]*version[[:space:]]*=/ {
            print "version = \"" nv "\""
            found_version = 1
            next
        }
        section == "deps" && /path[[:space:]]*=[[:space:]]*"crates\// {
            sub(/version[[:space:]]*=[[:space:]]*"[^"]*"/, "version = \"" major "\"")
            print
            next
        }
        { print }
    ' "$ROOT_MANIFEST" > "$tmp" || { rm -f "$tmp"; fail_target "$ROOT_MANIFEST" "awk write failed"; return; }
    mv "$tmp" "$ROOT_MANIFEST" || { rm -f "$tmp"; fail_target "$ROOT_MANIFEST" "mv failed (permission denied?)"; return; }
    git add "$ROOT_MANIFEST" 2>/dev/null || true
    printf '  updated %s [workspace.package].version -> %s\n' "$ROOT_MANIFEST" "$ver"
    printf '  updated %s internal crate pins -> "%s"\n' "$ROOT_MANIFEST" "$major"

    actual=$(read_workspace_version) || actual="<unreadable>"
    [ "$actual" = "$ver" ] || fail_target "$ROOT_MANIFEST" "expected version \"$ver\", found \"$actual\""

    stale_pins=""
    for pin in $(read_internal_crate_pin_versions); do
        [ "$pin" = "$major" ] || stale_pins="${stale_pins}${stale_pins:+, }$pin"
    done
    [ -z "$stale_pins" ] || fail_target "$ROOT_MANIFEST" "internal crate pin(s) still not \"$major\": $stale_pins"
}

# ---------- update a plain package.json's .version ----------
update_npm_package_version() {
    file_path=$1; ver=$2
    [ ! -f "$file_path" ] && return

    if [ "$DRY_RUN" -eq 1 ]; then
        printf '  (dry-run) would update %s .version -> %s\n' "$file_path" "$ver"
        return
    fi

    tmp=$(mktemp) || exit 1
    jq --arg v "$ver" '.version = $v' "$file_path" > "$tmp" || { rm -f "$tmp"; fail_target "$file_path" "jq write failed"; return; }
    mv "$tmp" "$file_path" || { rm -f "$tmp"; fail_target "$file_path" "mv failed (permission denied?)"; return; }
    git add "$file_path" 2>/dev/null || true
    printf '  updated %s .version -> %s\n' "$file_path" "$ver"

    actual=$(jq -r '.version' "$file_path" 2>/dev/null || echo "<unreadable>")
    [ "$actual" = "$ver" ] || fail_target "$file_path" "expected version \"$ver\", found \"$actual\""
}

# ---------- update npm/package.json: .version AND every
# optionalDependencies["@apimock-rs/bin-*"] pin ----------
update_npm_root_package() {
    file_path=$1; ver=$2
    [ ! -f "$file_path" ] && return

    if [ "$DRY_RUN" -eq 1 ]; then
        printf '  (dry-run) would update %s .version and optionalDependencies["@apimock-rs/bin-*"] -> %s\n' "$file_path" "$ver"
        return
    fi

    tmp=$(mktemp) || exit 1
    jq --arg v "$ver" '
        .version = $v
        | .optionalDependencies |= (
            to_entries
            | map(if (.key | startswith("@apimock-rs/bin-")) then .value = $v else . end)
            | from_entries
        )
    ' "$file_path" > "$tmp" || { rm -f "$tmp"; fail_target "$file_path" "jq write failed"; return; }
    mv "$tmp" "$file_path" || { rm -f "$tmp"; fail_target "$file_path" "mv failed (permission denied?)"; return; }
    git add "$file_path" 2>/dev/null || true
    printf '  updated %s .version and optionalDependencies["@apimock-rs/bin-*"] -> %s\n' "$file_path" "$ver"

    actual_version=$(jq -r '.version' "$file_path" 2>/dev/null || echo "<unreadable>")
    [ "$actual_version" = "$ver" ] || fail_target "$file_path" "expected .version \"$ver\", found \"$actual_version\""

    stale_pins=$(jq -r --arg v "$ver" '
        .optionalDependencies // {}
        | to_entries[]
        | select(.key | startswith("@apimock-rs/bin-"))
        | select(.value != $v)
        | .key
    ' "$file_path" 2>/dev/null || true)
    [ -z "$stale_pins" ] || fail_target "$file_path" "optionalDependencies pin(s) still stale: $(printf '%s' "$stale_pins" | tr '\n' ' ')"
}

# ---------- read a workspace-member crate's version straight out of
# Cargo.lock's persisted text, not a re-derived view (cargo metadata's
# view reflects Cargo.toml regardless of what Cargo.lock actually has on
# disk, which is exactly the gap that let a stale lockfile go unnoticed
# before). Relies on Cargo's own stable [[package]] field order for a
# path dependency: `name` immediately followed by `version`. ----------
read_cargo_lock_crate_version() {
    crate=$1
    awk -v name="$crate" '
        $0 == "name = \"" name "\"" {
            getline
            if ($0 ~ /^version = "/) {
                line = $0
                sub(/^version = "/, "", line)
                sub(/"$/, "", line)
                print line
            }
            exit
        }
    ' Cargo.lock
}

WORKSPACE_CRATES="apimock apimock-config apimock-routing apimock-server"

# ---------- refresh Cargo.lock and verify it actually landed at $ver.
# The original defect this RFC exists to fix was exactly this step:
# `cargo fetch >/dev/null 2>&1 || true` discarded the exit status and
# stderr both, so a resolution failure (e.g. C-2's major-bump pin issue)
# was invisible — the script printed "verified" while Cargo.lock was
# stale. Treat it as a first-class target: real exit status, visible
# stderr, and a direct re-read of the four workspace crates' persisted
# entries against the expected version. ----------
update_cargo_lock() {
    ver=$1
    [ -f "Cargo.lock" ] || return

    if [ "$DRY_RUN" -eq 1 ]; then
        printf '  (dry-run) would refresh Cargo.lock -> %s\n' "$ver"
        return
    fi

    # `if cmd; then` (not `out=$(cmd); status=$?`) is required here: under
    # `set -e`, a failing command substitution assigned to a variable
    # aborts the whole script immediately, before its exit status could
    # ever be checked — which is exactly the kind of invisible failure
    # this function exists to catch instead. The `if` condition is the
    # one place a command's failure is exempt from triggering `set -e`.
    if fetch_output=$(cargo fetch 2>&1); then
        fetch_status=0
    else
        fetch_status=$?
    fi
    if [ "$fetch_status" -ne 0 ]; then
        printf '%s\n' "$fetch_output" >&2
        fail_target "Cargo.lock" "cargo fetch failed (exit $fetch_status)"
        return
    fi
    git add Cargo.lock 2>/dev/null || true
    printf '  updated Cargo.lock (via cargo fetch) -> %s\n' "$ver"

    stale=""
    for crate in $WORKSPACE_CRATES; do
        actual=$(read_cargo_lock_crate_version "$crate") || actual="<unreadable>"
        [ "$actual" = "$ver" ] || stale="${stale}${stale:+, }$crate=$actual"
    done
    [ -z "$stale" ] || fail_target "Cargo.lock" "workspace crate(s) not at \"$ver\": $stale"
}

# ---------- list ----------
if [ "$LIST_MODE" -eq 1 ]; then
    printf 'Cargo workspace crates:\n'
    cargo metadata --no-deps --format-version 1 2>/dev/null | \
        jq -r '.packages[] | "\(.name)\t\(.version)"' | \
        awk -F'\t' '{ printf "  %-20s : %s\n", $1, $2 }'

    printf '\nWorkspace manifest ([workspace.package].version):\n'
    printf '  %-20s : %s\n' "$ROOT_MANIFEST" "$(read_workspace_version)"

    printf '\nnpm packages:\n'
    for f in $NPM_ROOT_PACKAGE $NPM_PLATFORM_PACKAGES; do
        [ -f "$f" ] || continue
        printf '  %-30s : %s\n' "$f" "$(jq -r '.version' "$f")"
    done

    pins=$(jq -r '.optionalDependencies // {} | to_entries[] | "\(.key)\t\(.value)"' "$NPM_ROOT_PACKAGE" 2>/dev/null || true)
    if [ -n "$pins" ]; then
        printf '\n%s optionalDependencies:\n' "$NPM_ROOT_PACKAGE"
        printf '%s\n' "$pins" | awk -F'\t' '{ printf "  %-30s : %s\n", $1, $2 }'
    fi

    [ "$UPDATE_MODE" -eq 0 ] && exit 0
fi

# ---------- update ----------
if [ "$UPDATE_MODE" -eq 1 ]; then
    [ -z "$NEW_VERSION" ] && { printf 'Error: Missing version.\n' >&2; exit 1; }

    printf 'Starting update to version "%s"...\n' "$NEW_VERSION"
    [ "$DRY_RUN" -eq 1 ] && printf '(dry-run: no files will be modified)\n'

    update_root_manifest_version "$NEW_VERSION"
    update_cargo_lock "$NEW_VERSION"

    for f in $NPM_PLATFORM_PACKAGES; do
        update_npm_package_version "$f" "$NEW_VERSION"
    done

    update_npm_root_package "$NPM_ROOT_PACKAGE" "$NEW_VERSION"

    if [ "$DRY_RUN" -eq 0 ]; then
        if [ -n "$FAILED_TARGETS" ]; then
            printf 'Error: self-verification failed for: %s\n' "$FAILED_TARGETS" >&2
            exit 1
        fi
        printf 'All targets verified at version "%s".\n' "$NEW_VERSION"
    fi
fi
