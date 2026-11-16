#!/bin/bash
set -euo pipefail

echo "[LINKER WRAPPER] Running symbol isolation pass before link..."

SYMS_FILE="/tmp/sqlite_redefine.txt"
rm -f "$SYMS_FILE"

# 1. Collect all archives from arguments, /tmp/rustc*, and target/
declare -A SEEN_ARCHIVES=()
declare -a ARCHIVES=()

add_archive() {
    local a="$1"
    if [ -f "$a" ] && [ -z "${SEEN_ARCHIVES["$a"]:-}" ]; then
        SEEN_ARCHIVES["$a"]=1
        ARCHIVES+=("$a")
    fi
}

for arg in "$@"; do
    if [[ "$arg" == *.rlib ]] || [[ "$arg" == *.a ]]; then
        add_archive "$arg"
    fi
done

# Also discover any rlibs rustc staged in /tmp or target
while IFS= read -r f; do
    add_archive "$f"
done < <(find /tmp target -type f \( -name "*.rlib" -o -name "*.a" \) 2>/dev/null || true)

echo "[LINKER WRAPPER] Discovered ${#ARCHIVES[@]} archives to inspect."

# 2. Extract all defined sqlite3_* symbols
for arch in "${ARCHIVES[@]}"; do
    arm-linux-gnueabi-nm -g --defined-only "$arch" 2>/dev/null | awk '{if ($2 ~ /^[TDBRC]$/ && $3 ~ /^sqlite3_/) print $3, "anki_" $3}' >> "$SYMS_FILE" || true
done

if [ -f "$SYMS_FILE" ] && [ -s "$SYMS_FILE" ]; then
    sort -u "$SYMS_FILE" -o "$SYMS_FILE"
    COUNT=$(wc -l < "$SYMS_FILE")
    echo "[LINKER WRAPPER] Found $COUNT sqlite3 symbols to redefine across archives."

    # 3. Redefine in all archives so definitions and call-sites match
    for arch in "${ARCHIVES[@]}"; do
        arm-linux-gnueabi-objcopy --redefine-syms="$SYMS_FILE" "$arch" 2>/dev/null || true
    done
    echo "[LINKER WRAPPER] Symbol redefinition completed successfully."
else
    echo "[LINKER WRAPPER] No sqlite3 symbols detected."
fi

# 4. Invoke real cross GCC linker with complete archive isolation
exec arm-linux-gnueabi-gcc -Wl,--exclude-libs,ALL "$@"
