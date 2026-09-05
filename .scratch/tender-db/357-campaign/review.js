export const meta = {
  name: 'cluster-country-review',
  description: 'Per-case review of identifier-under-several-codes clusters (issue 357): reviewer + adversarial challenger per stratified batch, blind second readers on a sample',
  phases: [
    { title: 'Review', detail: 'one reviewer per 20-case batch, structured verdicts with country moves' },
    { title: 'Challenge', detail: 'one challenger per batch tries to refute every move' },
    { title: 'Sample', detail: 'blind second readers on every 9th case' },
  ],
}

const pad = (n) => String(n).padStart(2, '0')
const fileOf = (kind, id) => `${args.dir}/${kind}-${pad(id)}.json`

const MOVE = {
  type: 'object',
  properties: {
    org: { type: 'integer' },
    from: { type: ['string', 'null'] },
    to: { type: 'string' },
    evidence: { type: 'string', enum: ['arithmetic', 'national-format', 'identical-identifier-weight', 'name-language'] },
    confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
    rationale: { type: 'string' },
  },
  required: ['org', 'from', 'to', 'evidence', 'confidence', 'rationale'],
}
const CASE = {
  type: 'object',
  properties: {
    identifier: { type: 'string' },
    verdict: { type: 'string', enum: ['wrong-country', 'same-entity-two-registrations', 'distinct-entities', 'needs-more-evidence'] },
    confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
    rationale: { type: 'string' },
    country_moves: { type: 'array', items: MOVE },
  },
  required: ['identifier', 'verdict', 'confidence', 'rationale', 'country_moves'],
}
const BATCH = { type: 'object', properties: { verdicts: { type: 'array', items: CASE } }, required: ['verdicts'] }
const CHALLENGE = {
  type: 'object',
  properties: {
    reviews: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          identifier: { type: 'string' },
          agree: { type: 'boolean' },
          disputed_moves: { type: 'array', items: { type: 'object', properties: { org: { type: 'integer' }, reason: { type: 'string' } }, required: ['org', 'reason'] } },
          missed_move: { type: 'boolean' },
          note: { type: 'string' },
        },
        required: ['identifier', 'agree', 'disputed_moves', 'missed_move', 'note'],
      },
    },
  },
  required: ['reviews'],
}

const reviewPrompt = (file, roots) => `You are reviewing organization rows from a public-procurement database. FIRST read the rubric at ${args.rubric} in full — it defines the evidence fields, the verdicts, the move evidence classes and the confidence bar. THEN read the cases at ${file}: a JSON array of ${roots.length} cases (identifiers ${JSON.stringify(roots)}). Each case is one register number under several country codes: "identifier" (the case key), the census's reading ("verdict", "codes", "asked", "named", "named_schemes", "one_letter_pair", "heavy_one_letter", "mentions" per code, "total_mentions") and "members" (the organization rows, each with org, country, identifier_kind, identifier, anchors, country_probed, country_agrees, mentions, name, variants, notices).

For EVERY case — all ${roots.length}, none skipped, one entry per identifier — decide the verdict and, only for wrong-country, the country_moves. A move names the exact row (its "org" id), "from" MUST equal that row's current "country" (null when the row has none), and "to" is where the evidence puts it. Assign the evidence class and confidence strictly as the rubric says: HIGH only when you would execute the rewrite yourself; never move a row whose country_agrees is true; never move on weight alone; silence (not probed, no anchors) is not evidence. Rationale: two or three sentences naming the fields you used. Return the structured output only.`

const challengePrompt = (file, roots, verdicts) => `You are the adversarial second pass on organization-row reviews. FIRST read the rubric at ${args.rubric} in full. THEN read the cases at ${file} (JSON array, identifiers ${JSON.stringify(roots)}). A reviewer produced these verdicts:

${JSON.stringify(verdicts)}

For EVERY case, try to REFUTE the reviewer. For each country move, check LITERALLY against the case fields: does the claimed evidence class hold (is the "to" scheme really in that row's anchors; is country_agrees really false on the moving row and true on the survivor; are the mention weights what the rationale says; is the national format really specific to "to")? Is there an innocent reading — a foreign branch or subsidiary with its own registration, a VAT id issued by the row's own country, a row with real standing (many mentions) being moved on thin evidence? Is HIGH over-claimed where the rubric says MEDIUM? Dispute a move when any of that fails; default to disputing when uncertain. Also flag a MISSED move: a case marked distinct-entities or needs-more-evidence where the fields clearly support a wrong-country move under the rubric. Return one entry per identifier with agree (true only if you accept the verdict AND every move at its stated confidence), the disputed moves with reasons, missed_move, and a one-sentence note.`

const stratumOf = Object.fromEntries(args.review.map(([id, s]) => [id, s]))
const reviewItems = args.review.map(([id]) => id)

const reviewed = pipeline(
  reviewItems,
  (id) => agent(reviewPrompt(fileOf('batch', id), args.roots[id]), { label: `review:${pad(id)}:${stratumOf[id]}`, phase: 'Review', schema: BATCH, model: args.model }),
  async (r, id) => {
    if (!r) return null
    const c = await agent(challengePrompt(fileOf('batch', id), args.roots[id], r.verdicts), { label: `challenge:${pad(id)}`, phase: 'Challenge', schema: CHALLENGE, model: args.model })
    return { id, stratum: stratumOf[id], verdicts: r.verdicts, challenge: c ? c.reviews : null }
  },
)
const sampled = parallel(args.samples.map((id) => () =>
  agent(reviewPrompt(fileOf('sample', id), args.roots[id]), { label: `sample:${pad(id)}`, phase: 'Sample', schema: BATCH })
    .then((r) => (r ? { id, verdicts: r.verdicts } : null))))

const [batches, samples] = await Promise.all([reviewed, sampled])
const done = batches.filter(Boolean)
log(`${done.length}/${reviewItems.length} batches reviewed and challenged; ${samples.filter(Boolean).length}/${args.samples.length} sample batches read`)
return { batches: done, samples: samples.filter(Boolean) }