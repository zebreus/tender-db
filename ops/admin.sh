#!/usr/bin/env bash
# tender-db admin CLI — call the local admin API with the operator secret.
#
#   admin.sh jobs                     GET /admin/jobs (pretty-printed)
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
