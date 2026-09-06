export const meta = {
  name: 'r2-denied-review',
  description: 'Per-group review of the R2 name-gate denials (issue 362): reviewer + adversarial challenger per batch, blind second readers on a sample',
  phases: [
    { title: 'Review', detail: 'one reviewer per 35-case batch: merge | keep | needs-more-evidence' },
    { title: 'Challenge', detail: 'one challenger per batch tries to refute every merge and every high keep' },
    { title: 'Sample', detail: 'blind second readers on every 9th case' },
  ],
}

const pad = (n) => String(n).padStart(2, '0')
const fileOf = (kind, id) => `${args.dir}/${kind}-${pad(id)}.json`

const CASE = {
  type: 'object',
  properties: {
    case: { type: 'string' },
    verdict: { type: 'string', enum: ['merge', 'keep', 'needs-more-evidence'] },
    confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
    rationale: { type: 'string' },
  },
  required: ['case', 'verdict', 'confidence', 'rationale'],
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
          case: { type: 'string' },
          agree: { type: 'boolean' },
          reason: { type: 'string' },
        },
        required: ['case', 'agree', 'reason'],
      },
    },
  },
  required: ['reviews'],
}

const reviewPrompt = (file, keys) => `You are reviewing groups of organization rows from a public-procurement database. FIRST read the rubric at ${args.rubric} in full. THEN read the cases at ${file}: a JSON array of ${keys.length} cases (keys ${JSON.stringify(keys)}). Each case has "case" (the key), "country", "scheme", "key" (the shared register number), "member_ids" and "members" (each with org, name, identifier, kind, country, provisional, mentions).

For EVERY case — all ${keys.length}, none skipped, one entry per case key — decide merge | keep | needs-more-evidence with a confidence, strictly as the rubric says: HIGH merge only when you would fold the rows yourself; HIGH keep only when you are sure they are distinct organizations. Rationale: two sentences naming the signature (rename, acronym, translation, typo, unit of a public body, buyer-and-contractor, consortium row, subsidiary…). Return the structured output only.`

const challengePrompt = (file, keys, verdicts) => `You are the adversarial second pass on organization-group reviews. FIRST read the rubric at ${args.rubric} in full. THEN read the cases at ${file} (JSON array, keys ${JSON.stringify(keys)}). A reviewer produced these verdicts:

${JSON.stringify(verdicts)}

For EVERY case, try to REFUTE the reviewer. For a merge: could these be two different organizations after all (a buyer and its contractor, a subsidiary with its own legal personality, a consortium row, two firms that merely share a word)? Is HIGH over-claimed where the names only suggest a relation? For a high keep: could they be one entity (a rename, acronym, translation, a unit publishing under its parent's number)? Default to disagreeing when uncertain. Return one entry per case with agree (true only if you accept the verdict AT its stated confidence) and a one-sentence reason.`

const stratumOf = Object.fromEntries(args.review.map(([id, s]) => [id, s]))
const reviewItems = args.review.map(([id]) => id)

const reviewed = pipeline(
  reviewItems,
  (id) => agent(reviewPrompt(fileOf('batch', id), args.roots[id]), { label: `review:${pad(id)}`, phase: 'Review', schema: BATCH, model: args.model }),
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
