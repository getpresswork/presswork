#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: check-server-image.sh IMAGE VERSION" >&2
  exit 1
fi

image="$1"
version="$2"
compose=(docker compose --file packaging/server/compose.yaml --project-name presswork-check)
failures=0

fail() {
  echo "Failed: $1" >&2
  failures=$((failures + 1))
}

output="$(docker run --rm "${image}" --version)"
if [[ "${output}" != "presswork-server ${version}" ]]; then
  fail "the image gives the version '${output}', not 'presswork-server ${version}'"
fi
user="$(docker run --rm --entrypoint id "${image}" -u)"
if [[ "${user}" != 10001 ]]; then
  fail "the server runs as the user ${user}, not 10001"
fi
group="$(docker run --rm --entrypoint id "${image}" -g)"
if [[ "${group}" != 10001 ]]; then
  fail "the server runs with the group ${group}, not 10001"
fi
if ! docker run --rm --entrypoint ocrmypdf "${image}" --version > /dev/null; then
  fail "OCRmyPDF does not run in the image"
fi
languages="$(docker run --rm --entrypoint tesseract "${image}" --list-langs 2>&1)"
if ! grep -qx eng <<< "${languages}"; then
  fail "Tesseract has no English data in the image"
fi

finish() {
  local status=$?
  if [[ "${status}" -ne 0 ]]; then
    "${compose[@]}" logs || true
  fi
  "${compose[@]}" down --volumes > /dev/null 2>&1 || true
}

export PRESSWORK_IMAGE="${image}"
export PRESSWORK_DATABASE_PASSWORD=check-password
export PRESSWORK_PORT=127.0.0.1:18080
trap finish EXIT
"${compose[@]}" up --detach --wait

ready=""
for _ in $(seq 1 60); do
  if ready="$(curl --fail --silent http://127.0.0.1:18080/api/v1/health/ready)"; then
    break
  fi
  ready=""
  sleep 1
done
if [[ "${ready}" != "{\"status\":\"ok\",\"version\":\"${version}\"}" ]]; then
  fail "the ready check answers '${ready}'"
fi

invite="$("${compose[@]}" exec -T server presswork-server create-organization \
  --name "Check Press" --administrator ann@example.com)"
if ! grep -q "pwi_" <<< "${invite}"; then
  fail "create-organization does not print an invite link"
fi
logs="$("${compose[@]}" logs server)"
if ! grep -q "OCR uses OCRmyPDF" <<< "${logs}"; then
  fail "the server in the image does not use OCRmyPDF"
fi

container="$("${compose[@]}" ps --quiet server)"
start="$(date +%s)"
"${compose[@]}" stop --timeout 15 server
seconds=$(($(date +%s) - start))
code="$(docker inspect --format '{{.State.ExitCode}}' "${container}")"
if [[ "${code}" != 0 ]]; then
  fail "the server stops with the exit code ${code}, not 0"
fi
if ((seconds >= 10)); then
  fail "the server needs ${seconds} seconds to stop"
fi

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} server image checks failed." >&2
  exit 1
fi
echo "The server image of Presswork ${version} is correct."
