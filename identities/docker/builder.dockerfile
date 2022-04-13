FROM rust:alpine
RUN apk update && apk upgrade && apk add curl gcc git make pkgconfig openssl-dev bash musl-dev protobuf && LIBC="musl"

ADD ./identities .

RUN cargo update -p serverd -p agentd

RUN cargo build -p serverd -p agentd --release
