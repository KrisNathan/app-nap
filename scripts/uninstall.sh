#!/usr/bin/env bash
set -euo pipefail

BIN_PATH="${HOME}/.local/bin/app-nap"
APP_NAP_LS_PATH="${HOME}/.local/bin/app-nap-ls"
CONFIG_DIR="${HOME}/.config/app-nap"
CONFIG_PATH="${CONFIG_DIR}/app-nap.toml"
SYSTEMD_USER_DIR="${HOME}/.config/systemd/user"
SERVICE_PATH="${SYSTEMD_USER_DIR}/app-nap.service"
DEFAULT_CONFIG="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/example/app-nap.toml"

if command -v systemctl >/dev/null 2>&1; then
  systemctl --user disable --now app-nap.service 2>/dev/null || true
fi

rm -f "${SERVICE_PATH}"

if command -v systemctl >/dev/null 2>&1; then
  systemctl --user daemon-reload
fi

rm -f "${BIN_PATH}" "${APP_NAP_LS_PATH}"

if [[ -f "${CONFIG_PATH}" ]] && cmp -s "${CONFIG_PATH}" "${DEFAULT_CONFIG}"; then
  rm -f "${CONFIG_PATH}"
  rmdir "${CONFIG_DIR}" 2>/dev/null || true
fi

if command -v kpackagetool6 >/dev/null 2>&1; then
  kpackagetool6 --type=KWin/Script -r appnap 2>/dev/null || true
fi

if command -v kwriteconfig6 >/dev/null 2>&1; then
  kwriteconfig6 --file kwinrc --group Plugins --key appnapEnabled false
fi

if command -v qdbus >/dev/null 2>&1; then
  qdbus org.kde.KWin /KWin reconfigure || true
elif command -v qdbus6 >/dev/null 2>&1; then
  qdbus6 org.kde.KWin /KWin reconfigure || true
fi

printf 'app-nap uninstalled\n'
