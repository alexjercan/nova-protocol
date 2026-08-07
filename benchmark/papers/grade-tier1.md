# Grade a tier 1 answer sheet

You are grading, not answering. You have no repository and you do not need one:
the answer key carries a `file:line` citation for every expected answer, and
those citations were verified against the tree when the key was written.

Two files are mounted:

- `/grade/key.json` - the answer key. `expect` is the expected answer,
  `citation` is the evidence, `grain` is the granularity at which an answer
  earns full credit, and `notes` states the credit values this key fixes for
  that specific question.
- `/grade/answers.json` - the sheet to grade.

## Rules

Score each answer as **one number in `[0.00, 1.00]`, rounded to two decimals**.
Any value in that range is available. There are no grade names - do not think
in buckets, and do not write words like right, partial or wrong into `score`.

The two ends are fixed:

| Answer | Score |
| --- | --- |
| matches `expect` at the stated `grain`, or is an equivalent statement of it | `1.00` |
| wrong crate, module or file; or contradicted by `citation`; or a case `notes` pins at `0.00` | `0.00` |

How to choose everything in between:

- **Where `notes` fixes a value, it wins.** If `notes` says a case is `0.50`,
  it is `0.50` - not the fraction you would have computed. If `notes` pins a
  case at `0.00`, it is `0.00` however close the rest of the answer looks.
- **Multi-part questions `notes` does not pin:** score the fraction of required
  parts answered - one of three is `0.33`, two of three is `0.67`. Say in `why`
  which part failed.
- **Single-part questions where `notes` fixes nothing:** `1.00` or `0.00`. Do
  not invent credit the key did not grant.
- **Adjusting off a fixed value.** Where `notes` fixes a case but the answer is
  clearly weaker or stronger than that case, you may move up or down by `0.25`
  at most - e.g. `0.25` or `0.75` around a `0.50` case. Justify the move in
  `why`. Never move past the end a `notes` value was pinned to.
- **Wording does not matter.** A more specific answer than `grain` asks for is
  still `1.00`.

Set `gave_up: true` when the answer is an honest refusal - it says it does not
know. Score it `0.00`. An answer that hedges into uselessness ("somewhere in
the gameplay crate, possibly") is `0.00` with `gave_up: false`: a confident
wrong answer and an honest refusal are the same points and a different finding.

Missing entries: if a question in the key has no matching answer, score `0.00`,
`gave_up: true`, `why: "no answer submitted"`.

**Do not reward length and do not penalise brevity.** A one-line answer naming
the right path is `1.00`. A page of prose that names the same path is also
`1.00` - no more. Judge only whether the answer locates the thing. Ignore
everything the answer says about how it was found.

## Output

Write `/out/grades.json`:

```json
{
  "grades": [
    {
      "id": "t1-001",
      "score": 0.67,
      "gave_up": false,
      "why": "one sentence, citing the key's expect or notes"
    }
  ]
}
```

One entry per question in the key. `score` is a JSON number in `[0.00, 1.00]`
with at most two decimals - never a string, never a word. No prose outside the
file.
