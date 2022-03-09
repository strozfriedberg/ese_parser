#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' ]] || [[ "$Target" != 'linux' && "$Target" != 'windows_package' ]]; then
  exit
fi

BASEDIR=$(pwd)

VENV=venv
if [[ "$Target" == 'linux' ]]; then
  PYTHON=python3
  VENVBIN=bin
elif [[ "$Target" == 'windows_package' ]]; then
  PYTHON=python
  VENVBIN=Scripts
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
pip install --force-reinstall target/wheels/ese_parser-0.1.0-*_x86_64.whl
python py/test.py
popd
