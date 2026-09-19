# 293 — text-era BODY extraction (the header-only profile's other half)

Status: BACKLOG (filed 2026-08-26 on Lennart's request — capture every deferral)
Kind: capability (TED text era 1993-2010)
Relates to: 244 (the campaign that extracted awards/money from the same era), 232, 291.

The text-era profile is deliberately header-only (spec.md non-goal: "heuristic
text-era body extraction"; CONTEXT.md "text era header-only … for now"). The OT
body prose — descriptions, conditions, the majority of each notice's content — is
parsed as one untagged blob at best. Related research debt: A18 (field-code
semantics) was never done; the profile shipped on inferred meanings
(docs/research/research-gaps-2026-08.md).

When picked up: start with A18 (verify the field-code semantics against TED's own
documentation/archive), then extract the highest-value body fields the 244
campaign's machinery already proved reachable (sectioned-form slicing). Scope it
per-field like 244's slices, not as one big-bang parser.

## Verify

    curl -s --max-time 20 https://tenders.zebreus.click/v1/notices/17424/content | python3 -c 'import sys,json; d=json.load(sys.stdin); print(len({v.get("field_id") for s in d["sections"] for v in s["values"] if str(v.get("field_id")).startswith("TXT-TX")}))'

- **done**: more than one `TXT-TX…` field id — the OT body is sliced into per-field facts (the 244 machinery), not one blob
- **open**: `1` (read 2026-09-19) — the body is one untagged `TXT-TX` blob beside the header fields
