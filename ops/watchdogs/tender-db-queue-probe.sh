#!/usr/bin/env bash
# tender-db queue probe — ONE line naming what GET /admin/jobs says about the job
# queue, for the two callers that must not act on a busy box (issue 459):
#
#   deploy.sh               pipes THIS file over ssh (`bash -s`), so a deploy never
#                           depends on what happens to be installed on the box;
#   tender-db-snapshot.sh   runs the installed copy beside it in /usr/local/bin.
#
# The answers, exactly one per run:
#
#   idle                       HTTP 200 and the body has a `current` key whose value is null
#   busy <id> <kind> <params>  HTTP 200 and `.current` is an object
#   down                       curl exit 7 (refused) on a LOOPBACK url, AND systemd says
#                              tender-db.service is loaded with no main process: no app
#                              is running, so nothing writes (issue 420's reasoning, as a
#                              NAMED arm that needs two independent readings, not a default)
#   error <what>               everything else, named: no secret, curl exit N, HTTP 403 bad or
#                              missing operator secret, HTTP 200 without a `current` key, …
#
# Exit 0 with idle/busy/down, exit 1 with error. Callers proceed ONLY on exit 0 with
# the exact line `idle` or `down`; anything else — an `error`, an empty answer, an ssh
# failure, two lines, output nobody foresaw — is a refusal or a wait.
#
# Why named answers. The two probes this replaces printed an empty string for "idle"
# AND for every failure to measure: ssh down, secret missing (`exit 0`!), curl timing
# out, a 403 or 404 JSON body (no `.current` → ""). On any of those a deploy would
# restart a running job from the top (issue 245) and the snapshot would reflink a fold
# mid-write (issue 420).
# That is instrument-discipline.md ledger #6 exactly — "0 means idle means go" — and the
# fix is its #9: proceeding must require a positive answer, so a failure cannot share
# its representation with the healthy one.
#
# The 403 and 404 are real answers of this server (admin.rs deny()): 404 `not found`
# when TENDER_ADMIN_SECRET is unset in the service, 403 `bad or missing operator secret`
# on a mismatch. The service reads /root/tender-admin-secret ONCE at start (the admin.conf
# drop-in's EnvironmentFile) and this reads it every run, so a secret rotated in the file
# without a restart is a 403 here — an error, never idle.
#
# `down` needs two readings, because a refusal alone is a correlate of "no app", not the
# artifact (review of 459). curl exits 7 just the same when the app is UP and the probe
# asked the wrong place: an http_proxy in the environment that refuses (so curl gets
# --noproxy for the loopback names), a PORT changed in the unit, or a non-loopback
# address that is unreachable from here — and on curl 8.5 the message is identical. So
# exit 7 counts only on 127.0.0.1 / localhost / [::1], and only when
# `systemctl show tender-db.service` says LoadState=loaded and MainPID=0: the service
# has no process at all. A running process, a unit systemd does not know, or a
# systemctl that cannot answer turns the refusal into an `error`. Start-up is not a
# `down` either way: dioxus binds the listener before it serves, so a starting app
# accepts the connection and the probe times out (curl exit 28, an `error`).
#
# Nothing here may read stdin: under `ssh … bash -s` stdin IS this script, and a command
# that read it would swallow the rest of the probe. Every command below has a file
# argument or </dev/null.
#
# Versioned in ops/watchdogs/ and installed to /usr/local/bin by install.sh (issue 224).
set -uo pipefail

