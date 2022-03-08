#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || "$Target" != 'linux' ]]; then
  exit
fi

pushd lib
cargo test --all-tragets
cargo test --all-targets --features nt_comparison
popd

pushd python
maturin build --interpreter python3.9 --release

pushd python
python -m venv test
. test/bin/activate
pip install --upgrade pip
pip install --force-reinstall target/wheels/ese_parser-0.1.0-cp36-cp36m-manylinux*_x86_64.whl
python py/test.py
popd
