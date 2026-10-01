#!/usr/bin/env bash
# tender-db upstream-drift watch — warn (to the journal) when the eForms-DE
# SDK publishes a release beyond the line this deployment vendors (issue 165).
# Detection only; runs daily via tender-db-driftwatch.timer.
#
# Why this exists: DÖE's acceptance window for eForms-DE 2.1 / SDK 1.14 ends
# 2026-12-02. Its successor (branches say both "2.2" and "3.0", built on EU SDK
# 1.15: BT-22 split, BR-DE-37/38, BT-DEX-05) is in active development and
# UNRELEASED as of 2026-08-21. The day it ships, notices will start declaring a
# CustomizationID our `resolve()` does not know and will quarantine — the honest
# failure, but one that costs a reprocess for every day nobody notices. This
# watch turns "someone remembers to check the repo" into a journal line and a
# `systemctl --failed` entry on the day it happens.
#
# The probe is ONE anonymous GET against the public gitlab.opencode.de API for
# OC000008125155/SDK-eforms-de (project id 418 — resolved by path here so an id
# reshuffle cannot silently watch the wrong project).
#
# A probe that could not look is an ERROR and exits 1 (issue 459): a network
# failure, an answer other than HTTP 200, or a 200 that is not a non-empty release
# list whose every tag parses as <major>.<minor>… (a GitLab 404, an API path change). This used to be a WARN on stdout with
# exit 0, "because a flaky mirror must not sit in `systemctl --failed` masking a
# REAL drift alarm". The record never supported that: 42 runs from 2026-08-21 to
# 2026-10-01, all `ok drift`, not one WARN — while the exit 0 meant a dead probe
# looked exactly like "no release yet" until 165's 2026-12-02 deadline. If the
# mirror ever does prove flaky, alarm on the AGE of the last good probe (a stamp
# file), not on exit 0. A drift alarm and an ERROR both exit 1; the line says which.
#
# Versioned in the repo (ops/watchdogs/), installed to /usr/local/bin by
# ops/watchdogs/install.sh — same durability rule as its siblings (issue 224).
set -euo pipefail

# The vendored line, as <major>.<minor>: a release on it or on an older line is known
# and fine; one on a newer line is the drift this watch exists for.
known="${TENDER_DRIFT_KNOWN_PREFIX:-1.14}"
api="${TENDER_DRIFT_API:-https://gitlab.opencode.de/api/v4/projects/OC000008125155%2FSDK-eforms-de/releases?per_page=10}"

body_file=$(mktemp)
trap 'rm -f "$body_file"' EXIT
curl_rc=0
code=$(curl -sS --max-time 30 -o "$body_file" -w '%{http_code}' "$api" 2>/dev/null) || curl_rc=$?
if [ "$curl_rc" -ne 0 ]; then
    echo "ERROR drift: SDK-eforms-de release probe failed (network: curl exit $curl_rc) — upstream is UNWATCHED this run"
    exit 1
fi
if [ "$code" != "200" ]; then
    echo "ERROR drift: SDK-eforms-de release probe answered HTTP $code, not a release list — upstream is UNWATCHED this run"
    exit 1
fi

# EVERY tag on the page is read, not the first (review of 459). The feed is ordered by
# released_at, and upstream publishes fixes to older lines after newer ones: on
# 2026-10-01 it read 1.14.4, 1.14.3, 1.13.3 (2026-05-04), 1.14.2, … — so once 1.15.0
# ships, the next 1.14.x patch sits at index 0, and a watch of releases[0] would say
# `ok` with the successor one entry down. Each tag is parsed as <major>.<minor>[…] and
# compared as numbers (a prefix match would also have accepted 1.140).
#
#   ok <count> <highest>       a non-empty release list, every tag parsed, none beyond
#   beyond <tag>[, <tag>…]     the tags on a line newer than $known
#   bad <why>                  anything else: not JSON, not a list, empty, a release
#                              without a tag, a tag that is not <major>.<minor>…, or a
#                              $known that is not <major>.<minor> — the probe is not
#                              reading what it thinks, so it cannot say "no drift"
# An empty list is `bad` too: the project has published 1.14.x releases, so [] means
# the probe is reading something else, not that upstream is quiet.
parsed=$(python3 - "$body_file" "$known" <<'PY'
import json, re, sys
known_raw = sys.argv[2]
k = re.fullmatch(r"(\d+)\.(\d+)", known_raw.strip())
if not k:
    print("bad TENDER_DRIFT_KNOWN_PREFIX=%r is not <major>.<minor>" % known_raw); sys.exit()
known = (int(k[1]), int(k[2]))
try:
    releases = json.load(open(sys.argv[1]))
except Exception:
    print("bad the body is not JSON"); sys.exit()
if not isinstance(releases, list):
    print("bad the body is a JSON %s, not a list" % type(releases).__name__); sys.exit()
if not releases:
    print("bad the release list is empty"); sys.exit()
tags = []
for i, r in enumerate(releases):
    tag = r.get("tag_name") if isinstance(r, dict) else None
    if not isinstance(tag, str) or not tag.strip():
        print("bad release #%d on the page has no tag_name" % i); sys.exit()
    tag = tag.strip()
    m = re.fullmatch(r"v?(\d+)\.(\d+)(?:\.(\d+))?(?:[-+][0-9A-Za-z.+-]*)?", tag)
    if not m:
        print("bad release tag %r is not <major>.<minor>[.<patch>][-…]" % tag[:60]); sys.exit()
    tags.append(((int(m[1]), int(m[2]), int(m[3] or 0)), tag))
beyond = [t for v, t in tags if v[:2] > known]
if beyond:
    print("beyond " + ", ".join(beyond))
else:
    print("ok %d %s" % (len(tags), max(tags)[1]))
PY
) || parsed="bad the parser failed"
case "$parsed" in
    "ok "?*)
        read -r _ count highest <<<"$parsed"
        echo "ok drift: no SDK-eforms-de release beyond the vendored ${known}.x line ($count releases read, highest $highest)"
        ;;
    "beyond "?*)
        echo "WARN drift: SDK-eforms-de released ${parsed#beyond } — BEYOND the vendored ${known}.x line."
        echo "The eForms-DE successor has shipped (issue 165): vendor its fields.json, add the"
        echo "resolve() arm, and update TENDER_DRIFT_KNOWN_PREFIX in the driftwatch unit."
        exit 1
        ;;
    *)
        echo "ERROR drift: SDK-eforms-de release probe answered unparseably (${parsed#bad }) — upstream is UNWATCHED this run"
        exit 1
        ;;
esac
