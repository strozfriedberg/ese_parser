#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || ( "$Target" == 'windows' ) ]]; then
  exit
fi

mkdir -p $INSTALL/lib/python/ese_parser
cp python/target/wheels/* $INSTALL/lib/python/ese_parser

if [[ "$Target" == 'windows_package' ]]; then
  pushd python
  pip install .
  popd
fi
