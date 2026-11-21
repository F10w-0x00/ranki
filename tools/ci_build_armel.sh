#!/bin/bash
set -euo pipefail

echo "=========================================================="
echo " Starting RAnki Route B Build & Verification Pipeline"
echo " Target Architecture: armel (ARMv7 softfp / EABI5)"
echo " GLIBC Ceiling Gate:  <= GLIBC_2.20"
echo "=========================================================="

cd /workspace

# 0. Run Pure-Rust Core Unit Tests on Host x86_64 first (Fail Fast!)
echo "[0/4] Running Pure-Rust Core Unit Tests (cargo test --lib)..."
cargo test --lib

# 1. Environment & Cross pkg-config for ARMEL
export PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabi/pkgconfig:/usr/share/pkgconfig
export PKG_CONFIG_LIBDIR=/usr/lib/arm-linux-gnueabi
export CC_arm_unknown_linux_gnueabi=arm-linux-gnueabi-gcc
export CXX_arm_unknown_linux_gnueabi=arm-linux-gnueabi-g++
export AR_arm_unknown_linux_gnueabi=arm-linux-gnueabi-ar
chmod +x tools/arm_linker_wrapper.sh
export CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABI_LINKER=/workspace/tools/arm_linker_wrapper.sh

# 2. Build Pure Rust Route B Binary for Kindle armel
echo "[1/4] Building RAnki Route B Binary (arm-unknown-linux-gnueabi)..."
rustup target add arm-unknown-linux-gnueabi
cargo build --release --target arm-unknown-linux-gnueabi

# 3. Strip and prepare artifact
echo "[2/4] Stripping and preparing binary artifact..."
mkdir -p build
cp target/arm-unknown-linux-gnueabi/release/ranki build/ranki-armel
arm-linux-gnueabi-strip build/ranki-armel

echo "=========================================================="
echo " Build Completed Successfully! Binary Information:"
echo "=========================================================="
file build/ranki-armel
sha256sum build/ranki-armel
ls -lh build/ranki-armel

# 4. Automated Mechanical Verification Gates
echo "[3/4] Running Automated Binary Mechanical Gates..."
python3 tools/check_symbols.py build/ranki-armel
python3 tools/check_dt_needed.py build/ranki-armel
python3 tools/check_dynsym.py build/ranki-armel

# 5. Binary size budget check (<= 20 MB per CONSTRAINTS.md)
echo "[4/4] Checking binary size budget (<= 20 MB)..."
BIN_SIZE=$(stat -c%s build/ranki-armel)
MAX_SIZE=$((20 * 1024 * 1024))
if [ "$BIN_SIZE" -gt "$MAX_SIZE" ]; then
    echo "[FAIL] build/ranki-armel size ($BIN_SIZE bytes) exceeds 20 MB budget!"
    exit 1
fi
echo "[PASS] Binary size $BIN_SIZE bytes <= $MAX_SIZE bytes (20 MB budget)."
echo "All verification gates passed with 0 errors!"
