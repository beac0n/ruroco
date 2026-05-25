#!/usr/bin/env bash
# Analyzes non-test Rust source files for human-readability LOC.
# Counts: all lines after stripping #[cfg(test)] blocks and trimming
# leading/trailing blank lines from the remainder.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_DIR="$REPO_ROOT/src"

mapfile -t RS_FILES < <(find "$SRC_DIR" -type f -name "*.rs" | sort)

declare -A module_totals
declare -a file_lines
total_all=0

for file in "${RS_FILES[@]}"; do
    # Strip #[cfg(test)] blocks, then count lines trimming leading/trailing blanks.
    loc=$(awk '
        /^[[:space:]]*#\[cfg\(test\)\]/ { in_test_attr = 1; next }
        in_test_attr && /^[[:space:]]*#\[/ { in_test_attr = 0 }
        in_test_attr && /\{/ {
            in_test = 1
            depth = 0
            in_test_attr = 0
        }
        in_test {
            for (i = 1; i <= length($0); i++) {
                c = substr($0, i, 1)
                if (c == "{") depth++
                else if (c == "}") {
                    depth--
                    if (depth == 0) { in_test = 0; next }
                }
            }
            next
        }
        { lines[++n] = $0 }
        END {
            # find first and last non-blank line
            first = 0; last = 0
            for (i = 1; i <= n; i++) {
                if (lines[i] ~ /[^[:space:]]/) {
                    if (!first) first = i
                    last = i
                }
            }
            if (first) print last - first + 1
            else print 0
        }
    ' "$file")

    rel="${file#"$SRC_DIR/"}"
    module=$(echo "$rel" | cut -d/ -f1)
    module_totals["$module"]=$(( ${module_totals["$module"]:-0} + loc ))
    total_all=$(( total_all + loc ))

    file_lines+=("$(printf "%6d  %s" "$loc" "$rel")")
done

printf '%s\n' "${file_lines[@]}" | sort -rn

echo ""
echo "--- by module ---"
for mod in $(echo "${!module_totals[@]}" | tr ' ' '\n' | sort); do
    printf "%6d  %s/\n" "${module_totals[$mod]}" "$mod"
done

echo ""
printf "%6d  TOTAL\n" "$total_all"
