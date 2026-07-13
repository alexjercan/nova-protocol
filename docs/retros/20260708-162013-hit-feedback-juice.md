# Retro: Hit feedback / game juice (camera shake, hit flash, impact FX)

- TASK: 20260708-162013
- BRANCH: feature/hit-feedback-juice (squash-merged as f4f24df)
- REVIEW ROUNDS: 1 (APPROVE with 2 MINORs + 1 NIT; both MINORs addressed before merge)

See `tasks/20260708-162013/TASK.md` for what shipped and
`docs/retros/20260709-hit-feedback-juice.md` for the design. This retro is about how the
working went.

## What went well

- **The audio retro's top lesson paid off immediately.** Before writing any shake
  code I searched bcs for an existing camera-shake module and found a complete,
  drift-free `CameraShakePlugin` (trauma model, ordered around the chase camera).
  The camera-shake half of the task collapsed from "implement shake" to "feed it
  trauma and attach the component" - the exact "read the bcs source first" win the
  previous cycle wrote down. Reading `audio.rs` as a template also made the whole
  module fall out fast: same seams, same distance-attenuation + per-cell throttle
  shape, same "pure helpers carry the testable logic" split.
- **Modelling on the sibling feature kept it consistent and small.** Because juice
  and sound are the same problem (react to a few events, attenuate by distance,
  collapse co-located bursts), copying `audio.rs`'s structure gave reviewers nothing
  surprising to flag - the one APPROVE round had only MINORs, no design pushback.
- **Applied the warm-target-dir + `cargo test --workspace` habits from the last
  three retros without being reminded.** Builds in the sprout stayed at seconds, and
  the wasm-safety claim was actually verified (`cargo check --target
  wasm32-unknown-unknown`) rather than asserted.

## What went wrong

- **R1.2 (no observer-level test) - the same gap the audio module shipped with.**
  I unit-tested every pure helper but not the wiring (that the observers actually
  feed trauma and queue a flash). Root cause: I inherited `audio.rs`'s test shape
  wholesale, and audio only tests helpers - so I copied its blind spot along with its
  strengths. AGENTS.md explicitly prefers integration tests; a minimal-`App` test was
  cheap and I should have written it first, not after review asked. Lesson: when
  copying a module as a template, copy its structure but re-check it against the
  standards (here, "prefer integration tests") rather than assuming the template
  already meets them.
- **R1.1 (nested reflect types unregistered).** I registered `JuiceSettings` but not
  its nested `ShakeSettings`/`FlashSettings`, so the debug inspector and the
  settings-menu-to-be would have seen the sub-structs as unregistered. Root cause:
  "make it tweakable via a resource" was the stated point of the task, yet I stopped
  at reflecting/registering the root instead of thinking through the whole traversal
  the inspector actually does. When a feature's *purpose* is reflection/editing,
  register the full type tree, not just the entry point.
- **Playtest feedback (too much shake) arrived only because the user ran it, not
  because I reasoned about feel.** The first defaults were tuned by gut and were too
  strong. I had already built distance attenuation in, but the base impulses were set
  high and the shake falloff used the *audio* distances (near 20 / far 320), so
  nearly all combat was inside the full-strength radius. Root cause: I reused the
  audio scene-scale constants for a different effect without asking whether shake
  should fall off on the same curve as sound - it should fall off faster. Lesson: a
  reused magic number is a decision, not a default; re-justify tuning constants for
  the new effect instead of inheriting them.

## What to improve next time

- When cloning a sibling module as a template, list what it does *not* do (here:
  integration tests) and decide deliberately whether to inherit that gap.
- For any "make it configurable/tweakable" task, register/serialize the entire
  reflected tree and sanity-check it through the actual consumer (inspector), not
  just the root type.
- Treat reused tuning constants as choices to re-justify for the new context,
  especially perception-tuned ones (shake vs audio falloff).

## Action items

- [x] Both review MINORs addressed on-branch before merge (nested reflect
  registration + observer-level integration tests).
- [ ] Standing lesson worth promoting if it recurs: "prefer integration tests" bit
  me the same way two cycles running (audio, juice). If a third event-observer
  feature ships helper-only tests, promote "event-driven modules must have at least
  one observer-level `App` test" into AGENTS.md or the `/work` skill rather than
  leaving it in retros.
- [ ] Optional follow-up (NIT R1.3, not filed as a task): source impact FX from the
  collision contact point rather than the damaged entity's transform, if a precise
  spark location is ever wanted.
