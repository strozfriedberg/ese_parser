#!/bin/bash -ex

. .world/build_config.sh

if [[ "$Linkage" == 'static' || ( "$Target" == 'windows' ) ]]; then
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

$PYTHON -m venv --clear $VENV
. "$VENV/$VENVBIN/activate"
pip install toml maturin
deactivate
