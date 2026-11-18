#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: package-tools.sh KIND FOLDER" >&2
  exit 1
fi

kind="$1"
folder="$2"
found=false
mkdir -p "${folder}"

while read -r tools name url digest; do
  if [[ "${tools}" != "${kind}" ]]; then
    continue
  fi
  found=true
  file="${folder}/${name}"
  if [[ ! -f "${file}" ]]; then
    curl --fail --silent --show-error --location --proto '=https' --output "${file}" "${url}"
  fi
  if ! echo "${digest#*:}  ${file}" | "${digest%%:*}sum" --check --quiet; then
    rm -f "${file}"
    echo "The digest of ${name} is not ${digest}." >&2
    exit 1
  fi
  chmod +x "${file}"
done <<'TOOLS'
appimage-x86_64 appimagetool https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage sha256:ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
appimage-x86_64 runtime https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64 sha256:2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
appimage-aarch64 appimagetool https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-aarch64.AppImage sha256:f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
appimage-aarch64 runtime https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-aarch64 sha256:00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444
wix wix.6.0.2.nupkg https://api.nuget.org/v3-flatcontainer/wix/6.0.2/wix.6.0.2.nupkg sha512:b4536ece74dc7ea01d5c809813456914f650d02607a70feec0875742d34756514673016659317788410bc54d47aeba1b131f1367accc7f2ae81bd07fe43560e6
wix wixtoolset.ui.wixext.6.0.2.nupkg https://api.nuget.org/v3-flatcontainer/wixtoolset.ui.wixext/6.0.2/wixtoolset.ui.wixext.6.0.2.nupkg sha512:769af5c9387939370d94b2a26b5a372560f6c14dc29df147cee03714ac916d1198def3083e4bdd961d82c7b0414465d53c1d85cfd0654456a5b48d067f7b3cc0
TOOLS

if [[ "${found}" != true ]]; then
  echo "There are no tools of the kind ${kind}." >&2
  exit 1
fi
