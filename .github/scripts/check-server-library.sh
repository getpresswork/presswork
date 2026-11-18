#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: check-server-library.sh API INVITE SCAN" >&2
  exit 1
fi

api="$1"
invite="$2"
scan="$3"

call() {
  curl --fail-with-body --silent --show-error "$@"
}

joined="$(call --request POST --header "Content-Type: application/json" \
  --data '{"name":"Ann Check","password":"a long password for the check","device":"check"}' \
  "${api}/invites/${invite}/accept")"
token="$(jq --raw-output .token <<< "${joined}")"
organization="$(jq --raw-output .organization_id <<< "${joined}")"
authorization=(--header "Authorization: Bearer ${token}")

work="$(call "${authorization[@]}" --request POST --header "Content-Type: application/json" \
  --data '{"key":"check","title":"Check scan","edition":{"language":"en"}}' \
  "${api}/organizations/${organization}/works")"
edition="$(jq --raw-output '.editions[0].id' <<< "${work}")"
call "${authorization[@]}" --request PUT --header "Content-Type: application/pdf" \
  --data-binary "@${scan}" "${api}/editions/${edition}/file" > /dev/null

file="null"
for _ in $(seq 1 180); do
  file="$(call "${authorization[@]}" "${api}/organizations/${organization}/works" \
    | jq --compact-output '.[0].editions[0].file')"
  state="$(jq --raw-output .state <<< "${file}")"
  if [[ "${state}" == ready || "${state}" == failed ]]; then
    break
  fi
  sleep 1
done
if [[ "$(jq --raw-output '"\(.state) \(.problem)"' <<< "${file}")" != "ready null" ]]; then
  echo "Failed: OCR of the scan gives ${file}" >&2
  exit 1
fi

hits="$(call "${authorization[@]}" "${api}/organizations/${organization}/library/search?q=paper")"
if ! jq --exit-status 'length > 0' <<< "${hits}" > /dev/null; then
  echo "Failed: the library search does not find the text of the scan" >&2
  exit 1
fi
echo "OCR gives the text of the scan: $(jq --raw-output '.[0].snippet' <<< "${hits}")"
