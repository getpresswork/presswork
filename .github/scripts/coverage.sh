#!/usr/bin/env bash
# The floor goes up step by step to 95 (D216).
set -euo pipefail

floor="${COVERAGE_FLOOR:-84}"

cargo llvm-cov --workspace --locked \
  --ignore-filename-regex '[/\\](main|screens|testing)\.rs$' \
  --summary-only --fail-under-lines "$floor"
