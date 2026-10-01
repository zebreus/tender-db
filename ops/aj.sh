#!/usr/bin/env bash
# aj.sh — the one-line admin helper the handover and the issue files call as /root/aj.sh.
#
#   aj.sh <path>             GET  <path>                    (a query string travels with it)
#   aj.sh <path> '<json>'    POST <path>, that JSON as the body
#   aj.sh <path> @<file>     POST <path>, the file's bytes as the body
#
# There is NO method word: a body is what makes it a POST, and `aj.sh POST /x …` is
# refused (it was two failed attempts on 2026-10-01). The server's answer goes to stdout
# exactly as it came, so `… | python3 -c 'json.load(sys.stdin)…'` reads what the server
# said. Any status but 2xx is ALSO named on stderr (`aj.sh: GET <path> answered HTTP 403`)
# and exits 1: the admin API's 403 and 404 are JSON, and a JSON error is not an answer
# (issue 459).
#
# Auth is the x-admin-secret header (not `Authorization: Bearer`; crates/app/src/admin.rs
# SECRET_HEADER). /root/tender-admin-secret is the systemd EnvironmentFile the service
# reads, one `TENDER_ADMIN_SECRET=<hex>` line, so the secret is the value after the `=`:
# the same reader as ops/admin.sh and the watchdogs. Sending the whole line is a 403. The
# secret is never echoed. TENDER_ADMIN_URL and TENDER_ADMIN_SECRET_FILE override the
# defaults, as they do for ops/admin.sh (the offline tests point both at a fixture).
#
# Versioned since issue 464: it lived only on the box (written 2026-09-08) while 24 issue
# files, six open Verify lines and the handover called it — issue 224's shape, where
# box-only scripts vanished and nobody noticed for a week. ops/watchdogs/install.sh
# installs this copy as /root/aj.sh (mode 0700); ops/watchdogs/test-watchdogs.sh pins it.
# ops/admin.sh (installed as tender-admin) is the CLI with named commands.
set -euo pipefail

base=${TENDER_ADMIN_URL:-http://127.0.0.1:8080}
secret_file=${TENDER_ADMIN_SECRET_FILE:-/root/tender-admin-secret}
usage="usage: aj.sh <path> [<json> | @<file>]  (a body makes it a POST; there is no method word)"

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    echo "aj.sh: $usage" >&2
    exit 2
fi
path=$1
case "$path" in
    /*) ;;
    *)
        echo "aj.sh: '$path' is not a path — $usage" >&2
        exit 2
        ;;
esac
method=GET
if [ $# -eq 2 ]; then
    method=POST
    # curl turns an unreadable @file into an EMPTY POST with exit 0; refuse it instead.
    case "$2" in
        @*)
            if [ ! -r "${2#@}" ]; then
                echo "aj.sh: cannot read ${2#@} — nothing was sent" >&2
                exit 2
            fi
            ;;
    esac
fi

secret=$(sed -n 's/^TENDER_ADMIN_SECRET=//p' "$secret_file" 2>/dev/null || true)
if [ -z "$secret" ]; then
    echo "aj.sh: no operator secret in $secret_file" >&2
    exit 1
fi

body_file=$(mktemp)
trap 'rm -f "$body_file"' EXIT
args=(-s -m 300 -o "$body_file" -w '%{http_code}' -H "x-admin-secret: $secret")
if [ "$method" = POST ]; then
    args+=(-X POST -H "content-type: application/json" --data-binary "$2")
fi
curl_rc=0
code=$(curl "${args[@]}" "$base$path") || curl_rc=$?
if [ "$curl_rc" -ne 0 ]; then
    echo "aj.sh: $method $path: no answer from $base (curl exit $curl_rc)" >&2
    exit 1
fi
cat "$body_file"
case "$code" in
    2??) ;;
    *)
        echo "aj.sh: $method $path answered HTTP $code" >&2
        exit 1
        ;;
esac
