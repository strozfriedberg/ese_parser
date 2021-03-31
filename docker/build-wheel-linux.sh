#!/bin/bash

set -ex

rm -rf builds
mkdir builds

IMAGE_NAME=python-linux-build

docker build \
	--build-arg USER_ID=$(id -u) \
	--build-arg GROUP_ID=$(id -g) \
	--file docker/Dockerfile.linux --tag $IMAGE_NAME .

docker run --rm \
	-v ${PWD}:/ese_parser \
	-v ${PWD}/builds:/builds \
	$IMAGE_NAME /bin/bash -c '\
	cd ~
	cp -r /ese_parser .
	cd ese_parser

	pushd app
	cargo build --release
	cp target/release/ese_parser /builds
	popd

	pushd python
	maturin build --release
	cp target/wheels/ese_parser-0.1.0-cp36-abi3-manylinux2010_x86_64.whl /builds
	popd
'
