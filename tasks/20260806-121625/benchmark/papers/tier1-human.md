# Tier 1 - locate

You are working in the repository itself. You are the human control.

Write your output to `/out`. Nothing else you write is kept.

There are **8** questions. They are numbered non-consecutively;
that is expected and means nothing.

## How to answer

Answer at the grain the question asks for - crate, module, file or symbol. Some
questions have two or three parts; answer every part, they are graded
separately. "None" and "nowhere" are legitimate answers to some of these.

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

**t1-008.** What is in the folder crates/nova_gameplay/src/hud/? Give the rough size split of what lives there.

**t1-016.** Name every module that implements author-time content lint - everything `content -- lint` runs.

**t1-020.** What is actually in crates/nova_modding?

**t1-023.** The mods screen, the scenarios screen and the portal Explore tab all render a list plus a details pane. Where does that shared composition live?

**t1-026.** NovaGameplayPlugin has a `render: bool` field documented as controlling whether the render-side plugins - meshes, HUD, particles - are added. Which plugins does it actually control?

**t1-029.** Which crate holds the engine-free serde types for the mod wire format - the bundle manifest, the installed catalog and the mod metadata?
