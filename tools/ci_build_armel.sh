#!/bin/bash
set -euo pipefail

echo "=========================================================="
echo " Starting RAnki ARMEL Cross-Compilation Pipeline (M0)"
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
export CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABI_LINKER=arm-linux-gnueabi-gcc

# 2. Patch and Compile Rust rslib backend
echo "[1/4] Compiling Rust rslib for arm-unknown-linux-gnueabi..."
cd anki
if git apply --reverse --check ../cargo-toml.patch >/dev/null 2>&1; then
    echo "  cargo-toml.patch already applied."
else
    patch -p1 < ../cargo-toml.patch
fi

if git apply --reverse --check ../rslib-cargo-toml.patch >/dev/null 2>&1; then
    echo "  rslib-cargo-toml.patch already applied."
else
    patch -p1 < ../rslib-cargo-toml.patch
fi

if git apply --reverse --check ../rslib-src-lib-rs.patch >/dev/null 2>&1; then
    echo "  rslib-src-lib-rs.patch already applied."
else
    patch -p1 < ../rslib-src-lib-rs.patch
fi

rustup target add arm-unknown-linux-gnueabi
cargo build --release --target arm-unknown-linux-gnueabi -p anki
cd ..

# 3. Build liblipc stub for linker resolution
echo "[2/4] Creating liblipc.so.1 stub for linking..."
mkdir -p build/libs
arm-linux-gnueabi-gcc -shared -fPIC -Wl,-soname,liblipc.so.1 tools/liblipc_stub.c -o build/libs/liblipc.so.1
ln -sf liblipc.so.1 build/libs/liblipc.so

# 4. Compile C and link final ranki-armel binary
echo "[3/4] Linking ranki-armel binary..."
PKG_CFLAGS=$(pkg-config --cflags gtk+-2.0 webkit-1.0 libsoup-2.4 libarchive json-glib-1.0)
PKG_LIBS=$(pkg-config --libs gtk+-2.0 webkit-1.0 libsoup-2.4 libarchive)

arm-linux-gnueabi-gcc \
    -O2 \
    build/src_c/*.c \
    -I. \
    ${PKG_CFLAGS} \
    -Lbuild/libs -llipc \
    -L/usr/lib/arm-linux-gnueabi -L/lib/arm-linux-gnueabi \
    anki/target/arm-unknown-linux-gnueabi/release/librslib.a \
    /usr/lib/arm-linux-gnueabi/libjson-glib-1.0.a \
    ${PKG_LIBS} \
    -lpthread -ldl -lm -lrt \
    -o build/ranki-armel

arm-linux-gnueabi-strip build/ranki-armel

echo "=========================================================="
echo " Build Completed Successfully! Binary Information:"
echo "=========================================================="
file build/ranki-armel
sha256sum build/ranki-armel
ls -lh build/ranki-armel

# 5. Automated Verification Gates
echo "[4/4] Running Automated Verification Gates (M0.5 & M0.6)..."
python3 tools/check_symbols.py build/ranki-armel
python3 tools/check_dt_needed.py build/ranki-armel

echo "All verification gates passed with 0 errors!"
