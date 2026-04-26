#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/app-nap"
SYSTEMD_USER_DIR="${HOME}/.config/systemd/user"
BIN_PATH="${BIN_DIR}/app-nap"
SERVICE_PATH="${SYSTEMD_USER_DIR}/app-nap.service"
CONFIG_PATH="${CONFIG_DIR}/app-nap.toml"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'missing required command: %s\n' "$1" >&2
    exit 1
  fi
}

need_cmd cargo
need_cmd kpackagetool6
need_cmd kwriteconfig6
need_cmd systemctl

cargo build --release --manifest-path "${ROOT_DIR}/Cargo.toml"

mkdir -p "${BIN_DIR}" "${CONFIG_DIR}" "${SYSTEMD_USER_DIR}"
install -m 0755 "${ROOT_DIR}/target/release/app-nap" "${BIN_PATH}"

if [[ ! -f "${CONFIG_PATH}" ]]; then
  install -m 0644 "${ROOT_DIR}/example/app-nap.toml" "${CONFIG_PATH}"
fi

cat >"${SERVICE_PATH}" <<EOF
[Unit]
Description=App Nap daemon
After=graphical-session.target

[Service]
Type=simple
ExecStart=${BIN_PATH}
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now app-nap.service

if kpackagetool6 --type=KWin/Script --list 2>/dev/null | grep -q 'appnap'; then
  kpackagetool6 --type=KWin/Script -u "${ROOT_DIR}/kwin-appnap"
else
  kpackagetool6 --type=KWin/Script -i "${ROOT_DIR}/kwin-appnap"
fi

kwriteconfig6 --file kwinrc --group Plugins --key appnapEnabled true
if command -v qdbus >/dev/null 2>&1; then
  qdbus org.kde.KWin /KWin reconfigure || true
elif command -v qdbus6 >/dev/null 2>&1; then
  qdbus6 org.kde.KWin /KWin reconfigure || true
fi

printf 'app-nap installed\n'
