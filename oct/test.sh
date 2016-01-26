#!/bin/bash
set -eu

go version || eval "$(gimme 1.5)"
ROOT=`dirname "${BASH_SOURCE[0]}"`
export PATH="$CARGO_TARGET_DIR/debug:$PATH"
export GOPATH=`mktemp -d`

cleanup() {
	rm -rf "$GOPATH"
}

trap cleanup 0

echo "Downloading OCI Test..."
go get -d github.com/opencontainers/specs
go get -d github.com/huawei-openlab/oct
go get -d github.com/zenlinTechnofreak/ocitools

patch -p1 -d "$GOPATH/src/github.com/huawei-openlab/oct" < "$ROOT/oct.patch"
cd "$GOPATH/src/github.com/huawei-openlab/oct"
make
./ocitest -r encage
