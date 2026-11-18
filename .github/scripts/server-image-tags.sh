#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: server-image-tags.sh LABEL" >&2
  exit 1
fi

label="$1"
if [[ "${label}" =~ ^v([0-9]+)\.([0-9]+)\.[0-9]+$ ]]; then
  echo "${label#v}"
  echo "${BASH_REMATCH[1]}.${BASH_REMATCH[2]}"
  echo latest
elif [[ "${label}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+- ]]; then
  echo "${label#v}"
else
  echo "${label//\//-}"
fi
