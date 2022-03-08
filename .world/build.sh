#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || "$Target" != 'linux' ]]; then
  exit
fi

BASEDIR=$(pwd)

PYTHON=python3
VENV=venv
VENVBIN=bin

. "$VENV/$VENVBIN/activate"

pushd lib
cargo test --all-targets
cargo test --all-targets --features nt_comparison
popd

pushd python
maturin build --interpreter python3 --release

pushd python
python -m venv test
. test/bin/activate
pip install --upgrade pip
ls target/wheels
pip install --force-reinstall target/wheels/ese_parser-0.1.0-*_x86_64.whl
python py/test.py
popd
