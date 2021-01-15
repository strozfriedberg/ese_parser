#!/bin/bash

set -ex

IMAGE_NAME=python-mingw-cross-build

docker build --file python/docker/Dockerfile --tag $IMAGE_NAME .

docker run --rm -v ${PWD}:/ese_parser -v${PWD}/builds:/builds $IMAGE_NAME /bin/bash -c '\
	cd ~
	cp -r /ese_parser .
	cd ese_parser/python/docker/
	unzip python37.zip
	cd ..
	 PYO3_CROSS_PYTHON_VERSION=3.7 \
	 PYO3_CROSS_LIB_DIR="$PWD/docker/Python37/libs" \
	maturin build --target x86_64-pc-windows-gnu -i "docker/mypython" --release
	cp target/wheels/ese_parser-0.1.0-cp37-none-win_amd64.whl /builds
'
