#!/usr/bin/env bash
# tender-db admin CLI — call the local admin API with the operator secret.
#
#   admin.sh jobs                     GET /admin/jobs (pretty-printed)
#   admin.sh queue                    the same, as three lines a person can read
#   admin.sh enqueue <kind> [json]    POST /admin/jobs {"kind":<kind>, ...json}
#   admin.sh raw <METHOD> <path>      arbitrary admin call, body on stdin
#
# The secret is read from the same systemd EnvironmentFile the server uses
# (see jobwatch header) and never echoed. Versioned in ops/ and installed to
# /usr/local/bin by ops/watchdogs/install.sh conventions (issue 224).
set -euo pipefail

base=${TENDER_ADMIN_URL:-http://localhost:8080}
secret_file=${TENDER_ADMIN_SECRET_FILE:-/root/tender-admin-secret}

secret=$(sed -n 's/^TENDER_ADMIN_SECRET=//p' "$secret_file" 2>/dev/null || true)
if [ -z "$secret" ]; then
    echo "admin.sh: no operator secret in $secret_file" >&2
    exit 1
fi

cmd=${1:?usage: admin.sh jobs | enqueue <kind> [json] | raw <METHOD> <path>}
case "$cmd" in
jobs)
    curl -sS --max-time 15 "$base/admin/jobs" -H "x-admin-secret: $secret" | jq .
    ;;
enqueue)
    kind=${2:?usage: admin.sh enqueue <kind> [json-params]}
    extra=${3:-'{}'}
    body=$(jq -cn --arg kind "$kind" --argjson extra "$extra" '{kind: $kind} + $extra')
    curl -sS --max-time 15 -X POST "$base/admin/jobs" \
        -H "x-admin-secret: $secret" -H "content-type: application/json" \
        -d "$body" | jq .
    ;;
queue)
    # The compact read: what is running, what is waiting, what just finished.
    #
    # It prints the phase AND the progress record's package/member counters, because
    # the phase alone is not enough to tell work from a stall — a `reparse` phase reads
    # "0 / 1" for the whole of a five-minute package while `members_done` climbs past
    # 20,000. Both are already in /admin/jobs; only the readers were hiding one.
    curl -sS --max-time 15 "$base/admin/jobs" -H "x-admin-secret: $secret" | jq -r '
      (if .current == null then "CURRENT none" else (.current
        | "CURRENT \(.id) \(.kind) \(.params // "")"
          + " | \(.phase.name // "-") \(.phase.done // "-")/\(.phase.total // "-") \(.phase.detail // "")"
          + (if (.packages_total // 0) > 0 then " | pkg \(.packages_done)/\(.packages_total) \(.package // "")" else "" end)
          + (if (.members_total // 0) > 0 then " | members \(.members_done)/\(.members_total)" else "" end)
      ) end),
      ("QUEUED " + ((.queued // []) | map("\(.id):\(.kind)") | join(", ") | if . == "" then "none" else . end)),
      ((.recent // [])[:3][] | "  \(.id) \(.kind) \(.outcome) | \(.counts // "")")
    '
    ;;
raw)
    method=${2:?usage: admin.sh raw <METHOD> <path>}
    path=${3:?usage: admin.sh raw <METHOD> <path>}
    curl -sS --max-time 60 -X "$method" "$base$path" \
        -H "x-admin-secret: $secret" -H "content-type: application/json" \
        --data-binary @- | jq .
    ;;
*)
    echo "admin.sh: unknown command '$cmd'" >&2
    exit 1
    ;;
esac
