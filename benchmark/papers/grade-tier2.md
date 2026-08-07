# Grade a tier 2 design note

You are grading, not answering. Three files are mounted:

- `/grade/key.md` - the ground-truth surface lists and the rubric. Find the
  section for the task named in `/grade/task.txt` and grade against that
  section only.
- `/grade/task.txt` - which task this is (`tier2a`, `tier2b` or `tier2c`).
- `/grade/NOTES.md` - the design note to grade.

Also mounted: `/grade/owner-tool-calls.txt`, the owner's tool-call count for
this task, and `/grade/tool-calls.txt`, the respondent's. Use them for the
Cost of arrival dimension.

## Rules

Score the four rubric dimensions 0-3 using the scoring table in the key. Every
score needs a citation - a quote from `/grade/NOTES.md`, or the specific
required surface that is absent from it.

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
    "ownership": 3,
    "completeness": 2,
    "no_phantom_structure": 3,
    "cost_of_arrival": 1
  },
  "citations": {
    "ownership": "why, quoting the note or naming what is absent",
    "completeness": "...",
    "no_phantom_structure": "...",
    "cost_of_arrival": "..."
  },
  "missed_required": ["paths from the key's Required list the note never names"],
  "phantom_paths": ["paths the note names that do not exist"],
  "uncertain": ["anything you could not judge without the source tree"]
}
```

The headline number is the total out of 12; the harness computes it. No prose
outside the file.
