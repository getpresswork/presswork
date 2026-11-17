#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: ocrs-models.sh FOLDER" >&2
  exit 1
fi

folder="$1"
host="https://ocrs-models.s3-accelerate.amazonaws.com"
mkdir -p "${folder}"

while read -r name digest; do
  file="${folder}/${name}"
  if [[ ! -f "${file}" ]]; then
    curl --fail --silent --show-error --location --proto '=https' --output "${file}" "${host}/${name}"
  fi
  if ! echo "${digest}  ${file}" | sha256sum --check --quiet; then
    rm -f "${file}"
    echo "The digest of ${name} is not ${digest}." >&2
    exit 1
  fi
done <<'MODELS'
text-detection.onnx a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe
text-recognition.onnx 86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5
MODELS
