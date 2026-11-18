#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: check-macos-package.sh TARGET LABEL VERSION" >&2
  exit 1
fi

target="$1"
label="$2"
version="$3"
dmg="dist/presswork-${label//\//-}-${target}.dmg"
failures=0

case "${target}" in
  aarch64-apple-darwin) arch=arm64 ;;
  x86_64-apple-darwin) arch=x86_64 ;;
  *)
    echo "There is no DMG file for ${target}." >&2
    exit 1
    ;;
esac

fail() {
  echo "Failed: $1" >&2
  failures=$((failures + 1))
}

expect_key() {
  local key="$1"
  local expected="$2"
  local actual
  actual="$(plutil -extract "${key}" raw "${app}/Contents/Info.plist")"
  if [[ "${actual}" != "${expected}" ]]; then
    fail "${key} is '${actual}', not '${expected}'"
  fi
}

mount="$(mktemp -d)"
hdiutil attach -nobrowse -readonly -mountpoint "${mount}" "${dmg}" > /dev/null
trap 'hdiutil detach "${mount}" > /dev/null || true' EXIT
app="${mount}/Presswork.app"

if [[ ! -L "${mount}/Applications" ]]; then
  fail "the DMG file has no link to the Applications folder"
fi
expect_key CFBundleIdentifier io.github.getpresswork.Presswork
expect_key CFBundleExecutable presswork
expect_key CFBundleIconFile presswork
expect_key CFBundleShortVersionString "${version}"
if [[ "$(lipo -archs "${app}/Contents/MacOS/presswork")" != "${arch}" ]]; then
  fail "the program is not for ${arch}"
fi
if [[ ! -f "${app}/Contents/Resources/presswork.icns" ]]; then
  fail "the app has no icon file"
fi
if ! codesign --verify --strict "${app}"; then
  fail "the signature of the app is not valid"
fi

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} macOS package checks failed." >&2
  exit 1
fi
echo "The DMG file of Presswork ${version} is correct."
