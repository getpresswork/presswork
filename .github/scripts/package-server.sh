#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: package-server.sh TARGET LABEL" >&2
  exit 1
fi

target="$1"
label="$2"
name="presswork-server-${label//\//-}-${target}"

case "${target}" in
  x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu) ;;
  *)
    echo "There is no server archive for ${target}." >&2
    exit 1
    ;;
esac

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT
mkdir -p "${work}/${name}" dist
cp "target/${target}/release/presswork-server" LICENSE NOTICE \
  packaging/server/server.toml packaging/server/presswork-server.service packaging/server/compose.yaml \
  "${work}/${name}/"
tar -czf "dist/${name}.tar.gz" -C "${work}" "${name}"
