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

Grade each answer `right`, `partial`, `wrong` or `gave-up`.

- `right` - matches `expect` at the stated `grain`, or is an equivalent
  statement of the same thing. Wording does not matter. A more specific answer
  than `grain` asks for is still `right`.
- `partial` - **only** as `notes` for that question defines it. If `notes` does
  not describe a partial case, there is no partial credit for that question:
  it is `right` or `wrong`. Where `notes` names something as WRONG rather than
  partial, honour that.
- `wrong` - names the wrong crate, module or file, or asserts something the
  citation contradicts.
- `gave-up` - the answer says so. An answer that hedges into uselessness
  ("somewhere in the gameplay crate, possibly") is `wrong`, not `gave-up`;
  `gave-up` is for an honest refusal.

Multi-part questions: grade the whole entry by its weakest part, and say in
`why` which part failed.

Do not reward confidence and do not penalise brevity. Judge only whether the
answer locates the thing.

Missing entries: if a question in the key has no matching answer, grade it
`gave-up` with `why: "no answer submitted"`.

## Output

Write `/out/grades.json`:

```json
{
  "grades": [
    {
      "id": "t1-001",
      "grade": "right | partial | wrong | gave-up",
      "why": "one sentence, citing the key's expect or notes"
    }
  ]
}
```

One entry per question in the key. No prose outside the file.
