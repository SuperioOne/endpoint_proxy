#!/bin/bash

TARGET=output
BINARY_NAME="rss_proxy"

if [ "$MUSL" = true ]; then
  install -d "$TARGET/musl"
  echo "Building MUSL";
  cargo build --target x86_64-unknown-linux-musl --profile release
  cp "target/x86_64-unknown-linux-musl/release/$BINARY_NAME" "$TARGET/musl/"
fi
if [ "$AARCH64" = true ]; then
  install -d "$TARGET/aarch64"
  echo "Building AARCH64";
  cargo build --target aarch64-unknown-linux-gnu --profile release
  cp "target/aarch64-unknown-linux-gnu/release/$BINARY_NAME" "$TARGET/aarch64/"
fi
if [ "$X86_64" = true ]; then
  install -d "$TARGET/x86_64"
  echo "Building X86_64";
  cargo build --target x86_64-unknown-linux-gnu --profile release
  cp "target/x86_64-unknown-linux-gnu/release/$BINARY_NAME" "$TARGET/x86_64/"
fi

