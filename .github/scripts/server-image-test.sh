#!/usr/bin/env bash
set -euo pipefail

folder="$(cd "$(dirname "$0")" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT
failures=0

mkdir -p "${work}/bin"
cat > "${work}/bin/docker" << 'EOF'
#!/usr/bin/env bash
echo "docker $*" >> "${DOCKER_LOG}"
EOF
chmod +x "${work}/bin/docker"
export DOCKER_LOG="${work}/docker.log"
repository="ghcr.io/getpresswork/presswork-server"

expect() {
  local description="$1"
  local expected="$2"
  local actual="$3"
  if [[ "${actual}" != "${expected}" ]]; then
    echo "Failed: ${description}. Got '${actual}', not '${expected}'."
    failures=$((failures + 1))
  fi
}

tags() {
  bash "${folder}/server-image-tags.sh" "$1" | paste -sd ' ' -
}

expect "a release gets its version, its minor version, and latest" "0.1.0 0.1 latest" "$(tags v0.1.0)"
expect "a later release gets the same three tags" "1.12.3 1.12 latest" "$(tags v1.12.3)"
expect "a prerelease version gets only its version" "1.0.0-rc.1" "$(tags v1.0.0-rc.1)"
expect "a nightly build gets the tag nightly" "nightly" "$(tags nightly)"
expect "a label with a slash gets a hyphen" "feature-image" "$(tags feature/image)"

: > "${DOCKER_LOG}"
PATH="${work}/bin:${PATH}" bash "${folder}/push-server-image.sh" presswork-server:check aarch64-unknown-linux-gnu v0.1.0
expect "the push tags the checked image for its architecture" "docker tag presswork-server:check ${repository}:0.1.0-arm64
docker push ${repository}:0.1.0-arm64" "$(cat "${DOCKER_LOG}")"

: > "${DOCKER_LOG}"
PATH="${work}/bin:${PATH}" bash "${folder}/push-server-image.sh" presswork-server:check x86_64-unknown-linux-gnu nightly
expect "an x86_64 image gets the architecture amd64" "docker tag presswork-server:check ${repository}:nightly-amd64
docker push ${repository}:nightly-amd64" "$(cat "${DOCKER_LOG}")"

: > "${DOCKER_LOG}"
PATH="${work}/bin:${PATH}" bash "${folder}/publish-server-image.sh" v0.1.0
expect "the publication joins the two architectures under all tags" "docker buildx imagetools create --tag ${repository}:0.1.0 --tag ${repository}:0.1 --tag ${repository}:latest ${repository}:0.1.0-amd64 ${repository}:0.1.0-arm64" "$(cat "${DOCKER_LOG}")"

if PATH="${work}/bin:${PATH}" bash "${folder}/push-server-image.sh" presswork-server:check x86_64-pc-windows-msvc v0.1.0 2> /dev/null; then
  echo "Failed: a target without a server image stops the push."
  failures=$((failures + 1))
fi

for script in server-image-tags.sh push-server-image.sh publish-server-image.sh; do
  if bash "${folder}/${script}" 2> /dev/null; then
    echo "Failed: ${script} needs its arguments."
    failures=$((failures + 1))
  fi
done

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} server image tests failed."
  exit 1
fi
echo "All server image tests passed."
