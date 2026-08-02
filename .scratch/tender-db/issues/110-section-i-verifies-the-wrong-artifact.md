# 110 — section I verifies the wrong artifact: it greps SSR HTML for a client-hydrated ledger

Status: open — POST-LANDING follow-up. Found during the 98+99 ship, 2026-08-02.
Kind: verification tooling / false-negative gate
Blocked by: — (waits on `issue98-de1x-org-refs` being unfrozen)
Relates to: 107 (snapshot-freshness witness), 102 (baseline naming the wrong source), 98, 100

## Defect

Section I of `de1x_verify.sh` asserts the honesty requirement — that the deployed ledger discloses the
DE-1.x award-winner gap. It implements that by fetching `GET /` and grepping the returned HTML for
`AWARD WINNERS ARE NOT RESOLVED`, `issue 100`, and the 241 sentence.

**The dashboard renders the Resolved-categories table client-side.** No ledger entry appears in the
server-rendered HTML — not this one, not the r208 entry, not the "Resolved categories" heading itself.
The initial hydration payload (`dioxus_hydration_data`, ~6.5 KB decoded) does not carry it either; the
client fetches it afterwards from `/api/dashboard`.

So the gate returns MISSING for a correct deployment. It hard-fails, and its failure is
indistinguishable from the ledger genuinely being absent.

## Why it survived review

It was tested against a stub HTTP server written for the test — which returned the string, because the
same author wrote both sides. The test proved the grep worked, never that the real application serves
that string in that place. **A check validated against a mock of its own assumptions is a mirror, not a
test.**

## Fix

Verify the artifact that carries the claim to the user, not a proxy:

1. **`/api/dashboard`** (the client-fetched payload) — assert the disclosure is in the served JSON.
   This is the real answer: it is what the browser renders from.
2. Optionally also assert it in the **deployed binary** (`grep` the nix-store `server` binary, since the
   ledger is `include_str!`-compiled) as a cheap corroborating check — necessary but not sufficient,
   since a string present in the binary is not proof it reaches a user.
3. **Handle the coverage-disabled case explicitly.** `/api/dashboard` returns all-null fields when the
   coverage refresher is disabled (`TENDER_DISABLE_COVERAGE`) or has not completed a cycle. That is
   *neither* pass nor fail for the disclosure — it means the dashboard has no data at all, which is its
   own alarm. The gate must distinguish three states: disclosure served ✓ / disclosure absent from a
   populated payload ✗ / payload empty → **not a verdict, a different problem**.

## Note

Third instance in this batch of an artifact-vs-proxy confusion, and the first in a gate I wrote myself:
102 (a baseline file naming the wrong source), 107 (a verify blind to which snapshot it read), and now a
gate measuring server-rendered HTML for data that is never server-rendered.

The reasoning was right — *"the ledger is `include_str!`-compiled, so only the deployed binary counts"* —
and the implementation then checked something else entirely. Being right about what matters does not
guarantee measuring it.
