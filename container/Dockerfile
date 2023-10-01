FROM rust:latest as Builder
RUN apt-get update && apt-get -y install ca-certificates cmake musl-tools libssl-dev openssl
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /build_dir
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine:3.18
ENV ROUTE_CONF_LOCATION="/etc/endpoint_proxy/config.yaml"
RUN adduser -H -D -g "<endpoint_proxy>" endpoint_proxy
COPY --chmod=111 --chown=root:endpoint_proxy --from=Builder /build_dir/target/x86_64-unknown-linux-musl/release/endpoint_proxy /usr/local/bin
COPY --chmod=644 --chown=root:endpoint_proxy ./config.yaml /etc/endpoint_proxy/config.yaml
COPY --chmod=755 --chown=root:endpoint_proxy ./scripts/endpoint_proxy_server.sh /usr/local/bin
USER endpoint_proxy
CMD ["/usr/local/bin/endpoint_proxy_server.sh"]
