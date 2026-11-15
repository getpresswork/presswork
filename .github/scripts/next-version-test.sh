#!/usr/bin/env bash
set -euo pipefail

script="$(dirname "$0")/next-version.sh"
failures=0

expect() {
  local description="$1"
  local expected="$2"
  shift 2
  local actual
  actual=$(bash "${script}" "$@" 2> /dev/null) || actual="error"
  if [[ "${actual}" != "${expected}" ]]; then
    echo "Failed: ${description}. Got '${actual}', not '${expected}'."
    failures=$((failures + 1))
  fi
}

expect "the first release uses the version in Cargo.toml" 0.1.0 "" "" 0.1.0 0.1.0
expect "a release before 1.0 bumps only the patch" 0.1.1 v0.1.0 "" 0.1.0 0.2.0
expect "the patch bump is numeric" 0.1.10 v0.1.9 "" 0.1.9 0.2.0
expect "a requested version starts a milestone" 0.2.0 v0.1.4 0.2.0 0.1.4 0.1.5
expect "a requested version can start with v" 0.2.0 v0.1.4 v0.2.0 0.1.4 0.1.5
expect "a requested version is compared as numbers" 0.1.10 v0.1.9 0.1.10 0.1.9 0.1.10
expect "a requested version can set the first release" 0.1.0 "" 0.1.0 0.1.0 0.1.0
expect "a release after 1.0 uses the version from git-cliff" 1.3.0 v1.2.5 "" 1.2.5 v1.3.0
expect "a release after 1.0 needs the version from git-cliff" error v1.2.5 "" 1.2.5 ""
expect "a requested version cannot repeat the last release" error v0.1.4 0.1.4 0.1.4 0.1.5
expect "a requested version cannot go back" error v0.1.4 0.1.3 0.1.4 0.1.5
expect "a requested version has three numbers" error v0.1.4 0.2 0.1.4 0.1.5
expect "a requested version contains only numbers" error v0.1.4 latest 0.1.4 0.1.5
expect "the first release needs a version in Cargo.toml" error "" "" "" 0.1.0
expect "the last tag must be a version" error v0.1 "" 0.1.0 0.2.0
expect "the script needs four arguments" error v0.1.0

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} version tests failed."
  exit 1
fi
echo "All version tests passed."
