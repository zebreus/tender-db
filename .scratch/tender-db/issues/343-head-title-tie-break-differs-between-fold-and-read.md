# 343 — a version with several tender-level titles: the fold's `current_title` and the read-time pick break the tie differently

Status: needs-triage (filed 2026-09-02 from a live observation during the 304
probe; cosmetic, bounded)
Kind: consistency (read layer vs fold-time head column)
Relates to: 115 (the set-based summary picks and their SQL oracle), 216 (the
materialised head columns), ADR-0013 D3 (`title_rank`)

## Observed

Tender 6287622, head version `seq 2` (a 2019 framework award, r209), carries
THREE tender-level titles in one language:

```
lot_id NULL  ENG  Domestic Courier Services on the Territory of Poland
lot_id NULL  ENG  Framework Contract for Courier Services for Frontex
lot_id NULL  ENG  International Export/Import Courier Services
```

(plus the same two lot titles under their lots). Two surfaces answer "the title"
differently:

| surface | title served |
| --- | --- |
| `tenders.current_title` (fold-time `head_title`, what `v_tenders` and the head sort see) | International Export/Import Courier Services |
| `/v1/tenders/6287622` and the list (read-time `title_rank` pick, `LIMIT 1`) | Domestic Courier Services on the Territory of Poland |

Both rows tie on every rank term (tender-level, ENG); each side breaks the tie by
its own scan order. The read layer's documented tie rule is "first in scan order"
(`summarise`'s doc, pinned by the SQL oracle); `head_title`'s is whatever its
precedence code does. So the same tender shows one title on `/v1/sql`'s
`v_tenders` and another on the REST detail.

## Why it is bounded

It needs a notice that publishes more than one tender-scoped title — r208/r209
framework notices that repeat `TI_TEXT` per lot group at tender scope do; eForms
publishes one BT-21 per procedure. Neither answer is wrong; they are different
members of the same tie. It does not affect matching, filters or the `?lang=`
legs — only which of two equally-ranked strings is shown.

## Fix shape

Make both sides break the tie the same way — the cheapest is to give the fold's
`head_title` the read layer's rule (first by `(tender_id, seq, rowid)` among the
top-ranked rows) or to make the read pick `ORDER BY … , rowid` explicit so scan
order is a stated rule rather than an accident of the plan. Pin with a fixture
carrying two tender-level titles and assert `current_title == list title ==
detail title`. Worth doing beside any other touch of `head_title`; not worth a
deploy of its own.
