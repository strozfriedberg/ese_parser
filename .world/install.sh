#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || "$Target" != 'linux' ]]; then
  exit
fi

mkdir -p $INSTALL/lib/python/ese_parser
cp python/target/wheels/* $INSTALL/lib/python/ese_parser
