# Grade a tier 2 design note

You are grading, not answering. Three files are mounted:

- `/grade/key.md` - the ground-truth surface lists and the rubric. Find the
  section for the task named in `/grade/task.txt` and grade against that
  section only.
- `/grade/task.txt` - which task this is (`tier2a`, `tier2b` or `tier2c`).
- `/grade/persona.txt` - which persona wrote the note.
- `/grade/NOTES.md` - the design note to grade.

Also mounted: `/grade/owner-tool-calls.txt`, the owner's tool-call count for
this task, and `/grade/tool-calls.txt`, the respondent's. Both are counted from
the transcript, not self-reported. Use them for the Cost of arrival dimension.

If `/grade/owner-tool-calls.txt` reads `not recorded`, **write `null` for
`cost_of_arrival`** and say so in its citation. The dimension is defined as a
ratio against the owner's count; with no denominator there is no score. Do not
substitute an absolute judgement of the respondent's count, and do not use the
0.67 anchor as a neutral default. The harness takes the mean of the dimensions
you did score.

## Rules

Score each of the four rubric dimensions as **one number in `[0.00, 1.00]`,
rounded to two decimals**, using the anchor table in the key. Any value in that
range is available - the anchors are reference points, not buckets, so a note
between two rows scores between them. Never write a grade name into a score
field. Every score needs a citation - a quote from `/grade/NOTES.md`, or the
specific required surface that is absent from it.

Before scoring Completeness, apply the key's **Channel scope** table to the
persona in `/grade/persona.txt`. A Required surface that is out of channel for
that persona is removed from the Required list entirely - it is not a
deduction, and it does not sit in the denominator of the fraction of required
surfaces named. List it in `out_of_channel` instead of `missed_required`.

Read the key's Required / Credit split carefully:

- A missing **Required** surface is a Completeness deduction.
- A missing **Credit** surface is not a deduction. Naming one is evidence for a
  higher Ownership or No-phantom-structure score.
- A path the note names that is on neither list is not automatically wrong.
  Judge whether it is defensible, and say which way you judged it.
- A path that **does not exist** is a phantom-structure deduction regardless of
  how reasonable it sounds. The key's own paths are the reference for what
  exists; if you cannot tell, say so in `uncertain` rather than guessing.

Grade the plan, not the prose. A terse note that names every surface beats a
long one that does not. Length is never evidence: a note is scored on the
surfaces it names and the structure it asserts, never on how much it wrote
about them. Do not treat hedging at length as coverage - a surface is named or
it is not.

## Output

Write `/out/grades.json`:

```json
{
  "task": "tier2a",
  "scores": {
    "ownership": 1.0,
    "completeness": 0.67,
    "no_phantom_structure": 0.9,
    "cost_of_arrival": 0.33
  },
  "citations": {
    "ownership": "why, quoting the note or naming what is absent",
    "completeness": "...",
    "no_phantom_structure": "...",
    "cost_of_arrival": "..."
  },
  "missed_required": ["in-channel paths from the key's Required list the note never names"],
  "out_of_channel": ["Required paths dropped for this persona by the Channel scope table"],
  "phantom_paths": ["paths the note names that do not exist"],
  "uncertain": ["anything you could not judge without the source tree"]
}
```

Every value in `scores` is a JSON number in `[0.00, 1.00]` with at most two
decimals - never a string, never a word. The headline number is the mean of the
four; the harness computes it. No prose outside the file.
