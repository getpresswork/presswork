#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: publish-server-image.sh LABEL" >&2
  exit 1
fi

label="$1"
repository="ghcr.io/getpresswork/presswork-server"

mapfile -t tags < <(bash "$(dirname "$0")/server-image-tags.sh" "${label}")
arguments=()
for tag in "${tags[@]}"; do
  arguments+=(--tag "${repository}:${tag}")
done
docker buildx imagetools create "${arguments[@]}" \
  "${repository}:${tags[0]}-amd64" "${repository}:${tags[0]}-arm64"
