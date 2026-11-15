#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: typst-digests.sh OUTPUT VERSION..." >&2
  exit 1
fi

output="$1"
shift
work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT

{
  echo "# SHA-256 digests of the Typst release files that GitHub publishes without a digest."
  echo "# The \"Typst digests\" workflow calculates these lines. Refer to docs/src/design/toolchains.md."
  echo "# version file sha256"
  for version in "$@"; do
    folder="${work}/${version}"
    mkdir -p "${folder}"
    gh release download "v${version}" --repo typst/typst --dir "${folder}" \
      --pattern 'typst-*.zip' --pattern 'typst-*.tar.xz'
    sizes="$(gh api "repos/typst/typst/releases/tags/v${version}" --jq '.assets[] | "\(.name) \(.size)"')"
    for file in "${folder}"/*; do
      name="$(basename "${file}")"
      published="$(awk -v name="${name}" '$1 == name { print $2 }' <<< "${sizes}")"
      actual="$(wc -c < "${file}" | tr -d ' ')"
      if [[ "${published}" != "${actual}" ]]; then
        echo "The size of ${name} ${version} is ${actual}, but GitHub reports ${published}." >&2
        exit 1
      fi
      echo "${version} ${name} $(sha256sum "${file}" | cut -d ' ' -f 1)"
    done
  done
} > "${output}"
