# Stage: Builder
FROM docker.io/rust:latest as BUILDER
ARG RUST_TOOLCHAIN
RUN apt-get update && \
    apt-get -y install ca-certificates cmake musl-tools libssl-dev openssl gcc-aarch64-linux-gnu clang llvm libc6-dev-arm64-cross && \
    rustup target add "$RUST_TOOLCHAIN"
WORKDIR /build_dir
COPY Cargo.toml Cargo.lock ./
COPY ./src ./src
COPY ./.cargo ./.cargo
RUN cargo fetch --target "$RUST_TOOLCHAIN"
RUN mkdir /build_dir/output && \
    cargo build --target "$RUST_TOOLCHAIN" --release && \
    cp "/build_dir/target/$RUST_TOOLCHAIN/release/endpoint_proxy" /build_dir/output

# Stage: Main image
ARG TARGETPLATFORM
ARG ALPINE_TAG
FROM --platform=${TARGETPLATFORM} docker.io/alpine:${ALPINE_TAG}
ENV ROUTE_CONF_LOCATION="/etc/endpoint_proxy/config.yaml"
RUN adduser -H -D -g "<endpoint_proxy>" endpoint_proxy
COPY --chmod=111 --chown=root:endpoint_proxy --from=BUILDER /build_dir/output/endpoint_proxy /usr/local/bin
COPY --chmod=664 --chown=root:endpoint_proxy ./config.yaml /etc/endpoint_proxy/config.yaml
COPY --chmod=551 --chown=root:endpoint_proxy endpoint_proxy_server.sh /usr/local/bin
USER endpoint_proxy
CMD ["/usr/local/bin/endpoint_proxy_server.sh"]
