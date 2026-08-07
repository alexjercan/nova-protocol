# Grade a tier 1 answer sheet

You are grading, not answering. You have no repository and you do not need one:
the answer key carries a `file:line` citation for every expected answer, and
those citations were verified against the tree when the key was written.

Two files are mounted:

- `/grade/key.json` - the answer key. `expect` is the expected answer,
  `citation` is the evidence, `grain` is the granularity at which an answer
  counts as right, and `notes` defines partial credit for that specific
  question.
- `/grade/answers.json` - the sheet to grade.

## Rules

Score each answer from `0.0` to `1.0`.

The key is written in the older `right` / `partial` / `wrong` vocabulary. It
still rules; those words are the anchors of the scale:

| Key says | Score |
| --- | --- |
| matches `expect` at the stated `grain`, or is an equivalent statement of it | `1.0` |
| the partial case `notes` describes for that question | `0.5` |
| wrong crate, module or file; or contradicted by `citation`; or named WRONG in `notes` | `0.0` |

Rules that constrain the scale:

- **Where `notes` fixes a value, it wins.** "Any two of three is partial" is
  `0.5`, not `0.67`. "Both halves must be right" means either half failing is
  `0.0`. Something `notes` calls WRONG rather than partial is `0.0` however
  close it looks.
- **Where `notes` describes no partial case, the question is `1.0` or `0.0`.**
  Do not invent partial credit the key did not grant.
- **Multi-part questions `notes` does not pin:** score the fraction of parts
  answered correctly - two of three is `0.67`. Say in `why` which part failed.
- **Between the anchors:** `0.25` and `0.75` are available for an answer that
  is clearly weaker or stronger than the `0.5` case `notes` describes. Use them
  sparingly and justify them. Do not reach for two decimal places outside a
  part fraction.
- **Wording does not matter.** A more specific answer than `grain` asks for is
  still `1.0`.

Set `gave_up: true` when the answer is an honest refusal - it says it does not
know. Score it `0.0`. An answer that hedges into uselessness ("somewhere in the
gameplay crate, possibly") is `0.0` with `gave_up: false`: a confident wrong
answer and an honest refusal are the same points and a different finding.

Missing entries: if a question in the key has no matching answer, score `0.0`,
`gave_up: true`, `why: "no answer submitted"`.

**Do not reward length and do not penalise brevity.** A one-line answer naming
the right path is `1.0`. A page of prose that names the same path is also
`1.0` - no more. Judge only whether the answer locates the thing. Ignore
everything the answer says about how it was found.

## Output

Write `/out/grades.json`:

```json
{
  "grades": [
    {
      "id": "t1-001",
      "score": 0.5,
      "gave_up": false,
      "why": "one sentence, citing the key's expect or notes"
    }
  ]
}
```

One entry per question in the key. No prose outside the file.
