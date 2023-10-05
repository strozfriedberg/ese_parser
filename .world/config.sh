#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || ( "$Target" == 'windows' ) ]]; then
  exit
fi

# Install dependencies for build environment
pushd python
poetry lock --check && poetry install
popd
