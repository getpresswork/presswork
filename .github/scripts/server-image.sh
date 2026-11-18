#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: server-image.sh TARGET VERSION IMAGE" >&2
  exit 1
fi

target="$1"
version="$2"
image="$3"

case "${target}" in
  x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu) ;;
  *)
    echo "There is no server image for ${target}." >&2
    exit 1
    ;;
esac

context="$(mktemp -d)"
trap 'rm -rf "${context}"' EXIT
cp "target/${target}/release/presswork-server" LICENSE NOTICE packaging/server/Dockerfile "${context}/"
docker build \
  --tag "${image}" \
  --label "org.opencontainers.image.version=${version}" \
  --label "org.opencontainers.image.revision=$(git rev-parse HEAD)" \
  --label "org.opencontainers.image.created=$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  "${context}"
