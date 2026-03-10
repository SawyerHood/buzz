#!/usr/bin/env bash

set -euo pipefail

ROOT_PID="${1:-}"
INTERVAL_SECONDS="${2:-5}"
WEBKIT_CACHE_ROOT="${HOME}/Library/Caches/tauri-app/WebKit"

if [[ -z "${ROOT_PID}" ]]; then
  ROOT_PID="$(pgrep -nf 'target/.*/debug/tauri-app' || true)"
fi

if [[ -z "${ROOT_PID}" ]]; then
  echo "Unable to find a running dev tauri-app process. Pass the root PID explicitly." >&2
  exit 1
fi

collect_descendants() {
  local parent_pid="$1"
  echo "${parent_pid}"

  while IFS= read -r child_pid; do
    [[ -z "${child_pid}" ]] && continue
    collect_descendants "${child_pid}"
  done < <(pgrep -P "${parent_pid}" || true)
}

collect_app_webkit_helpers() {
  for process_id in $(pgrep -f 'com.apple.WebKit.(WebContent|Networking|GPU)' || true); do
    if lsof -nP -p "${process_id}" 2>/dev/null | grep -q "${WEBKIT_CACHE_ROOT}"; then
      echo "${process_id}"
    fi
  done
}

while true; do
  if ! kill -0 "${ROOT_PID}" 2>/dev/null; then
    echo "Root PID ${ROOT_PID} exited."
    exit 0
  fi

  process_ids=()
  while IFS= read -r process_id; do
    [[ -z "${process_id}" ]] && continue
    process_ids+=("${process_id}")
  done < <(
    {
      collect_descendants "${ROOT_PID}"
      collect_app_webkit_helpers
    } | awk '!seen[$0]++'
  )
  process_list="$(IFS=,; echo "${process_ids[*]}")"
  timestamp="$(date '+%Y-%m-%dT%H:%M:%S%z')"
  ps_output="$(ps -o pid=,ppid=,rss=,comm= -p "${process_list}" | sort -k3 -nr)"
  total_kb="$(awk '{sum += $3} END {print sum + 0}' <<<"${ps_output}")"
  total_mib="$(awk -v kb="${total_kb}" 'BEGIN {printf "%.1f", kb / 1024}')"

  echo "[${timestamp}] root_pid=${ROOT_PID} total_rss_mib=${total_mib}"
  awk '{printf "  pid=%s ppid=%s rss_mib=%.1f cmd=%s\n", $1, $2, $3 / 1024, $4}' <<<"${ps_output}"
  echo

  sleep "${INTERVAL_SECONDS}"
done
