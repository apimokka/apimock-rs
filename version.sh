#!/bin/sh
#
# version.sh – Cargo, Node.js, Python 関連ファイルのバージョンを一括更新
#
# 必要ツール: cargo, jq, awk, grep, find

# ---------- ヘルプ ----------
show_help() {
    cat <<EOF
Usage: ${0##*/} [OPTIONS]

Options:
  -l, --list                List each crate with its current version.
  -u, --update VERSION      Set all Cargo, npm, and pip files to VERSION.
                            Includes package.json in subdirectories of packages.
  -d, --dry-run             Show what would be changed, but do not modify files.
  -h, --help                Show this help and exit.

Examples:
  ${0##*/} --list
  ${0##*/} --update 1.2.3
EOF
    exit 0
}

# ---------- 引数解析 ----------
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

# ---------- ツール確認 ----------
for cmd in cargo jq awk find; do
    command -v "$cmd" >/dev/null 2>&1 || { printf 'Error: %s not found.\n' "$cmd" >&2; exit 1; }
done

# ---------- メタデータ取得 ----------
METADATA_JSON=$(cargo metadata --no-deps --format-version 1)
[ -z "$METADATA_JSON" ] && { printf 'Error: Failed to obtain metadata.\n' >&2; exit 1; }

# ---------- 更新関数 ----------
# update_file <path> <type:toml|json> <version>
update_file() {
    file_path=$1; type=$2; ver=$3
    [ ! -f "$file_path" ] && return

    if [ "$DRY_RUN" -eq 1 ]; then
        printf '  (dry-run) would update %s\n' "$file_path"
        return
    fi

    tmp=$(mktemp) || exit 1
    if [ "$type" = "toml" ]; then
        # TOML用: 最初の [package] や [project] セクション直後の version を狙い撃ち
        awk -v nv="$ver" '
            !found && /^[[:space:]]*version[[:space:]]*=/ {
                print "version = \"" nv "\""
                found=1; next
            }
            { print }
        ' "$file_path" > "$tmp"
    else
        # JSON用: jq で確実に更新
        jq --arg v "$ver" '.version = $v' "$file_path" > "$tmp"
    fi

    mv "$tmp" "$file_path"
    git add "$file_path"
    printf '  updated %s\n' "$file_path"
}

# ---------- メイン処理 ----------

# 1. バージョン一覧表示
if [ "$LIST_MODE" -eq 1 ]; then
    printf 'Current versions:\n'
    echo "$METADATA_JSON" | jq -r '.packages[] | "\(.name)\t\(.version)"' | \
        awk -F'\t' '{ printf "  %-20s : %s\n", $1, $2 }'
    [ "$UPDATE_MODE" -eq 0 ] && exit 0
fi

# 2. バージョン更新
if [ "$UPDATE_MODE" -eq 1 ]; then
    [ -z "$NEW_VERSION" ] && { printf 'Error: Missing version.\n' >&2; exit 1; }

    printf 'Starting update to version "%s"...\n' "$NEW_VERSION"

    # cargo metadata から各クレートのパスを抽出
    echo "$METADATA_JSON" | jq -r '.packages[] | .manifest_path' | while read -r cargo_toml; do
        crate_dir=$(dirname "$cargo_toml")
        
        # 1. Cargo.toml 更新
        update_file "$cargo_toml" "toml" "$NEW_VERSION"

        # 2. [拡張] 直下のサブディレクトリにある package.json を検索・更新
        # find で crate_dir の直下 (-maxdepth 1) のディレクトリを探し、
        # その中にある package.json を見つける
        find "$crate_dir" -mindepth 1 -maxdepth 2 -type d -print0 2>/dev/null | \
        while IFS= read -r -d '' subdir; do
            sub_pkg_json="$subdir/package.json"
            if [ -f "$sub_pkg_json" ]; then
                update_file "$sub_pkg_json" "json" "$NEW_VERSION"
            fi
            sub_pkg_lock_json="$subdir/package-lock.json"
            if [ -f "$sub_pkg_lock_json" ]; then
                update_file "$sub_pkg_lock_json" "json" "$NEW_VERSION"
            fi
        done

        # 3. 同一ディレクトリ内の pyproject.toml をチェック
        update_file "$crate_dir/pyproject.toml" "toml" "$NEW_VERSION"
    done

    # Cargo.lock の更新（dry-run でない場合のみ）
    #
    # Edit this package's own version line directly. Do NOT invoke cargo
    # to refresh the lockfile.
    #
    # `cargo fetch` was used here until 2026-08-26. Any cargo command
    # that rewrites this lockfile re-resolves it, and on this branch that
    # consolidates two `rand` entries (0.9.4 and 0.10.1) down to one,
    # taking `rand_core 0.10.1`, `chacha20` and `cpufeatures` with it —
    # 43 lines removed. The test suite then fails to compile against the
    # surviving `rand`, because it is written for the other one.
    #
    # The committed lockfile is the working configuration; cargo's
    # re-resolution is what breaks it. It bit the 4.8.1 release and would
    # have shipped a version whose tests do not build.
    #
    # Verified below rather than assumed: the only permitted change is
    # this package's version line.
    if [ "$DRY_RUN" -eq 0 ] && [ -f "Cargo.lock" ]; then
        LOCK_BEFORE=$(grep -c '' Cargo.lock)

        awk -v new="$NEW_VERSION" '
            /^\[\[package\]\]$/       { inpkg = 1; ours = 0 }
            inpkg && /^name = "apimock"$/ { ours = 1 }
            ours && /^version = / && !done_ours {
                print "version = \"" new "\""; done_ours = 1; next
            }
            { print }
        ' Cargo.lock > Cargo.lock.tmp && mv Cargo.lock.tmp Cargo.lock

        LOCK_AFTER=$(grep -c '' Cargo.lock)
        if [ "$LOCK_BEFORE" -ne "$LOCK_AFTER" ]; then
            printf 'Error: Cargo.lock changed line count (%s -> %s); expected only a version edit.\n' \
                "$LOCK_BEFORE" "$LOCK_AFTER" >&2
            exit 1
        fi
        if ! grep -q "^version = \"$NEW_VERSION\"$" Cargo.lock; then
            printf 'Error: Cargo.lock was not updated to %s.\n' "$NEW_VERSION" >&2
            exit 1
        fi
        printf '  updated Cargo.lock (version line only) -> %s\n' "$NEW_VERSION"

        git add Cargo.lock
    fi
fi