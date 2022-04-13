FROM rust:alpine as builder

RUN apk update && apk upgrade && apk add curl gcc git make pkgconfig openssl-dev bash musl-dev protobuf && LIBC="musl"

ADD ./identities .

RUN cargo update -p serverd

RUN cargo build -p serverd 



FROM alpine:3.15
RUN apk update && apk add openssl 

COPY --from=builder ./target/debug/serverd  .

CMD ./serverd