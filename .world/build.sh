#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || ( "$Target" == 'windows' ) ]]; then
  exit
fi

BASEDIR=$(pwd)

VENV=venv
if [[ "$Target" == 'windows_package' ]]; then
  PYTHON=python
  VENVBIN=Scripts
else
  PYTHON=python3
  VENVBIN=bin
fi

. "$VENV/$VENVBIN/activate"

pushd lib
cargo test --all-targets
cargo test --all-targets --features nt_comparison
popd

pushd python
maturin build --interpreter $PYTHON --release

python -m venv test
. test/$VENVBIN/activate
ls target/wheels
pip install --force-reinstall target/wheels/ese_parser-0.1.0-*.whl
python py/test.py
popd
