# Tier 1 - locate

You are working in the repository itself, with everything you already know
about it. You are the control: your job is to establish the ceiling and to
validate that the questions are answerable at all.

Write your output to `/out`. Nothing else you write is kept.

## Style

Be terse. Fragments over sentences, bullets over prose. No preamble, no
restating the question, no summary of what you are about to do. Name the thing
and stop.

Length is not evidence. An answer that names the right path in one line scores
exactly what a page naming the same path scores. Padding a thin answer does not
move it: the grader scores what you located, never how much you wrote.

There are **5** questions. They are numbered non-consecutively;
that is expected and means nothing.

## How to answer

Answer at the grain the question asks for - crate, module, file or symbol. Some
questions have two or three parts; answer every part, they are scored
separately. "None" and "nowhere" are legitimate answers to some of these.

**Keep `answer` under 40 words.** Paths and symbols, plus at most one clause of
justification. Do not describe what each file does, do not explain how you
found it, do not hedge in both directions.

If you cannot answer, use `"gave-up"` as the answer and record the tool calls
you spent getting there.

**Do not guess silently.** A confident wrong answer and an honest `gave-up` are
graded differently, and the difference is the point of the exercise.

## Output

Write `/out/answers.json`:

```json
{
  "answers": [
    {
      "id": "t1-001",
      "answer": "your answer, at the grain asked",
      "tool_calls": 3,
      "detours": ["paths you opened that were not on the path to the answer"],
      "confidence": "high | medium | low"
    }
  ]
}
```

One entry per question below, in order. `tool_calls` is your own count for that
question; the harness counts independently from the transcript, so do not
inflate or round it - the two are compared.

Record `confidence` before you check anything. It is colour, not a score.

## Questions

**t1-004.** Where does a probe run write frametime.csv, and which process writes it - the game under test or the harness that launched it?

**t1-005.** Which crate owns the colour palette the HUD draws with?

**t1-016.** Name every module that implements author-time content lint - everything `content -- lint` runs.

**t1-020.** What is actually in crates/nova_modding?

**t1-029.** Which crate holds the engine-free serde types for the mod wire format - the bundle manifest, the installed catalog and the mod metadata?