secret_file=${TENDER_ADMIN_SECRET_FILE:-/root/tender-admin-secret}
url=${TENDER_ADMIN_URL:-http://127.0.0.1:8080}
# The unit `down` is checked against. Named, not matched (instrument-discipline.md,
# "Select by naming"), and not configurable: a probe told to look at another unit would
# find that one stopped.
unit=tender-db.service

answer() { printf '%s\n' "$*"; }
# Flatten to one line: an answer is a single line by contract, whatever the server sent.
oneline() { tr '\r\n\t' '   ' | cut -c1-200 | sed 's/ *$//'; }
fail() { answer "error $*"; exit 1; }

if [ ! -r "$secret_file" ]; then
    fail "no readable secret file $secret_file"
fi
# /root/tender-admin-secret is a systemd EnvironmentFile (`TENDER_ADMIN_SECRET=…`).
secret=$(sed -n 's/^TENDER_ADMIN_SECRET=//p' "$secret_file" </dev/null 2>/dev/null | head -n 1)
if [ -z "$secret" ]; then
    fail "no TENDER_ADMIN_SECRET in $secret_file"
fi

tmp=$(mktemp) || fail "mktemp failed"
errf=$(mktemp) || { rm -f "$tmp"; fail "mktemp failed"; }
trap 'rm -f "$tmp" "$errf"' EXIT

curl_rc=0
code=$(curl -sS --max-time 10 --noproxy '127.0.0.1,localhost,::1' -o "$tmp" -w '%{http_code}' \
    -H "x-admin-secret: $secret" "$url/admin/jobs" </dev/null 2>"$errf") || curl_rc=$?

if [ "$curl_rc" -eq 7 ]; then
    # The host part of the url: scheme, userinfo, path and port stripped; [v6] unbracketed.
    hostport=${url#*://}; hostport=${hostport%%/*}; hostport=${hostport##*@}
    case "$hostport" in
        \[*\]*) host=${hostport#\[}; host=${host%%\]*} ;;
        *)      host=${hostport%%:*} ;;
    esac
    case "$host" in
        127.0.0.1|localhost|::1) ;;
        *) fail "curl exit 7 ($(oneline <"$errf")) at $url — $host is not a loopback address, so a refusal there says nothing about whether this box's app is running" ;;
    esac
    svc=$(systemctl show --property=LoadState --property=MainPID "$unit" </dev/null 2>/dev/null) \
        || fail "curl exit 7 at $url, and systemctl could not say whether $unit has a process"
    load=$(sed -n 's/^LoadState=//p' <<<"$svc" | head -n 1)
    pid=$(sed -n 's/^MainPID=//p' <<<"$svc" | head -n 1)
    if [ "$load" = loaded ] && [ "$pid" = 0 ]; then
        answer down
        exit 0
    fi
    fail "curl exit 7 at $url, but $unit is not stopped (LoadState=${load:-?} MainPID=${pid:-?}) — a wrong port, a proxy or a changed bind address, not a down app"
fi
if [ "$curl_rc" -ne 0 ]; then
    fail "curl exit $curl_rc ($(oneline <"$errf")) at $url/admin/jobs"
fi

if [ "$code" != "200" ]; then
    # deny() and error() answer {"error":{"status":N,"message":"…"}}; anything else is
    # quoted raw (an nginx 502 page, an empty body).
    msg=$(jq -r '.error.message // empty' "$tmp" </dev/null 2>/dev/null | oneline)
    [ -n "$msg" ] || msg=$(oneline <"$tmp")
    fail "HTTP $code ${msg:-(empty body)}"
fi

# HTTP 200. The body must be an object with a `current` key: null is idle, an object is
# busy, and anything else — a missing key, another type, not JSON — is not a reading.
# shellcheck disable=SC2016
state=$(jq -r '
    if type != "object" then "error HTTP 200 but the body is a JSON \(type), not the jobs object"
    elif has("current") | not then "error HTTP 200 without a current key"
    elif .current == null then "idle"
    elif (.current | type) == "object" then
        "busy \(.current.id // "?") \(.current.kind // "?") \(.current.params // "")"
    else "error HTTP 200 with a current of type \(.current | type)"
    end' "$tmp" </dev/null 2>/dev/null) || state="error HTTP 200 but the body is not JSON"
state=$(printf '%s' "$state" | oneline)

case "$state" in
    idle|"busy "*) answer "$state"; exit 0 ;;
    "error "*)     answer "$state"; exit 1 ;;
    *)             fail "unreadable probe state: ${state:-empty}" ;;
esac
