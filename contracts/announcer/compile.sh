#!/bin/bash

CURDIR=`pwd`
BASEDIR="~/lunc/taskforce"
CONTRACTDIR="${BASEDIR}/contracts"
CONTRACTNAME=$(basename $(dirname $(realpath "$0")))
CONTRACTFILE=$(echo "$CONTRACTNAME" | tr '-' '_')

echo "Compiling $CONTRACTNAME"
cd $CONTRACTDIR/$CONTRACTNAME
docker run --rm -v "$(pwd)":/code   --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target   --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry cosmwasm/rust-optimizer:0.14.0

echo "Done."
cd $CURDIR
