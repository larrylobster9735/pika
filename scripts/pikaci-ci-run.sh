#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: pikaci-ci-run.sh <target>

Run a `pikaci` target through the packaged control-plane binaries instead of
Cargo-building `target/debug/pikaci` on the host runner.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -ne 1 ]]; then
  echo "error: expected exactly one target id" >&2
  usage >&2
  exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

source "$script_dir/lib/pikaci-tools.sh"

resolve_pikaci_tools "$repo_root"
log_pikaci_tool_resolution "ci-run"

cd "$repo_root"
target="$1"
run_output="$(mktemp)"
cleanup() {
  rm -f "$run_output"
}
trap cleanup EXIT

set +e
"$PIKACI_BIN" run "$target" --output json | tee "$run_output"
status=${PIPESTATUS[0]}
set -e

if [[ $status -ne 0 ]]; then
  mapfile -t run_ids < <(
    python3 - "$run_output" <<'PY'
import json
import pathlib
import sys

payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
run_id = payload.get("run_id")
if run_id:
    print(run_id)
PY
  )

  if [[ ${#run_ids[@]} -eq 0 ]]; then
    echo "error: failed to determine pikaci run id for target \`$target\`" >&2
  fi

  for run_id in "${run_ids[@]}"; do
    while IFS=$'\t' read -r job_id log_path; do
      echo
      echo "===== pikaci host log: run=$run_id job=$job_id =====" >&2
      cat "$log_path" >&2
    done < <(
      "$PIKACI_BIN" logs "$run_id" --metadata-json \
        | python3 -c '
import json
import sys

payload = json.load(sys.stdin)
for job in payload.get("jobs", []):
    if job.get("host_log_exists"):
        print("{}\t{}".format(job["id"], job["host_log_path"]))
'
    )
  done
fi

exit "$status"
