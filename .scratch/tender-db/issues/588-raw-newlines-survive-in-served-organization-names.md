# 588 — raw newlines survive in served organization names

Status: ready-for-agent — **TRIAGED 2026-10-24 ~19:00Z (owner, code read)**. Where it comes from: a single-mention organization serves the name it was minted with, raw (`org_name_norm`, canonical.rs:7504, collapses whitespace only for the KEY, so the wrapped spelling is its own key and its own row); head election never reaches it (2+ mention rows only: 2771 elected 2,095,436, 3,582,492 below), and for elected heads the v12 lines guard already withholds a line-broken head unless every break is a wrap (R0w; 2771: lines 21 held, R0w 0). No existing census can count it: `name-phrase-census` folds the newline away and a windowed /v1/sql sweep of `organizations.name` is a corpus scan (prod-box-reads: no on-box path). NEXT: unit 1 BUILD — a `line_breaks: true` mode on `name-phrase-census` (served names, and mention names with `mentions`), reporting hits by source/profile, wrap vs lines (`head_line_wrap`), and the top second-line texts; rides the first code deploy after deploy #4 (canonical.rs is mid-edit for 561 U4). Unit 2 then decides per shape: a wrap → collapse at mint (the R0w rule, so the key heals); a name + role/department line ("…\nInstance chargée de l'introduction des recours") → strip a known role-label line before the key (586 D2's lists) so it binds the body's row; anything else → serve as published (N). Was: needs-triage — filed 2026-10-24 from 576 U4 and 586 U3's read-backs (576-u4 "raw newlines survive in two served
names"; 586-u3 B28 "Tribunal administratif de Poitiers\nInstance chargée de l'introduction des recours", org 31975366).
Next: unit 1, count organizations / mention names with \n or \r (bounded windows) and decide where to collapse (mint-time
name normalisation for served names vs. head election; 524's R0w already collapses wraps for heads).
Kind: data quality / served text (N: the name is right, its formatting is not; a second org per wrapped spelling is W-ish)

## Verify

    curl -s https://tenders.zebreus.click/v1/organizations/31975366 | jq -r .name | grep -c $'\n'

- **open (2026-10-24)**: `1` (the served name holds a raw newline; the org is a second row beside the tribunal 2644).
- **done**: `0`, and the wrapped spelling binds the tribunal's row.
