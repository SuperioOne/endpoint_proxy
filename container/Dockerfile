FROM rust:latest as Builder
RUN apt-get update && apt-get -y install ca-certificates cmake musl-tools libssl-dev openssl
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /usr/src
COPY ../Cargo.toml Cargo.lock ./
COPY ../src ./src
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine:3.18
WORKDIR /rss_proxy
ENV ROUTE_CONF_LOCATION="config/route_config.yaml"
COPY --chmod=777 --from=Builder /usr/src/target/x86_64-unknown-linux-musl/release/rss_proxy ./server
COPY ../route_config.yaml config/route_config.yaml
CMD ["./server"]
