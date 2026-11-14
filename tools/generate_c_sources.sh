#!/bin/bash
set -euo pipefail

echo "=========================================================="
echo " Generating C sources from Vala & Protobuf"
echo "=========================================================="

cd "$(dirname "$0")/.."

# 1. Build host protoc-gen-vala plugin
if [ ! -f protobuf-vala/src/protoc-gen-vala ]; then
    echo "[1/2] Building protoc-gen-vala plugin on host..."
    cd protobuf-vala
    autoreconf -fi
    ./configure
    make -j$(nproc)
    cd ..
fi

# 2. Generate Vala protobuf bindings from anki/proto
echo "[2/2] Generating Vala bindings and translating to C..."
mkdir -p build/proto build/src_c
protoc --plugin=protoc-gen-vala=./protobuf-vala/src/protoc-gen-vala \
       --vala_out=build/proto \
       -Ianki/proto \
       anki/proto/anki/*.proto

# 3. Translate Vala application to C
valac -C \
    --vapidir=vapi \
    --pkg gtk+-2.0 \
    --pkg webkit-1.0 \
    --pkg libsoup-2.4 \
    --pkg libarchive \
    --pkg posix \
    protobuf-vala/src/protobuf-vala.vala \
    build/proto/*.vala \
    spices.vala \
    lipstick.vala \
    backend.vala \
    views/base.vala \
    views/decktree.vala \
    views/review.vala \
    views/settings.vala \
    main.vala \
    -d build/src_c

echo "C sources generated successfully in build/src_c/ ($(ls -1 build/src_c/*.c | wc -l) files)!"
