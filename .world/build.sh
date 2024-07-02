#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || ("$Target" == 'windows') ]]; then
  exit
fi

if [[ "$Target" == 'windows_package' ]]; then
  PYTHON=python
else
  PYTHON=python3.11
fi

pushd lib
cargo test --all-targets
cargo test --all-targets --features nt_comparison
popd

pushd python
poetry run maturin develop --release
poetry run pytest
poetry run maturin build -i $PYTHON --release
popd
