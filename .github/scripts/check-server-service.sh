#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: check-server-service.sh TARGET LABEL VERSION" >&2
  exit 1
fi

target="$1"
label="$2"
version="$3"
name="presswork-server-${label//\//-}-${target}"
failures=0

fail() {
  echo "Failed: $1" >&2
  failures=$((failures + 1))
}

repository="$(pwd)"
work="$(mktemp -d)"
finish() {
  local status=$?
  if [[ "${status}" -ne 0 || "${failures}" -ne 0 ]]; then
    sudo journalctl --unit presswork-server --no-pager || true
  fi
  sudo systemctl disable --now presswork-server > /dev/null 2>&1 || true
  rm -rf "${work}"
}
trap finish EXIT

tar -xzf "dist/${name}.tar.gz" -C "${work}"
cd "${work}/${name}"

packages=(ocrmypdf tesseract-ocr-eng)
if ! command -v pg_lsclusters > /dev/null; then
  packages+=(postgresql)
fi
sudo apt-get install --yes --no-install-recommends "${packages[@]}"
sudo systemctl start postgresql
sudo -u postgres psql --quiet --command "CREATE ROLE presswork LOGIN PASSWORD 'check-password'"
sudo -u postgres createdb --owner presswork presswork

sudo useradd --system --user-group --home-dir /var/lib/presswork --no-create-home --shell /usr/sbin/nologin presswork
sudo install --mode 755 presswork-server /usr/local/bin/
sudo install -D --mode 640 --group presswork server.toml /etc/presswork/server.toml
sudo sed -i 's/change-this-password/check-password/' /etc/presswork/server.toml
sudo install --mode 644 presswork-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now presswork-server

ready=""
for _ in $(seq 1 60); do
  if ready="$(curl --fail --silent http://127.0.0.1:8080/api/v1/health/ready)"; then
    break
  fi
  ready=""
  sleep 1
done
if [[ "${ready}" != "{\"status\":\"ok\",\"version\":\"${version}\"}" ]]; then
  fail "the ready check of the service answers '${ready}'"
fi

pid="$(systemctl show --property MainPID --value presswork-server)"
uid="$(ps -o uid= -p "${pid}" | tr -d ' ')"
if [[ "${uid}" != "$(id -u presswork)" ]]; then
  fail "the service runs as the user ID '${uid}', not as the user presswork"
fi
owner="$(stat -c %U /var/lib/presswork)"
if [[ "${owner}" != presswork ]]; then
  fail "the data folder belongs to '${owner}', not presswork"
fi

invite="$(sudo -u presswork PRESSWORK_CONFIG=/etc/presswork/server.toml presswork-server create-organization \
  --name "Check Press" --administrator ann@example.com)"
if ! grep -q "pwi_" <<< "${invite}"; then
  fail "create-organization does not print an invite link"
fi
logs="$(sudo journalctl --unit presswork-server --no-pager)"
if ! grep -q "OCR uses OCRmyPDF" <<< "${logs}"; then
  fail "the service does not use OCRmyPDF"
fi
gs -q -dSAFER -dBATCH -dNOPAUSE -sDEVICE=pdfimage24 -r200 -sOutputFile="${work}/scan.pdf" \
  "${repository}/crates/presswork-ocr/fixtures/typst.pdf"
token="$(grep -o 'pwi_[^[:space:]]*' <<< "${invite}" || true)"
if ! bash "${repository}/.github/scripts/check-server-library.sh" http://127.0.0.1:8080/api/v1 "${token}" "${work}/scan.pdf"; then
  fail "OCR in the service does not give the text of a scanned page"
fi

sudo systemctl stop presswork-server
code="$(systemctl show --property ExecMainStatus --value presswork-server)"
if [[ "${code}" != 0 ]]; then
  fail "the service stops with the exit code ${code}, not 0"
fi

if [[ "${failures}" -ne 0 ]]; then
  echo "${failures} server service checks failed." >&2
  exit 1
fi
echo "The server service of Presswork ${version} is correct."
