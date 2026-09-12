#!/usr/bin/env bash

set -u

grammar_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
repo_root=$(cd "$grammar_root/.." && pwd -P)
temp_dir=$(mktemp -d)
parse_output=$temp_dir/parse-output
parser_library=$temp_dir/rock.so
trap 'rm -rf "$temp_dir"' EXIT

declare -A excluded=()
manifest_count=0

load_manifest() {
    local manifest=$1
    local relative

    while IFS= read -r relative || [[ -n $relative ]]; do
        [[ -z $relative ]] && continue

        if [[ $relative != stdlib/*.rk && $relative != examples/*.rk && $relative != examples/**/*.rk && $relative != test_projects/*.rk && $relative != test_projects/**/*.rk ]]; then
            printf 'invalid conformance exclusion outside source roots: %s\n' "$relative" >&2
            exit 1
        fi
        if [[ ! -f $repo_root/$relative ]]; then
            printf 'missing conformance exclusion: %s\n' "$relative" >&2
            exit 1
        fi
        if [[ -n ${excluded[$relative]+present} ]]; then
            printf 'duplicate conformance exclusion: %s\n' "$relative" >&2
            exit 1
        fi

        excluded[$relative]=1
        ((manifest_count += 1))
    done < "$manifest"
}

load_manifest "$grammar_root/test/conformance/compiler-failures.txt"
load_manifest "$grammar_root/test/conformance/malformed-sources.txt"

mapfile -d '' candidates < <(
    find "$repo_root/stdlib" "$repo_root/examples" "$repo_root/test_projects" \
        -type f -name '*.rk' -print0 | sort -z
)

if (( ${#candidates[@]} != 120 )); then
    printf 'unexpected Rock source count: expected 120, found %d\n' "${#candidates[@]}" >&2
    exit 1
fi
if (( manifest_count != 12 )); then
    printf 'unexpected exclusion count: expected 12, found %d\n' "$manifest_count" >&2
    exit 1
fi

passed=0
failed=0

cd "$grammar_root"
tree-sitter build --output "$parser_library"
for file in "${candidates[@]}"; do
    relative=${file#"$repo_root"/}
    if [[ -n ${excluded[$relative]+present} ]]; then
        continue
    fi

    if timeout 10s tree-sitter parse --lib-path "$parser_library" --lang-name rock "$file" \
        >"$parse_output" 2>&1 \
        && ! grep -Eq '\((ERROR|MISSING)([[:space:]]|\))' "$parse_output"; then
        ((passed += 1))
    else
        printf 'failed: %s\n' "$relative" >&2
        ((failed += 1))
    fi
done

if (( failed != 0 )); then
    printf 'tree-sitter-rock conformance: %d passed, %d excluded, %d failed\n' \
        "$passed" "$manifest_count" "$failed" >&2
    exit 1
fi

printf 'tree-sitter-rock conformance: %d files passed, %d excluded\n' \
    "$passed" "$manifest_count"
