FROM builder as builder

FROM alpine:3.15
RUN apk update && apk add openssl 

COPY --from=builder ./target/release/agentd  .

CMD ./agentd