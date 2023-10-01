#!/bin/bash
set -e;

VERSION=$(cargo pkgid | awk -F '#' '{print $2}')
NAME="endpoint_proxy"

echo "Building image as '$NAME:musl-$VERSION' and '$NAME:latest'."
if [ "$UID" -eq 0 ]; then
  buildah build --layers -f container/Dockerfile -t "$NAME:latest" -t "$NAME:musl-$VERSION" ./
else
  sudo buildah build --layers -f container/Dockerfile -t "$NAME:latest" -t "$NAME:musl-$VERSION" ./
fi
