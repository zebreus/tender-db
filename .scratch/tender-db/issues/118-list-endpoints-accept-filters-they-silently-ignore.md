# 118 — the list endpoints accept filters they silently ignore

Status: needs-triage — filed 2026-08-03 (sdk-vendor). **Observation, not an asserted defect**: the
behaviour may be deliberate, and the question is which contract we want. See "The judgement call".
Kind: API contract / honest data
Blocked by: —
Relates to: 114 (found by its mechanical enumeration), 117 (same reads, different axis)

## What was found

`/v1/organizations?cpv=4521` returns **every** organization, with nothing in the response to say the
`cpv` filter was dropped. There are **16 such (collection, parameter) pairs**, verified by planning the
statement each one actually emits and comparing it byte-for-byte with the unfiltered statement:

| collection | honoured | **accepted and ignored** |
|---|---|---|
| `/v1/tenders` | buyer, country, cpv, kind, max_value, min_value, source, status, winner | `tender` |
| `/v1/lots` | buyer, country, cpv, kind, max_value, min_value, source, status, tender, winner | — |
| `/v1/organizations` | buyer, country, kind | `cpv, max_value, min_value, source, status, tender, winner` |
| `/v1/notices` | kind, source | `buyer, country, cpv, max_value, min_value, status, tender, winner` |

"Ignored" here is not an inference from reading the code. The emitted SQL for, say,
`/v1/notices?country=DE` is **byte-identical** to the SQL for `/v1/notices` with no parameters at all.

## Why it happens

`Params::filter` builds one `Filter` for every collection, and each builder uses only the fields
meaningful to it. There is no per-collection notion of which parameters apply, so anything a builder
does not read is dropped in silence. `read::organizations`' doc comment states the intent plainly —
"Only `country` and `kind` (the identifier kind) narrow them — the value/CPV/status predicates are
Tender-shaped and have no meaning here" — so the *behaviour* is understood. What is missing is any
signal of it reaching the client.

## Why it is worth a decision rather than a shrug

**Every one of these parameters is honoured on some other collection.** `country` narrows tenders, lots
and organizations, and is dropped by notices. `cpv` narrows tenders and lots, and is dropped by
organizations and notices. So a client cannot infer from the parameter's name, or from the API's
behaviour, whether it applied — the same spelling means "filtered" on one path and "ignored" on another,
with an identical-looking 200 either way.

The consequence is a wrong answer that looks right. A client asking for German notices gets every
notice, ordered by id, and no error. If it paginates and counts, it reports a "DE notice count" equal to
the whole corpus. That is the failure mode this project cares about most: not a crash, but data that is
confidently wrong and carries no marker saying so.

`tenders?tender=` is the mildest of the 16 — a Tender filter on the Tenders collection is arguably
meaningless rather than misleading — and is listed for completeness rather than as a concern.

## The judgement call

Three coherent contracts; this issue is not asserting which is right.

1. **Reject** — `400` naming the parameter and the collection ("`cpv` does not apply to
   `/v1/organizations`"). Unambiguous, and it turns a silent wrong answer into a fixable client bug. It
   is a breaking change for any client currently sending a parameter that has always been ignored.
2. **Report** — honour what applies and echo the dropped parameters in the response envelope (e.g.
   `"ignored_filters": ["cpv"]`). Non-breaking, and it makes the response self-describing, which suits a
   data API whose users are counting things.
3. **Document only** — say in `/docs` which parameters apply per collection and leave the behaviour. The
   cheapest, and the one that leaves the misleading-count failure in place.

Whichever is chosen, it should be **uniform across all four collections**, since the current
inconsistency is the part that makes it hard for a client to reason about.

## How it was found, and why that matters

Not by reading the code with a question in mind — by **enumerating the paginated read set mechanically**
(issue 114 part 2): every (collection × filter × cursor position) combination the public API can
produce, each statement extracted from its own builder, deduplicated by emitted text. The dedup is what
surfaced this: 88 candidates collapsed to 56 distinct statements, and the collapses were exactly the
cases where a parameter changes nothing.

That is the same argument 114 makes about coverage. This was not on anyone's list to check, and it was
not found by recall; it fell out of deriving the set. Recorded here because the *method* earning a
finding is itself evidence for the method.

## Verification notes

- The statement comparison is exact (byte equality of emitted SQL), so the "ignored" column carries no
  judgement — only the honoured/ignored split does, and it is mechanical.
- **The live API was deliberately not probed.** Several of these paths are the 22–99 s reads of issue
  117, and firing them at production to confirm something the emitted SQL already settles would be a
  self-inflicted outage. The claim here rests on the builders' own output, not on a request.
- No timing or cost claim is made. This is a correctness/contract observation; it is orthogonal to 117,
  which concerns how the same reads are planned.
