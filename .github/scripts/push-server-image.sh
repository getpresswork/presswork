#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: push-server-image.sh IMAGE TARGET LABEL" >&2
  exit 1
fi

image="$1"
target="$2"
label="$3"
repository="ghcr.io/getpresswork/presswork-server"

case "${target}" in
  x86_64-unknown-linux-gnu) arch=amd64 ;;
  aarch64-unknown-linux-gnu) arch=arm64 ;;
  *)
    echo "There is no server image for ${target}." >&2
    exit 1
    ;;
esac

mapfile -t tags < <(bash "$(dirname "$0")/server-image-tags.sh" "${label}")
remote="${repository}:${tags[0]}-${arch}"
docker tag "${image}" "${remote}"
docker push "${remote}"
