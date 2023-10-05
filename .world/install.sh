#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || ( "$Target" == 'windows' ) ]]; then
  exit
fi

pushd python
poetry run maturin build --release --interpreter python --no-sdist --out $INSTALL/lib/python/ese_parser

if [[ "$Target" == 'windows_package' ]]; then
  pip install .
fi

popd
