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
# reshuffle cannot silently watch the wrong project). Network failure is a
# WARN, not silence: an upstream watch that fails quietly is the issue-224
# lesson all over again — but it exits 0 on network trouble, because a flaky
# mirror must not sit in `systemctl --failed` masking a REAL drift alarm.
#
# Versioned in the repo (ops/watchdogs/), installed to /usr/local/bin by
# ops/watchdogs/install.sh — same durability rule as its siblings (issue 224).
set -euo pipefail

# The vendored line: releases whose tag starts with this are known and fine.
known="${TENDER_DRIFT_KNOWN_PREFIX:-1.14}"
api="${TENDER_DRIFT_API:-https://gitlab.opencode.de/api/v4/projects/OC000008125155%2FSDK-eforms-de/releases?per_page=10}"

if ! body=$(curl -sS --max-time 30 "$api"); then
    echo "WARN drift: SDK-eforms-de release probe failed (network) — upstream is UNWATCHED this run"
    exit 0
fi

newest=$(printf '%s' "$body" | python3 -c "
import json, sys
try:
    releases = json.load(sys.stdin)
    print(releases[0]['tag_name'] if isinstance(releases, list) and releases else '')
except Exception:
    print('')
")
if [ -z "$newest" ]; then
    echo "WARN drift: SDK-eforms-de release probe answered unparseably — upstream is UNWATCHED this run"
    exit 0
fi

case "$newest" in
    "$known"*)
        echo "ok drift: newest SDK-eforms-de release is $newest (vendored line ${known}.x)"
        ;;
    *)
        echo "WARN drift: SDK-eforms-de released $newest — BEYOND the vendored ${known}.x line."
        echo "The eForms-DE successor has shipped (issue 165): vendor its fields.json, add the"
        echo "resolve() arm, and update TENDER_DRIFT_KNOWN_PREFIX in the driftwatch unit."
        exit 1
        ;;
esac
