#!/usr/bin/env bash
# app-nap-ls - list the apps the app-nap daemon is tracking.
set -euo pipefail

SERVICE="dev.appnap.AppNap"
OBJECT="/dev/appnap/AppNap"
INTERFACE="dev.appnap.AppNap1"

usage() {
  cat <<'EOF'
usage: app-nap-ls [-v|--verbose] [-j|--json]

Ask the running app-nap daemon which apps it tracks and print each app's
voted policy, CPU usage, throttle, and window count.

  -v, --verbose  one block per app
  -j, --json     the raw snapshot as JSON
  -h, --help     show this help

Usage and throttle are in core-equivalents (1.00 = one fully busy core) and
are not sampled under the performance policy.
EOF
}

format="table"
while [[ $# -gt 0 ]]; do
  case "$1" in
    -v | --verbose) format="verbose" ;;
    -j | --json) format="json" ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      printf 'app-nap-ls: unknown option: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

for cmd in busctl jq awk; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    printf 'app-nap-ls: missing required command: %s\n' "${cmd}" >&2
    exit 1
  fi
done

if ! reply="$(busctl --user --json=short call "${SERVICE}" "${OBJECT}" "${INTERFACE}" ListApps 2>&1)"; then
  printf 'app-nap-ls: cannot query the app-nap daemon: %s\n' "${reply}" >&2
  printf 'app-nap-ls: is it running? systemctl --user status app-nap.service\n' >&2
  exit 1
fi

# busctl reports D-Bus structs positionally; name the fields once, here.
# The `+ 0` on the doubles drops busctl's exponential literals (0E-21).
apps="$(jq -c '.data[0] | map({
  window_pid: .[0], comm: .[1], policy: .[2], usage: (.[3] + 0),
  throttle: (.[4] + 0), window_count: .[5]
})' <<<"${reply}")"

if [[ "${format}" == "json" ]]; then
  jq . <<<"${apps}"
  exit 0
fi

if [[ "$(jq 'length' <<<"${apps}")" -eq 0 ]]; then
  printf 'no apps tracked\n'
  exit 0
fi

# Usage and throttle are only tracked off the performance policy.
if [[ "${format}" == "verbose" ]]; then
  jq -r '.[] | [
    .window_pid, .comm, .policy, .usage, .throttle, .window_count
  ] | @tsv' <<<"${apps}" |
    awk -F'\t' '{
      printf "%s (pid %s)\n", ($2 == "" ? "?" : $2), $1
      printf "  policy     %s\n", $3
      if ($3 == "Performance") {
        printf "  usage      -\n"
        printf "  throttle   -\n"
      } else {
        printf "  usage      %.2f\n", $4
        printf "  throttle   %.2f\n", $5
      }
      printf "  windows    %s\n\n", $6
    }'
  exit 0
fi

table="$(jq -r '.[] | [
    .window_pid, .comm, .policy, .usage, .throttle, .window_count
  ] | @tsv' <<<"${apps}" |
  awk -F'\t' -v OFS='\t' '
    BEGIN { print "PID", "APP", "POLICY", "USAGE", "THROTTLE", "WINDOWS" }
    {
      usage = sprintf("%.2f", $4)
      throttle = sprintf("%.2f", $5)
      if ($3 == "Performance") {
        usage = "-"
        throttle = "-"
      }
      print $1, ($2 == "" ? "?" : $2), $3, usage, throttle, $6
    }')"

if command -v column >/dev/null 2>&1; then
  column -t -s $'\t' <<<"${table}"
else
  printf '%s\n' "${table}"
fi
