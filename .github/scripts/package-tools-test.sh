#!/usr/bin/env bash
set -euo pipefail

script="$(cd "$(dirname "$0")" && pwd)/package-tools.sh"
work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT
failures=0

mkdir -p "${work}/bin"
cat > "${work}/bin/curl" << 'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "${@: -1}" >> "${CURL_LOG}"
while [[ $# -gt 0 ]]; do
  if [[ "$1" == "--output" ]]; then
    printf 'not the tool' > "$2"
  fi
  shift
done
EOF
chmod +x "${work}/bin/curl"
export CURL_LOG="${work}/curl.log"

fail() {
  echo "Failed: $1"
  failures=$((failures + 1))
}

if PATH="${work}/bin:${PATH}" bash "${script}" appimage-x86_64 "${work}/tools" 2> /dev/null; then
  fail "a download with a different digest stops the script"
fi
if [[ -e "${work}/tools/appimagetool" ]]; then
  fail "the script removes a download with a different digest"
fi
if [[ "$(cat "${CURL_LOG}")" != "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage" ]]; then
  fail "the script downloads the pinned appimagetool release for x86_64"
fi

: > "${CURL_LOG}"
mkdir -p "${work}/kept"
printf 'changed' > "${work}/kept/wix.6.0.2.nupkg"
if PATH="${work}/bin:${PATH}" bash "${script}" wix "${work}/kept" 2> /dev/null; then
  fail "a file in the folder with a different digest stops the script"
fi
if [[ -s "${CURL_LOG}" ]]; then
  fail "the script does not download a file that is in the folder"
fi
if [[ -e "${work}/kept/wix.6.0.2.nupkg" ]]; then
  fail "the script removes a file in the folder with a different digest"
fi

if PATH="${work}/bin:${PATH}" bash "${script}" appimage-riscv64 "${work}/tools" 2> /dev/null; then
  fail "an unknown kind of tools stops the script"
fi

if bash "${script}" 2> /dev/null; then
  fail "the script needs a kind and a folder"
fi

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} package tool tests failed."
  exit 1
fi
echo "All package tool tests passed."
