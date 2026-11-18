#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: check-server-archive.sh TARGET LABEL VERSION" >&2
  exit 1
fi

target="$1"
label="$2"
version="$3"
name="presswork-server-${label//\//-}-${target}"
failures=0

fail() {
  echo "Failed: $1" >&2
  failures=$((failures + 1))
}

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT
tar -xzf "dist/${name}.tar.gz" -C "${work}"
folder="${work}/${name}"

for file in presswork-server LICENSE NOTICE server.toml presswork-server.service compose.yaml; do
  if [[ ! -f "${folder}/${file}" ]]; then
    fail "the archive has no ${file}"
  fi
done
output="$("${folder}/presswork-server" --version)"
if [[ "${output}" != "presswork-server ${version}" ]]; then
  fail "the program gives the version '${output}', not 'presswork-server ${version}'"
fi
sudo install -m 755 "${folder}/presswork-server" /usr/local/bin/presswork-server
if ! systemd-analyze verify "${folder}/presswork-server.service"; then
  fail "systemd does not accept the service file"
fi
sudo rm -f /usr/local/bin/presswork-server

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} server archive checks failed." >&2
  exit 1
fi
echo "The server archive of Presswork ${version} is correct."
