FROM rust:1.55

ARG USER_ID
ARG GROUP_ID

RUN apt-get update && \
	apt-get install -y mingw-w64 && \
	rustup target add x86_64-pc-windows-gnu

RUN apt install -y llvm-dev libclang-dev clang

RUN apt-get install -y python3 curl && \
	curl https://bootstrap.pypa.io/get-pip.py -o get-pip.py && \
	python3 get-pip.py && \
	rm get-pip.py

RUN pip install maturin

RUN addgroup --gid $GROUP_ID builder
RUN adduser --disabled-password --gecos '' --uid $USER_ID --gid $GROUP_ID builder
USER builder
