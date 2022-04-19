FROM rust:buster
RUN apt update && apt-get install -y \
            curl gcc g++ git make pkg-config cmake \
            libssl-dev protobuf-compiler openssl musl-tools

COPY ./ci/install-openssl.sh .
RUN ./install-openssl.sh

ADD ./identities .

RUN rustup target add x86_64-unknown-linux-musl

ENV PKG_CONFIG_ALLOW_CROSS=1 \
    OPENSSL_STATIC=true \
    OPENSSL_DIR=/musl

RUN cargo build -p serverd -p agentd --target=x86_64-unknown-linux-musl
