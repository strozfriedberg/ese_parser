#!/bin/bash

set -ex

rm -rf builds
mkdir builds

IMAGE_NAME=python-mingw-cross-build

docker build \
	--build-arg USER_ID=$(id -u) \
	--build-arg GROUP_ID=$(id -g) \
	--file docker/Dockerfile --tag $IMAGE_NAME .

docker run --rm \
	-v ${PWD}:/ese_parser \
	-v ${PWD}/builds:/builds \
	$IMAGE_NAME /bin/bash -c '\
	cd ~
	cp -r /ese_parser .
	cd ese_parser

	pushd app
	cargo build --target x86_64-pc-windows-gnu --release
	cp target/x86_64-pc-windows-gnu/release/ese_parser.exe /builds
	popd

	pushd docker/
	unzip python37.zip
	popd

	pushd python
	 PYO3_CROSS_PYTHON_VERSION=3.7 \
	 PYO3_CROSS_LIB_DIR="$PWD/../docker/Python37/libs" \
	maturin build --target x86_64-pc-windows-gnu -i "../docker/mypython" --release
	cp target/wheels/ese_parser-0.1.0-cp36-none-win_amd64.whl /builds
	popd
'
