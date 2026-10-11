# Rubric — a register number and a PPON published together for one party (issue 469, unit 2)

Read this beside the issue 448 rubric (`.scratch/tender-db/448-campaign/rubric.md`). This rubric
changes three things: the register is no longer Companies House, the register org SURVIVES (it
is `keep`), and this read decides whether a whole class of pairs can be built at all, so wrong
CLASSES matter as much as wrong pairs.

## What you see per case

- `key`: `<scheme>-<value as published>~<ppon>`. Schemes: `GB-UKPRN` (UK Register of Learning
  Providers), `GB-CHC` (Charity Commission for England and Wales), `GB-MPR` (the FCA Mutuals
  Public Register: co-operative and community benefit societies, many of them housing
  associations, numbers like `30938R`, `RS007639`, `IP6243R`, `7522`), `GB-SC` (OSCR, the
  Scottish charity register, `SC0xxxxx`), `GB-NIC` (the Charity Commission for Northern Ireland,
  `NIC1xxxxx`) and `GB-NHS` (NHS ODS codes).
- `members`: the register org (`identifier` `GB<SCHEME><value>`; for NHS it can read `None`
  when the code minted no identity) and the PPON org (`GBPPON…`). The PPON org can carry a
  company number instead when it is a 448 survivor (it then holds the PPON as a merged
  identifier).
- `facts`: per org, from the public API: `mentions` (how many notice mentions the org has),
  `merged_identifiers`, and how many tenders it is the `buyer` / `winner` / `bidder` on (capped
  at 50; `*_more` says there are more).
- `register`: what the register says about the value, from the fetcher's cache: found or not,
  name, status, other or previous names, linked numbers (a linked charity, a company number), and
  notes. `found: false` with a working fetch means the value is NOT a number in that register.

## Register verdict (`register_verdict`)

- `confirmed`: the register finds the value, it is live (registered / active), and the PPON
  org's name is the register's current or a previous name, a working name, or a clear spelling
  variant of it.
- `contradicts`: the register finds the value and the PPON org's name is a DIFFERENT body.
- `mislabelled`: the value is not a number of the stated register (a society number published
  as a charity number, a company number published as a charity number, a PPON pasted into an
  NHS field, an 11-digit "UKPRN"). Say which register it belongs to if you can tell.
- `inactive`: found, but removed / dissolved / closed. Say what replaced it if the register says.
- `unavailable`: the fetcher could not read the register for this value.

## Verdict (`verdict`, `confidence`)

As in 448: `merge` means the PPON org IS the register's body; `keep` means two different
bodies; `needs-more-evidence` says what would settle it. HIGH means you would execute it
yourself. A register-confirmed pair can still be a `keep` (the register confirms the number,
not the PPON). For a mislabelled value, judge the PARTY: if both orgs are plainly one body,
say `merge` and let the shape gate refuse the value; record the class.

## Wrong class (`class`)

One of:
- `none`: a true pair.
- `consortium-agent`: the PPON belongs to a purchasing consortium, a framework operator or an
  agent that publishes on members' behalf (one PPON paired with many members' numbers).
- `trading-subsidiary`: a charity, university or society beside its trading or services
  company (`X Services Limited`).
- `university-company`: a university beside a company it owns.
- `group-member`: a housing (or other) group beside a member association, or the reverse.
- `linked-charity`: a linked charity (`1234567-1`) beside its main charity.
- `renamed-or-merged`: the same body under an old and a new name, or two bodies that merged
  into the one the register now shows. This is a TRUE pair when the register's current body is
  the PPON's body.
- `mislabelled-scheme`: see above.
- `polluted-name`: one org's name is not a name (a notice title, an event).
- `site-vs-trust` (NHS): a site or department code beside its trust's PPON, or the reverse.
- `different-body`: two unrelated bodies.
- `other`: say what.

Output exactly one entry per case, with a two-sentence rationale that names the evidence (a
register name, a status, a linked number, a name match, or a stray).
