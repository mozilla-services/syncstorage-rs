#!/bin/bash

# *NOTE*: Make sure to update cargo plugins after protobuf updates
#  cargo install grpcio-compiler
#  cargo install protobuf-codegen
# May need to delete the ./src/*/[^(mod)].rs to force regeneration of files
# (deleting the mod.rs files will require adding `pub (crate)mod crate::*`
# cross referencing.)
set -e
cd "$(dirname "$0")"

## remove old files:
echo "Purging old files..."
find src -name "*.rs" -and -not \( -name "mod.rs" -or -name "lib.rs" \) -print -delete

## updating plugins
echo "Updating cargo..."
cargo update
echo "Updating plugin..."
cargo install protobuf-codegen

if ! [[ -x "$(command -v grpc_rust_plugin)" ]]; then
    echo "Error: grpc_rust_plugin was not found"
    echo
    echo "To install, run: cargo install grpcio-compiler"
    exit 1
fi

echo "Pulling git submodules"
git submodule update --init --recursive

apis=grpc/third_party/googleapis

proto_files="
grpc/src/proto/grpc/testing/empty.proto
"

for proto in $proto_files; do
    echo "Processing: $proto"
    protoc \
        --rust_out=$PWD/src \
        --grpc_out=$PWD/src \
        --plugin=protoc-gen-grpc=`which grpc_rust_plugin` \
        --proto_path=grpc/src/proto/grpc/testing \
        $proto
done

old_proto_dirs="
bigtable/admin/cluster/v1
bigtable/admin/table/v1
bigtable/admin/v2
bigtable/v1
bigtable/v2
iam/v1
longrunning
pubsub/v1
pubsub/v1beta2
rpc
spanner/admin/database/v1
spanner/admin/instance/v1
spanner/v1
"

proto_dirs="
iam/v1
longrunning
rpc
spanner/admin/database/v1
spanner/admin/instance/v1
spanner/v1
"
SRC_ROOT=$PWD


for dir in $proto_dirs; do
    mkdir -p "$SRC_ROOT/src/$dir"
    echo "Processing: $dir..."

    for proto in `find $apis/google/$dir/*.proto`; do
        echo "Processing: $proto ..."
        protoc \
            --rust_out="$SRC_ROOT/src/$dir" \
            --grpc_out="$SRC_ROOT/src/$dir" \
            --plugin=protoc-gen-grpc="`which grpc_rust_plugin`" \
            --proto_path="$apis:grpc/third_party/upb:grpc/third_party/protobuf/src/:" \
            $proto
    done
done

echo "Make sure you generate the mod.rs files!"

# ls -1 --color=never . |grep -v mod |sed "s/\.rs//" |sed "s/^/pub mod /" | sed "s/$/;/" > mod.rs \; --print
# echo "pub(crate) use crate::empty;" >> */v1/mod.rs
