#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || ( "$Target" == 'windows' ) ]]; then
  exit
fi

pushd python
poetry run maturin build --release --interpreter python

mkdir -p $INSTALL/lib/python/ese_parser
cp target/wheels/* $INSTALL/lib/python/ese_parser


popd
