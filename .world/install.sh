#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' ]] || [[ "$Target" != 'linux' && "$Target" != 'windows_package' ]]; then
  exit
fi

mkdir -p $INSTALL/lib/python/ese_parser
cp python/target/wheels/* $INSTALL/lib/python/ese_parser

if [[ "$Target" == 'windows_package' ]]; then
  pushd python
  pip install . --use-feature=in-tree-build
  popd
fi
