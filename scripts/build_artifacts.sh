#!/bin/bash

TARGET=output
TEMP_LOCATION="/tmp/build_artifacts"
BINARY_NAME="endpoint_proxy"

install -D "$TEMP_LOCATION"
cp -r . "$TEMP_LOCATION"

if [ "$LINUX_MUSL_x86_64" = true ]; then
  install -d "$TARGET/linux_musl_x86_64"
  echo "Building for linux_musl_x86_64";
  cargo build --manifest-path "$TEMP_LOCATION/Cargo.toml" --target x86_64-unknown-linux-musl --profile release && \
  cp "$TEMP_LOCATION/target/x86_64-unknown-linux-musl/release/$BINARY_NAME" "$TARGET/linux_musl_x86_64/"
fi
if [ "$LINUX_AARCH64" = true ]; then
  install -d "$TARGET/linux_aarch64"
  echo "Building for linux_aarch64";
  cargo build --manifest-path "$TEMP_LOCATION/Cargo.toml" --target aarch64-unknown-linux-musl --profile release && \
  cp "$TEMP_LOCATION/target/aarch64-unknown-linux-musl/release/$BINARY_NAME" "$TARGET/linux_aarch64/"
fi
if [ "$LINUX_x86_64" = true ]; then
  install -d "$TARGET/linux_x86_64"
  echo "Building linux_X86_64";
  cargo build --manifest-path "$TEMP_LOCATION/Cargo.toml" --target x86_64-unknown-linux-gnu --profile release && \
  cp "$TEMP_LOCATION/target/x86_64-unknown-linux-gnu/release/$BINARY_NAME" "$TARGET/linux_x86_64/"
fi
if [ "$WIN_x86_64" = true ]; then
  install -d "$TARGET/win_x86_64"
  echo "Building win_x86_64";
  cargo build --manifest-path "$TEMP_LOCATION/Cargo.toml" --target x86_64-pc-windows-gnu --profile release && \
  cp "$TEMP_LOCATION/target/x86_64-pc-windows-gnu/release/$BINARY_NAME.exe" "$TARGET/win_x86_64/"
fi

