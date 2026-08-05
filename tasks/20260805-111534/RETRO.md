# Retro: Authorable scenario lighting: let a scene pose its own lights instead of one hardcoded top-down key

- TASK: 20260805-111534
- BRANCH: feature/authorable-lighting
- REVIEW ROUNDS: 2

## What went well

Step order held: the vocabulary was proven on one entity before ~30 scenes were
edited, so no API flaw surfaced mid-sweep. The plan's refusal to split the
delete from the relight was right - neither order is testable alone.

`ThreePointRig` landing in `nova_scenario::objects::light` rather than
`nova_probe::fixtures` (Step 8's open question) reached both consumer groups
from one helper, and made the screenshot migration exact rather than
approximate: `around(prefix, ZERO, 1.0)` IS the old `kit.rs` rig, verified
constant-by-constant during review.

Both mis-recorded proofs were caught by RUNNING rather than reasoning - the
`RUSTFLAGS=-Dwarnings` guard that was red on master, and the owner's catch that
the webmod content was relit without bumping the bundle versions that carry it
to installed copies.

## What went wrong

Breadth. 59 files, and the size was designed for, not stumbled into: the
decision made "relight every rendering scene" the deliverable, so the sweep IS
the proof that lighting moved into content. No independently landable split
existed. Not a plan failure.

Churn. Round 1's two MAJOR findings were both about EVIDENCE, not code: the
`aim` path (the only path all 8 hand-authored mod files use) was pinned only
under `MinimalPlugins`, and the shadow-map cost of putting a shadow caster in
every scene - where master had none anywhere - was never measured. The plan
asked "would we build this route from scratch?" but never "which authored path
does shipped CONTENT depend on, and is that the path the tests exercise?" The
DoD's three test proofs were written from the API's shape (Directional, Point,
render flag), and the API's shape is not the content's shape: every shipped
scene uses `ThreePointRig` (`aim: None`) and every mod scene uses `aim: Some`,
so the branch the mods depend on had the thinnest coverage.

The round-1 fix over-corrected first. The suspected mechanism (avian seeding
`Position`/`Rotation` from the spawn transform and interpolating a later
`Transform` insert away) was plausible enough that a correct-by-construction
refactor was written before it was tested. Testing it took one scratch rig and
falsified it: the aimed rotation survives 8 ticks with `angle_between == 0`. The
refactor was reverted and only the harness change kept. Cheap here, and the
lesson is the order - falsify, then fix.

Round 2 caught the perf number being over-precise: a 0.25 ms mean delta measured
in one session did not survive a second session's ~0.8 ms between-session
spread. The conclusion (keep the shadows) was unchanged; the stated precision
was not defensible.

## What to improve next time

- When a feature adds an authoring vocabulary, list which VARIANTS shipped
  content actually uses before writing the DoD's test proofs. Coverage that
  follows the enum's shape can leave the variant all the content depends on as
  the least-tested one.
- A single-session A/B frame-time delta needs a same-session repeat AND a
  cross-session check before it is written down as a percentage. Report the
  bound that survives the noise, not the number the run produced.
- Falsify a suspected mechanism before refactoring around it. A scratch rig cost
  minutes; the refactor it justified was reverted whole.
- Context: no pressure observed. No checkpoint, no compaction warning, no
  handoff; both review rounds ran as out-of-context subagents, which kept the
  recording pass small.

## Action items

- None requiring a task. The two improvements above are practice, not work;
  the deferred `base_scenario_object` question (`RigidBody::Dynamic` +
  `TransformInterpolation` on non-physical objects, now two overriders) stays
  recorded in TASK.md for whoever adds the third.

## Landing message

```
feat(scenario): let a scene author its own lights

Lighting moves from the engine into content. A fifth ScenarioObjectKind::Light
(Directional + Point) spawns through the existing SpawnScenarioObject path, and
the loader's hardcoded top-down DirectionalLight is deleted with no fallback -
a scene that authors no light renders black. Breaking for third-party mods.

The relight is the proof: 9 shipped scenarios, 17 hand-authored mod RON files,
17 examples and the editor sandbox each pose their own rig, most through the
shared ThreePointRig helper carrying the screenshot reel's key/rim/fill numbers.
The webmod bundles are republished at bumped versions so installed copies get
the relit content. Documented in modding-ron.md and the scenario guides.
```
