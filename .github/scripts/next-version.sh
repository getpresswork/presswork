#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "Usage: next-version.sh LAST_TAG REQUESTED_VERSION CARGO_VERSION GIT_CLIFF_VERSION" >&2
  exit 2
fi

last="${1#v}"
requested="${2#v}"
cargo_version="$3"
calculated="${4#v}"
pattern='^[0-9]+\.[0-9]+\.[0-9]+$'

fail() {
  echo "next-version.sh: $1" >&2
  exit 1
}

newer() {
  local version="$1"
  [[ "${version}" =~ ${pattern} ]] || fail "'${version}' does not have the form 1.2.3."
  if [[ -n "${last}" ]]; then
    local highest
    highest=$(printf '%s\n' "${last}" "${version}" | sort -V | tail -n 1)
    [[ "${version}" != "${last}" && "${highest}" == "${version}" ]] || fail "${version} is not greater than ${last}."
  fi
  echo "${version}"
}

if [[ -n "${last}" && ! "${last}" =~ ${pattern} ]]; then
  fail "The last tag v${last} does not have the form v1.2.3."
fi

if [[ -n "${requested}" ]]; then
  newer "${requested}"
elif [[ -z "${last}" ]]; then
  newer "${cargo_version}"
elif [[ "${last%%.*}" == "0" ]]; then
  IFS=. read -r major minor patch <<< "${last}"
  newer "${major}.${minor}.$((patch + 1))"
else
  newer "${calculated}"
fi
