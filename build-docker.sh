#!/bin/sh
set -eu
here=$(cd "$(dirname "$0")"; pwd)
if test -n "${1:-}"; then
    GIT_VERSION=$1
else
    GIT_VERSION=$(git describe --always --dirty=-modified)
fi
sudo docker build --build-arg="GIT_VERSION=$GIT_VERSION" -t build-ipgrep "$here"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
sudo docker image save build-ipgrep | tar -C "$tmp" -x

if test -f "$tmp/repositories"; then
    layertar=$(jq -r '.["build-ipgrep"].latest' <"$tmp/repositories")
    if test -n "$layertar"; then
        layertar=$tmp/blobs/sha256/$layertar
    fi
else
    layertar=$(find "$tmp" -type f -name layer.tar)
fi
if test -z "$layertar"; then
    echo "$0: could not parse docker layout to extract output" >&2
fi
tar -xvf "$layertar"
ls -l ./ipgrep
