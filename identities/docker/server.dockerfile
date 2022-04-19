FROM builder as builder

FROM alpine:3.15

COPY --from=builder ./target/x86_64-unknown-linux-musl/debug/serverd .

CMD ./serverd