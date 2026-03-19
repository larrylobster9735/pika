#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/incus-dogfood-check.sh --api-base-url URL --nsec NSEC [options]

Options:
  --remote-host HOST     SSH host running Incus (default: pika-build)
  --project NAME         Incus project (default: pika-managed-agents)
  --storage-pool NAME    Incus storage pool (default: default)

This is a repeated dogfood helper for the internal Incus lane. It prints:
  - current managed-agent API state
  - VM ID
  - whether the Incus instance exists
  - whether the matching -state volume exists
  - the current guest ready marker, when available
EOF
}

api_base_url=""
nsec=""
remote_host="pika-build"
project="pika-managed-agents"
storage_pool="default"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --api-base-url)
      api_base_url="${2:-}"
      shift 2
      ;;
    --nsec)
      nsec="${2:-}"
      shift 2
      ;;
    --remote-host)
      remote_host="${2:-}"
      shift 2
      ;;
    --project)
      project="${2:-}"
      shift 2
      ;;
    --storage-pool)
      storage_pool="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "$api_base_url" || -z "$nsec" ]]; then
  usage >&2
  exit 1
fi

tmp_json="$(mktemp)"
trap 'rm -f "$tmp_json"' EXIT

cargo run -q -p pikachat -- agent me \
  --api-base-url "$api_base_url" \
  --nsec "$nsec" >"$tmp_json"

echo "== API state =="
cat "$tmp_json"
echo

vm_id="$(
  python3 - <<'PY' "$tmp_json"
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)
print((data.get("agent") or {}).get("vm_id") or "")
PY
)"

if [[ -z "$vm_id" ]]; then
  echo "No VM ID is currently assigned."
  exit 0
fi

echo "== Incus instance =="
ssh "$remote_host" \
  "incus list --project '$project' --format json | python3 - <<'PY' '$vm_id'
import json, sys
target = sys.argv[1]
rows = json.load(sys.stdin)
matches = [row for row in rows if row.get('name') == target]
if not matches:
    print('missing')
else:
    row = matches[0]
    print(json.dumps({'name': row.get('name'), 'status': row.get('status')}, indent=2))
PY"

echo
echo "== Incus state volume =="
ssh "$remote_host" \
  "incus storage volume list '$storage_pool' --project '$project' --format json | python3 - <<'PY' '${vm_id}-state'
import json, sys
target = sys.argv[1]
rows = json.load(sys.stdin)
matches = [row for row in rows if row.get('name') == target]
if not matches:
    print('missing')
else:
    row = matches[0]
    print(json.dumps({'name': row.get('name'), 'type': row.get('type')}, indent=2))
PY"

echo
echo "== Guest ready marker =="
ssh "$remote_host" \
  "incus file pull --project '$project' '$vm_id'/workspace/pika-agent/service-ready.json - 2>/dev/null || echo missing"
