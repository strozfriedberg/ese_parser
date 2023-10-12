#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || ("$Target" == 'windows') ]]; then
  exit
fi

if [[ "$Target" == 'windows_package' ]]; then
  export POETRY_CACHE_DIR=python/.poetry
fi

# Install dependencies for build environment
pushd python
poetry lock --check && poetry install --no-cache
popd
