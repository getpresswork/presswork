#!/usr/bin/env bash
set -euo pipefail

floor="${COVERAGE_FLOOR:-95}"

cargo llvm-cov --workspace --locked \
  --ignore-filename-regex '[/\\](main|screens|testing)\.rs$' \
  --summary-only --fail-under-lines "$floor"
