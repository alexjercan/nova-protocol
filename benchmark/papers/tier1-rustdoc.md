# Tier 1 - locate

Your working directory `/work` holds the rendered `cargo doc` output for
every crate in the workspace: the public API and its doc comments, as HTML.
**The `[source]` pages have been removed** - you cannot read the source, only
what the public API and its documentation disclose.

Write your output to `/out`. Nothing else you write is kept.

There are **27** questions. They are numbered non-consecutively;
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

**t1-001.** Which module owns the terminal and windowing runtime that draws the in-game NOVA OS screens?

**t1-002.** Where is mod bundle merging implemented - the code that flattens and overlays bundles into GameSections and GameScenarios?

**t1-003.** Which file holds the roster that decides what a probe run is checked against?

**t1-004.** Where does a probe run write frametime.csv, and which process writes it - the game under test or the harness that launched it?

**t1-005.** Which crate owns the colour palette the HUD draws with?

**t1-006.** You need to change the order in which gameplay plugins are added to the app. Which files decide that order?

**t1-007.** The NOVA OS monitor has brightness, scanline and sound settings. Which file declares that settings type, which crate registers it as a resource, and which file persists it?

**t1-008.** What is in the folder crates/nova_gameplay/src/hud/? Give the rough size split of what lives there.

**t1-009.** Which crate owns the NOVA OS terminal model, the shell command matcher and the app runtime - and does that crate draw any of it?

**t1-010.** The NOVA OS map app and the ship-viewer app each draw a 3D orbit-camera scene with blips. Name every file that implements that scene code.

**t1-012.** How does an example opt into probe collection today? Name what it has to add.

**t1-013.** crates/nova_probe/src/fixtures.rs - what does it contain, and why is it in nova_probe?

**t1-014.** Where are the built-in ship sections' stats authored - the health, fire-rate and thrust numbers for the stock hull, turret, thruster, torpedo and controller sections?

**t1-015.** Where do the built-in (non-mod) scenario scripts live - the shakedown campaign and its siblings?

**t1-016.** Name every module that implements author-time content lint - everything `content -- lint` runs.

**t1-017.** Which file downloads installed mods, caches them on disk and maintains the installed index?

**t1-018.** What does crates/nova_scenario/src/render_scale.rs do, and is it scenario vocabulary?

**t1-019.** Which crate contains the mod portal network client - the code that talks to the remote catalog and installs from it?

**t1-020.** What is actually in crates/nova_modding?

**t1-021.** crates/nova_debug is described as debug-gated tooling. Is everything in it debug tooling?

**t1-022.** What is nova_events for? Should a gameplay system use it to notify another gameplay system in the same crate?

**t1-023.** The mods screen, the scenarios screen and the portal Explore tab all render a list plus a details pane. Where does that shared composition live?

**t1-024.** Which file or files compute the maximum scroll offset for a scrollable list?

**t1-025.** Which file owns the shared button widget, and how many distinct paint backends does it contain?

**t1-026.** NovaGameplayPlugin has a `render: bool` field documented as controlling whether the render-side plugins - meshes, HUD, particles - are added. Which plugins does it actually control?

**t1-029.** Which crate holds the engine-free serde types for the mod wire format - the bundle manifest, the installed catalog and the mod metadata?

**t1-030.** Which files in crates/nova_os spawn bevy UI nodes?
