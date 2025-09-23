FROM alpine AS arm-builder
COPY ./target/armv7-unknown-linux-musleabihf/release/metadada /metadada

FROM alpine AS arm64-builder
COPY ./target/aarch64-unknown-linux-gnu/release/metadada /metadada

FROM alpine AS amd64-builder
COPY ./target/x86_64-unknown-linux-musl/release/metadada /metadada

FROM ${TARGETARCH}-builder AS builder

FROM alpine
LABEL maintainer="Paul Delafosse <paul.delafosse@protonmail.com>"

RUN addgroup -S metadada && adduser -S metadada -G metadada
USER metadada

COPY --from=builder /metadada /usr/bin/metadada

EXPOSE 3000

COPY ./docker/entrypoint.sh /entrypoint.sh

CMD ["metadada"]
ENTRYPOINT ["/entrypoint.sh"]
