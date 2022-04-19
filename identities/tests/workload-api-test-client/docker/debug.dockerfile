FROM rust:buster

RUN apt update && apt-get install -y \
            curl gcc g++ git make pkg-config cmake \
            libssl-dev protobuf-compiler openssl musl-tools

ADD ./identities .

RUN cargo build -p workload-api-test-client

CMD ./target/debug/workload-api-test-client