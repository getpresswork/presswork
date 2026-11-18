#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "Usage: package-linux.sh TARGET LABEL VERSION APPIMAGE-TOOLS" >&2
  exit 1
fi

target="$1"
label="$2"
version="$3"
tools="$4"
id="io.github.getpresswork.Presswork"
name="presswork-${label//\//-}-${target}"
binary="target/${target}/release/presswork"
icons="crates/presswork/icons"

case "${target}" in
  x86_64-unknown-linux-gnu) deb_arch=amd64 rpm_arch=x86_64 ;;
  aarch64-unknown-linux-gnu) deb_arch=arm64 rpm_arch=aarch64 ;;
  *)
    echo "There are no Linux packages for ${target}." >&2
    exit 1
    ;;
esac

work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT
mkdir -p dist

stage() {
  local root="$1"
  install -D -m 755 "${binary}" "${root}/usr/bin/presswork"
  install -D -m 644 "packaging/linux/${id}.desktop" "${root}/usr/share/applications/${id}.desktop"
  for size in 16 24 32 48 64 128 256 512; do
    install -D -m 644 "${icons}/presswork-${size}.png" \
      "${root}/usr/share/icons/hicolor/${size}x${size}/apps/${id}.png"
  done
  install -D -m 644 "${icons}/presswork.svg" "${root}/usr/share/icons/hicolor/scalable/apps/${id}.svg"
  install -D -m 644 -t "${root}/usr/share/doc/presswork" LICENSE NOTICE README.md
}

summary="Typesetting and publication workbench for Typst"
description="Presswork is a desktop application to write, typeset, and publish books and other publications with Typst."

deb="${work}/deb"
stage "${deb}"
mkdir -p "${work}/shlibs/debian"
printf 'Source: presswork\n\nPackage: presswork\nArchitecture: any\n' > "${work}/shlibs/debian/control"
depends="$(cd "${work}/shlibs" && dpkg-shlibdeps -O -e "${deb}/usr/bin/presswork" | sed -n 's/^shlibs:Depends=//p')"
size="$(du -sk "${deb}" | cut -f 1)"
mkdir -p "${deb}/DEBIAN"
cat > "${deb}/DEBIAN/control" << EOF
Package: presswork
Version: ${version}
Architecture: ${deb_arch}
Maintainer: The Presswork Authors <https://github.com/getpresswork/presswork/issues>
Installed-Size: ${size}
Depends: ${depends}
Recommends: libxkbcommon0, libvulkan1 | libegl1
Section: editors
Priority: optional
Homepage: https://getpresswork.github.io/presswork/
Description: ${summary}
 ${description}
EOF
dpkg-deb --root-owner-group --build "${deb}" "dist/${name}.deb"

rpm_root="${work}/rpm"
stage "${rpm_root}"
cat > "${work}/presswork.spec" << EOF
Name: presswork
Version: ${version}
Release: 1
Summary: ${summary}
License: Apache-2.0
URL: https://getpresswork.github.io/presswork/
%global debug_package %{nil}
%global __os_install_post %{nil}
%global _build_id_links none

%description
${description}

%install
cp -a ${rpm_root}/. %{buildroot}/

%files
/usr/bin/presswork
/usr/share/applications/${id}.desktop
/usr/share/icons/hicolor/*/apps/${id}.*
/usr/share/doc/presswork
EOF
rpmbuild -bb --target "${rpm_arch}" --define "_topdir ${work}/rpmbuild" "${work}/presswork.spec"
cp "${work}/rpmbuild/RPMS/${rpm_arch}/presswork-${version}-1.${rpm_arch}.rpm" "dist/${name}.rpm"

appdir="${work}/AppDir"
stage "${appdir}"
ln -s usr/bin/presswork "${appdir}/AppRun"
cp "packaging/linux/${id}.desktop" "${appdir}/${id}.desktop"
cp "${icons}/presswork-256.png" "${appdir}/${id}.png"
ln -s "${id}.png" "${appdir}/.DirIcon"
ARCH="${rpm_arch}" "${tools}/appimagetool" --appimage-extract-and-run --no-appstream \
  --runtime-file "${tools}/runtime" "${appdir}" "dist/${name}.AppImage"
