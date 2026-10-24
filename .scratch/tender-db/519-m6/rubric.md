Each case is one organization whose identity is a GB-NHS value that is not an NHS ODS code. You see its id,
literal, head name, every mention name with counts and raw identifiers, the shape of the value, and candidate
right identifiers with evidence: a same-name PPON or company-number row (id, literal, name), the 513/519
survivor of the code the value seems to spell (`code_survivors`), and register answers for company numbers.
Give ONE row: `verdict` (`wrong` | `right` | `skip`), `correct_identifier` (or null), `expected`
(`{shape: merge|move|unkeyed|none|hold, target: <org id or null>}`), `confidence` (high|medium|low) and a
two-sentence `rationale` naming the evidence.
- A **PPON value**: when a standing org carries `GBPPON<value>` and IS this organization (by its names):
  `wrong`, `GBPPON<value>`, `merge` into it. When it is a different entity (a host trust): `wrong`, no
  correction. With no standing owner: `wrong`, `GBPPON<value>`, `hold`.
- An **NHS body's own row** (a trust, ICB, health board, ambulance service, NI HSC body) under any other
  value: when a standing PPON org is that body (the same-name PPON row, or the survivor of the code the value
  spells): `wrong`, its `GBPPON…` literal, `merge` into it. Only when EVERY mention name is that body's.
  Never for NHS England, a site, place, hosted unit, network or collaborative.
- A **company number**: when the register's current or a previous name is this organization's: `wrong`, the
  8-character number, `merge` (a standing org carries it) or `move` (none does). Otherwise `wrong` with no
  correction, unless you find its own number or PPON on a standing same-name row.
- A **mistyped or glued NHS code** whose body has no standing PPON org: `wrong`, the right ODS code,
  `unkeyed`.
- **NHS England** (regions, offices, hubs, health and justice): `wrong`; the NHS England code if the value
  spells one (`unkeyed`), else none. Never a PPON.
- **Anything else**: `wrong`, no correction, `none`.
- `right` only if the value IS this org's valid identifier under GB-NHS (not expected). `skip` if the org is
  gone or you cannot settle it; say what would settle it.
- `high` only when the evidence is plain. Mixed mention names (another body's or a supplier's) are never
  `high` for a merge.
