# 294 — sdk-1.0 award-chain check (the sibling 188 left for its own pass)

Status: BACKLOG (filed 2026-08-26; the deferral lives in 188's status line)
Kind: diagnosis (eForms sdk-1.0 era)
Relates to: 188 (sdk-0.1 verdicted publication reality), 264 (age-confound method).

Issue 188 verdicted sdk-0.1's 98% unchained awards as publication reality (the
source publishes no folder key) and explicitly "left sdk-1.0 for its own check" —
sdk-1.0 awards measure ~99.9% unchained and nobody has yet determined whether
that is the same publication reality or a mapping gap on our side.

Method exists: 264's within-era sampling (compare linkage against what the raw
XML actually publishes on a bounded sample). Outcome: either a panel explanation
row (like 188's) or a mapping fix + refold.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "/root/aj.sh /admin/reports/data-quality" | python3 -c 'import sys,json; b=json.load(sys.stdin)["body"]; s=b[b.find("== 2."):b.find("== 3.")]; print([l.strip() for l in s.split("\n") if "sdk-1.0" in l])'

- **done**: the row is EXPLAINED — either `linked` far above 0.3 % after a mapping fix and refold, or a verdict on this record (188's shape: publication reality, with the bounded-sample method of 264)
- **open**: `['eforms-sdk-1.0  713  711  0.3%']` unexplained (read 2026-09-19: 713 awards, 711 unchained)
