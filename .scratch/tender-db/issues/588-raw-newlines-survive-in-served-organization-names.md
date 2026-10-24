# 588 — raw newlines survive in served organization names

Status: needs-triage — filed 2026-10-24 from 576 U4 and 586 U3's read-backs (576-u4 "raw newlines survive in two served
names"; 586-u3 B28 "Tribunal administratif de Poitiers\nInstance chargée de l'introduction des recours", org 31975366).
Next: unit 1, count organizations / mention names with \n or \r (bounded windows) and decide where to collapse (mint-time
name normalisation for served names vs. head election; 524's R0w already collapses wraps for heads).
Kind: data quality / served text (N: the name is right, its formatting is not; a second org per wrapped spelling is W-ish)

## Verify

    curl -s https://tenders.zebreus.click/v1/organizations/31975366 | jq -r .name | grep -c $'\n'

- **open (2026-10-24)**: `1` (the served name holds a raw newline; the org is a second row beside the tribunal 2644).
- **done**: `0`, and the wrapped spelling binds the tribunal's row.
