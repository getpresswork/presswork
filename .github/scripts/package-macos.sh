#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: package-macos.sh TARGET LABEL VERSION" >&2
  exit 1
fi

target="$1"
label="$2"
version="$3"
name="presswork-${label//\//-}-${target}"

case "${target}" in
  aarch64-apple-darwin | x86_64-apple-darwin) ;;
  *)
    echo "There is no DMG file for ${target}." >&2
    exit 1
    ;;
esac

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT
app="${work}/image/Presswork.app"
mkdir -p "${app}/Contents/MacOS" "${app}/Contents/Resources" dist

cp "target/${target}/release/presswork" "${app}/Contents/MacOS/presswork"
cp crates/presswork/icons/presswork.icns LICENSE NOTICE "${app}/Contents/Resources/"
sed "s/@VERSION@/${version}/g" packaging/macos/Info.plist > "${app}/Contents/Info.plist"
plutil -lint "${app}/Contents/Info.plist"
codesign --force --sign - "${app}"
ln -s /Applications "${work}/image/Applications"

for attempt in 1 2 3 4 5; do
  if hdiutil create -volname Presswork -srcfolder "${work}/image" -fs HFS+ -format UDZO -ov "dist/${name}.dmg"; then
    exit 0
  fi
  echo "Attempt ${attempt} of hdiutil did not complete." >&2
  sleep 10
done
exit 1
