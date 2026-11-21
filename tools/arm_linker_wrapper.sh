#!/bin/bash
set -euo pipefail

echo "[LINKER WRAPPER] Running symbol isolation pass before link..."

SYMS_FILE="/tmp/sqlite_redefine.txt"
rm -f "$SYMS_FILE"

# 1. Collect all archives and object files from arguments, /tmp, and target/
declare -A SEEN_OBJECTS=()
declare -a OBJECTS=()

add_object() {
    local a="$1"
    if [ -f "$a" ] && [ -z "${SEEN_OBJECTS["$a"]:-}" ]; then
        SEEN_OBJECTS["$a"]=1
        OBJECTS+=("$a")
    fi
}

for arg in "$@"; do
    if [[ "$arg" == *.rlib ]] || [[ "$arg" == *.a ]] || [[ "$arg" == *.o ]]; then
        add_object "$arg"
    fi
done

# Also discover any rlibs, static libs, or CGU object files in /tmp or target/arm-unknown-linux-gnueabi
while IFS= read -r f; do
    add_object "$f"
done < <(find /tmp target/arm-unknown-linux-gnueabi -type f \( -name "*.rlib" -o -name "*.a" -o -name "*.o" \) 2>/dev/null || true)

echo "[LINKER WRAPPER] Discovered ${#OBJECTS[@]} objects and archives to inspect."

# 2. Extract all defined sqlite3_* symbols
for obj in "${OBJECTS[@]}"; do
    arm-linux-gnueabi-nm -g --defined-only "$obj" 2>/dev/null | awk '{if ($2 ~ /^[TDBRC]$/ && $3 ~ /^sqlite3_/) print $3, "anki_" $3}' >> "$SYMS_FILE" || true
done

if [ -f "$SYMS_FILE" ] && [ -s "$SYMS_FILE" ]; then
    sort -u "$SYMS_FILE" -o "$SYMS_FILE"
    COUNT=$(wc -l < "$SYMS_FILE")
    echo "[LINKER WRAPPER] Found $COUNT sqlite3 symbols to redefine across objects."

    # 3. Redefine in all objects and archives so definitions and callers (CGU .o) match
    for obj in "${OBJECTS[@]}"; do
        arm-linux-gnueabi-objcopy --redefine-syms="$SYMS_FILE" "$obj" 2>/dev/null || true
    done
    echo "[LINKER WRAPPER] Symbol redefinition completed successfully."
else
    echo "[LINKER WRAPPER] No sqlite3 symbols detected."
fi

# 4. Invoke real cross GCC linker with complete archive isolation
exec arm-linux-gnueabi-gcc -Wl,--exclude-libs,ALL "$@"
