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
tier, CPU load state, and currently applied policy.

  -v, --verbose  one block per app, including cgroups and windows
  -j, --json     the raw snapshot as JSON
  -h, --help     show this help

Usage and throttle are in core-equivalents (1.00 = one fully busy core) and
are only sampled on the background and nap tiers.
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
  pid: .[0], name: .[1], tier: .[2], load: .[3], policy: .[4],
  usage: (.[5] + 0), throttle: (.[6] + 0), cgroups: .[7],
  windows: (.[8] | map({window_id: .[0], minimized: .[1], active: .[2]}))
})' <<<"${reply}")"

if [[ "${format}" == "json" ]]; then
  jq . <<<"${apps}"
  exit 0
fi

if [[ "$(jq 'length' <<<"${apps}")" -eq 0 ]]; then
  printf 'no apps tracked\n'
  exit 0
fi

# Load, usage and throttle are only tracked off the performance tier.
if [[ "${format}" == "verbose" ]]; then
  jq -r '.[] | [
    .pid, .name, .tier, .load, .policy, .usage, .throttle,
    (.cgroups | join(",")),
    (.windows | map("\(.window_id) \(if .active then "active" else "inactive" end)\(if .minimized then " minimized" else "" end)") | join(","))
  ] | @tsv' <<<"${apps}" |
    awk -F'\t' '{
      printf "%s (pid %s)\n", ($2 == "" ? "?" : $2), $1
      printf "  tier       %s\n", $3
      if ($3 == "performance") {
        printf "  load       -\n"
      } else {
        printf "  load       %s (usage %.2f, throttle %.2f)\n", $4, $6, $7
      }
      printf "  policy     %s\n", ($5 == "" ? "-" : $5)
      count = split($8, cgroups, ",")
      for (i = 1; i <= count; i++) printf "  %-10s %s\n", (i == 1 ? "cgroups" : ""), cgroups[i]
      count = split($9, windows, ",")
      for (i = 1; i <= count; i++) printf "  %-10s %s\n", (i == 1 ? "windows" : ""), windows[i]
      print ""
    }'
  exit 0
fi

table="$(jq -r '.[] | [
    .pid, .name, .tier, .load, .policy, .usage, .throttle, (.windows | length)
  ] | @tsv' <<<"${apps}" |
  awk -F'\t' -v OFS='\t' '
    BEGIN { print "PID", "APP", "TIER", "LOAD", "POLICY", "USAGE", "THROTTLE", "WINDOWS" }
    {
      load = $4
      usage = sprintf("%.2f", $6)
      throttle = sprintf("%.2f", $7)
      if ($3 == "performance") {
        load = "-"
        usage = "-"
        throttle = "-"
      }
      print $1, ($2 == "" ? "?" : $2), $3, load, ($5 == "" ? "-" : $5), usage, throttle, $8
    }')"

if command -v column >/dev/null 2>&1; then
  column -t -s $'\t' <<<"${table}"
else
  printf '%s\n' "${table}"
fi
