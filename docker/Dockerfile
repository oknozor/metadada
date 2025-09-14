FROM alpine AS arm-builder
COPY ./target/armv7-unknown-linux-musleabihf/release/mbmeta /mbmeta

FROM alpine AS arm64-builder
COPY ./target/aarch64-unknown-linux-gnu/release/mbmeta /mbmeta

FROM alpine AS amd64-builder
COPY ./target/x86_64-unknown-linux-musl/release/mbmeta /mbmeta

FROM ${TARGETARCH}-builder AS builder

FROM alpine
LABEL maintainer="Paul Delafosse <paul.delafosse@protonmail.com>"

RUN addgroup -S mbmeta && adduser -S mbmeta -G mbmeta
USER mbmeta

COPY --from=builder /mbmeta /usr/bin/mbmeta

EXPOSE 3000

COPY ./docker/entrypoint.sh /entrypoint.sh

CMD ["mbmeta"]
ENTRYPOINT ["/entrypoint.sh"]
