#!/bin/bash
set -euo pipefail

echo "[LINKER WRAPPER] Running symbol isolation pass before link..."

SYMS_FILE="/tmp/sqlite_redefine.txt"
rm -f "$SYMS_FILE"

# 1. Discover all defined sqlite3_* symbols across all target rlibs
if [ -d target/arm-unknown-linux-gnueabi/release/deps ]; then
    find target/arm-unknown-linux-gnueabi/release/deps -name "*.rlib" | while read -r rlib; do
        arm-linux-gnueabi-nm -g --defined-only "$rlib" 2>/dev/null | awk '{if ($2 ~ /^[TDBRC]$/ && $3 ~ /^sqlite3_/) print $3, "anki_" $3}' >> "$SYMS_FILE" || true
    done
fi

if [ -f "$SYMS_FILE" ] && [ -s "$SYMS_FILE" ]; then
    sort -u "$SYMS_FILE" -o "$SYMS_FILE"
    COUNT=$(wc -l < "$SYMS_FILE")
    echo "[LINKER WRAPPER] Found $COUNT sqlite3 symbols to redefine across rlibs."

    # 2. Redefine in all rlibs (both definitions and call sites)
    find target/arm-unknown-linux-gnueabi/release/deps -name "*.rlib" | while read -r rlib; do
        arm-linux-gnueabi-objcopy --redefine-syms="$SYMS_FILE" "$rlib" 2>/dev/null || true
    done
    echo "[LINKER WRAPPER] Symbol redefinition completed."
else
    echo "[LINKER WRAPPER] No sqlite3 symbols detected."
fi

# 3. Invoke real cross GCC linker with complete archive isolation
exec arm-linux-gnueabi-gcc -Wl,--exclude-libs,ALL "$@"
