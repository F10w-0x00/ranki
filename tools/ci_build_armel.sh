#!/bin/bash
set -euo pipefail

echo "=========================================================="
echo " Starting RAnki Pure Rust ARMEL Build Pipeline (Task 1)"
echo " Target Architecture: armel (ARMv7 softfp / EABI5)"
echo " GLIBC Ceiling Gate:  <= GLIBC_2.20"
echo "=========================================================="

cd /workspace

# 1. Environment & Cross pkg-config
export PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabi/pkgconfig:/usr/share/pkgconfig
export PKG_CONFIG_LIBDIR=/usr/lib/arm-linux-gnueabi
export CC_arm_unknown_linux_gnueabi=arm-linux-gnueabi-gcc
export CXX_arm_unknown_linux_gnueabi=arm-linux-gnueabi-g++
export AR_arm_unknown_linux_gnueabi=arm-linux-gnueabi-ar
chmod +x tools/arm_linker_wrapper.sh
export CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABI_LINKER=/workspace/tools/arm_linker_wrapper.sh

# 2. Build Pure Rust Host Binary for Kindle armel
echo "[1/3] Building Pure Rust Host Binary (ranki)..."
rustup target add arm-unknown-linux-gnueabi
cargo build --release --target arm-unknown-linux-gnueabi

# 3. Strip and prepare artifact
echo "[2/3] Preparing binary artifact..."
mkdir -p build
cp target/arm-unknown-linux-gnueabi/release/ranki build/ranki-armel
arm-linux-gnueabi-strip build/ranki-armel

echo "=========================================================="
echo " Build Completed Successfully! Binary Information:"
echo "=========================================================="
file build/ranki-armel
sha256sum build/ranki-armel
ls -lh build/ranki-armel

# 4. Automated Verification Gates
echo "[3/3] Running Automated Verification Gates (Task 1)..."
python3 tools/check_symbols.py build/ranki-armel
python3 tools/check_dt_needed.py build/ranki-armel
python3 tools/check_dynsym.py build/ranki-armel

echo "All verification gates passed with 0 errors!"
