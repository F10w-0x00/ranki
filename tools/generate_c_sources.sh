#!/bin/bash
set -euo pipefail

echo "=========================================================="
echo " Generating C sources from Vala & Protobuf"
echo "=========================================================="

cd "$(dirname "$0")/.."

# 1. Compile host protoc-gen-vala plugin directly with valac
if [ ! -f protobuf-vala/src/protoc-gen-vala ]; then
    echo "[1/2] Compiling protoc-gen-vala plugin with valac..."
    valac --pkg posix -X -DVERSION=\"0.1.0\" -o protobuf-vala/src/protoc-gen-vala \
        protobuf-vala/src/protobuf-vala.vala \
        protobuf-vala/src/descriptor.pb.vala \
        protobuf-vala/src/plugin.pb.vala \
        protobuf-vala/src/protoc-gen-vala.vala
fi

# 2. Generate Vala protobuf bindings from anki/proto
echo "[2/2] Generating Vala bindings and translating to C..."
mkdir -p build/proto build/src_c
protoc --experimental_allow_proto3_optional \
       --plugin=protoc-gen-vala=./protobuf-vala/src/protoc-gen-vala \
       --vala_out=build/proto \
       -Ianki/proto \
       anki/proto/anki/*.proto

# 3. Find all generated protobuf vala files
PROTO_VALA=$(find build/proto -name "*.vala")
echo "Discovered generated protobuf Vala files: $(echo "$PROTO_VALA" | wc -w)"

# 4. Translate Vala application to C
valac -C \
    --vapidir=vapi \
    --pkg gtk+-2.0 \
    --pkg webkit-1.0 \
    --pkg libsoup-2.4 \
    --pkg libarchive \
    --pkg posix \
    protobuf-vala/src/protobuf-vala.vala \
    ${PROTO_VALA} \
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
