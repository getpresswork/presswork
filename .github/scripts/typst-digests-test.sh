#!/usr/bin/env bash
set -euo pipefail

script="$(cd "$(dirname "$0")" && pwd)/typst-digests.sh"
work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT
failures=0

mkdir -p "${work}/bin"
cat > "${work}/bin/gh" << 'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "$1 $2" in
  "release download")
    while [[ $# -gt 0 ]]; do
      if [[ "$1" == "--dir" ]]; then
        printf 'zip' > "$2/typst-x86_64-pc-windows-msvc.zip"
        printf 'xz' > "$2/typst-x86_64-unknown-linux-musl.tar.xz"
      fi
      shift
    done
    ;;
  "api repos/typst/typst/releases/tags/"*)
    printf 'typst-x86_64-pc-windows-msvc.zip %s\n' "${ZIP_SIZE:-3}"
    printf 'typst-x86_64-unknown-linux-musl.tar.xz 2\n'
    printf 'typst-documentation.pdf 9\n'
    ;;
esac
EOF
chmod +x "${work}/bin/gh"

zip_digest="$(printf 'zip' | sha256sum | cut -d ' ' -f 1)"
xz_digest="$(printf 'xz' | sha256sum | cut -d ' ' -f 1)"

expect() {
  local description="$1"
  local expected="$2"
  local actual
  actual="$(grep -v '^#' "${work}/digests.txt" 2> /dev/null || true)"
  if [[ "${actual}" != "${expected}" ]]; then
    echo "Failed: ${description}. Got '${actual}', not '${expected}'."
    failures=$((failures + 1))
  fi
}

PATH="${work}/bin:${PATH}" bash "${script}" "${work}/digests.txt" 0.12.0 0.13.1
expect "each release file gets a line with its version and its digest" "0.12.0 typst-x86_64-pc-windows-msvc.zip ${zip_digest}
0.12.0 typst-x86_64-unknown-linux-musl.tar.xz ${xz_digest}
0.13.1 typst-x86_64-pc-windows-msvc.zip ${zip_digest}
0.13.1 typst-x86_64-unknown-linux-musl.tar.xz ${xz_digest}"

if grep -q '^# version file sha256$' "${work}/digests.txt"; then :; else
  echo "Failed: the list starts with the header of digests.txt."
  failures=$((failures + 1))
fi

rm -f "${work}/digests.txt"
if PATH="${work}/bin:${PATH}" ZIP_SIZE=4 bash "${script}" "${work}/digests.txt" 0.13.1 2> /dev/null; then
  echo "Failed: a file with a size that GitHub does not report stops the script."
  failures=$((failures + 1))
fi

if bash "${script}" 2> /dev/null; then
  echo "Failed: the script needs an output file and a version."
  failures=$((failures + 1))
fi

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} digest tests failed."
  exit 1
fi
echo "All digest tests passed."
