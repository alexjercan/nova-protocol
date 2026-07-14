# Retro: RON scenario/mod format + built-in port

- TASK: 20260525-133028 (family: 133029, 083326, 103622, 091336)
- BRANCH: modding-language
- REVIEW ROUNDS: 2 (APPROVE)

Process only; what/why is in TASK.md, family status in the spike fix-records
(083224, 091336), findings in REVIEW.md.

## What went well

- **Bottom-up, one green commit per layer** (leaf serde -> AssetRef -> container
  serde -> loader crate -> wiring -> port). The workspace built feature-on and off
  at every step, so the big cross-cutting `AssetRef` change was reviewable in
  isolation and never left a broken tree. 11 commits, no reverts.
- **Generated the data files by serializing the code configs, not hand-authoring.**
  A parity test rebuilds each config and asserts byte-equality with the committed
  RON, so the 55 KB shakedown file is provably faithful and every RON-syntax gotcha
  (Color as `Srgba((..))`, Quat as a bare tuple, enum newtype form) was sidestepped.
- **Checkpointed the scope fork with the user** (AskUserQuestion) before the big
  Tier-2 refactor, rather than silently committing to hours of nova_gameplay work.
- **Out-of-context 3-agent review** (correctness/tests/design) on a branch I
  implemented myself caught the one real gap a same-session read would have
  rationalized away.

## What went wrong

- **Scope was discovered incrementally, not up front.** I first believed only two
  `Handle<Image>` fields blocked serialization ("just an ImageRef"); the serde pass
  then surfaced 13 section handles plus three foreign non-serde types
  (`FlightVerb`/`SectionConfig`/`Binding`), i.e. a whole second tier. Root cause: the
  design spike reasoned about the config tree from the top types without grepping the
  leaves for non-derivable members before estimating.
- **R1.1 (MAJOR): the `ScatterObjects` action had no headless effect test.** I
  unit-tested the pure `ScatterRegion::sample` helper (easy) and leaned on the one
  windowed example for the spawn loop. Root cause: tested the sub-function that was
  trivial to test and treated the non-asserting example as integration coverage,
  when the `NovaEventWorld` action-test harness (the despawn test) was right there.
- **A `tatr new` ID collision** overwrote one task: two `tatr new` in one bash call
  in the same second. Known gotcha, repeated.
- **Two honesty MINORs (R1.3/R1.4):** a task note ("tier 2 as a follow-on") and a doc
  claim ("engine build stays serde-free") were written mid-flight and went stale as
  the work delivered tier 2 and enabled serialize workspace-wide. Root cause: wrote
  forward-looking framing and did not circle back when the outcome changed it.

## What to improve next time

- On any serde/derive migration, first grep the whole target type tree for raw
  `Handle`s, foreign-crate types, and Reflect-only types - scope hides in the leaves.
- When adding an action/system with a spawn or mutation effect, write the headless
  effect assertion (fire -> drain -> assert on the world) in the same pass; a passing
  pure-helper test plus a non-asserting example is not effect coverage.
- Space `tatr new` calls (one per invocation); never chain two in a second.
- Treat mid-flight "follow-on"/"stays X" notes as provisional; reconcile them against
  the actual outcome at close-out, before review has to.

## Action items

- [x] Follow-up spike created for the generated-RON duplication (tatr 20260714-110502).
- [x] Catch-all confirmed no new events/filters were needed (tatr 20260714-103611, closed).
- [x] Lessons ledger updated (serde-scope-grep, generate-data-from-code,
      effect-not-just-helper; bumped tatr-same-second-collision).
