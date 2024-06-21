#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || ("$Target" == 'windows') ]]; then
  exit
fi

pushd lib
cargo test --all-targets
cargo test --all-targets --features nt_comparison
popd

pushd python
poetry run maturin develop --release
poetry run pytest
poetry run maturin build -i python3.11 --release
popd
