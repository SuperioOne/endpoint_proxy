#!/bin/bash
set -e;

PKGID=$(cargo pkgid | awk -F '#' '{print $2}')
VERSION=$(awk -F '@' '{print $2}' <<< "$PKGID")
NAME=$(awk -F '@' '{print $1}' <<< "$PKGID")

if [ "$UID" -eq 0 ]; then
  buildah build -f container/Dockerfile -t "$NAME:latest" -t "$NAME:musl-$VERSION"
else
  sudo buildah build -f container/Dockerfile -t "$NAME:latest" -t "$NAME:musl-$VERSION"
fi
