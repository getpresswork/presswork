#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: check-linux-packages.sh TARGET LABEL VERSION" >&2
  exit 1
fi

target="$1"
label="$2"
version="$3"
id="io.github.getpresswork.Presswork"
name="dist/presswork-${label//\//-}-${target}"
failures=0

case "${target}" in
  x86_64-unknown-linux-gnu) deb_arch=amd64 rpm_arch=x86_64 ;;
  aarch64-unknown-linux-gnu) deb_arch=arm64 rpm_arch=aarch64 ;;
  *)
    echo "There are no Linux packages for ${target}." >&2
    exit 1
    ;;
esac

fail() {
  echo "Failed: $1" >&2
  failures=$((failures + 1))
}

expect_files() {
  local kind="$1"
  local list="$2"
  shift 2
  for file in "$@"; do
    if ! grep -qx "${file}" <<< "${list}"; then
      fail "the ${kind} package has no ${file}"
    fi
  done
}

files=(
  /usr/bin/presswork
  "/usr/share/applications/${id}.desktop"
  "/usr/share/icons/hicolor/16x16/apps/${id}.png"
  "/usr/share/icons/hicolor/256x256/apps/${id}.png"
  "/usr/share/icons/hicolor/scalable/apps/${id}.svg"
  /usr/share/doc/presswork/LICENSE
)

fields="$(dpkg-deb --show --showformat='${Package} ${Version} ${Architecture}' "${name}.deb")"
if [[ "${fields}" != "presswork ${version} ${deb_arch}" ]]; then
  fail "the DEB package is '${fields}', not 'presswork ${version} ${deb_arch}'"
fi
list="$(dpkg-deb --fsys-tarfile "${name}.deb" | tar -t | sed 's|^\./|/|')"
expect_files DEB "${list}" "${files[@]}"
sudo apt-get install --yes --no-install-recommends "./${name}.deb"
if [[ ! -x /usr/bin/presswork ]]; then
  fail "the DEB package does not install /usr/bin/presswork"
fi
sudo apt-get remove --yes presswork
if [[ -e /usr/bin/presswork ]]; then
  fail "the removal of the DEB package does not remove /usr/bin/presswork"
fi

fields="$(rpm -q --package --queryformat '%{NAME} %{VERSION} %{ARCH}' "${name}.rpm")"
if [[ "${fields}" != "presswork ${version} ${rpm_arch}" ]]; then
  fail "the RPM package is '${fields}', not 'presswork ${version} ${rpm_arch}'"
fi
list="$(rpm -q --package --list "${name}.rpm")"
expect_files RPM "${list}" "${files[@]}"

appimage="$(realpath "${name}.AppImage")"
extracted="$(mktemp -d)"
trap 'rm -rf "${extracted}"' EXIT
(cd "${extracted}" && "${appimage}" --appimage-extract > /dev/null)
for file in AppRun "${id}.desktop" "${id}.png" usr/bin/presswork; do
  if [[ ! -e "${extracted}/squashfs-root/${file}" ]]; then
    fail "the AppImage has no ${file}"
  fi
done

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} Linux package checks failed." >&2
  exit 1
fi
echo "The DEB, RPM, and AppImage packages of Presswork ${version} are correct."
