# Retro: Objective conveyance visuals

- TASK: 20260712-093831
- BRANCH: objective-conveyance-visuals (landed as 63293fd)
- REVIEW ROUNDS: 2 (round 1: 1 MAJOR, 3 MINOR, 3 NIT; round 2 APPROVE)

## What went well

- The spike carried the whole design load: colors, pulse constants, the
  dedupe rule, the teardown hazard and the per-beat attach map were all
  written down before /plan, so /work was near-mechanical and every piece
  had a proven template one file away (beacon_chips, target_candidates,
  BeaconBlink, the despawn action). Zero structural surprises during
  implementation is what a good spike buys.
- The ledger paid out concretely: teardown-clears-emphasis was designed in
  BEFORE review because state-diff-aliases-reset (20260712-125342) was
  fresh; the sed rename asserted its replace count
  (verify-scripted-edits-applied); the glow-test clock cited the
  Time<Virtual> clamp lesson (20260525-133025); the landing ran
  branch-guarded while master moved mid-cycle (ammo work) and absorbed it
  via the merge-then-squash discipline without drama.
- The fresh-context review pass caught the one real bug (R1.1, below) AND
  independently re-derived two load-bearing claims - the attach-after-spawn
  FIFO ordering and colors-do-not-wake-layout - turning "the implementer
  believes" into "the reviewer traced". Eighth catch for this pattern.

## What went wrong

- R1.1 (MAJOR): the item-highlight bracket used ApparentSize, which unions
  the anchor subtree's ColliderAabbs - and a salvage crate's only collider
  is its 8u pickup SENSOR, so the bracket ballooned to the trigger volume
  instead of hugging the 1.5u crate. Root cause: the widget mode was
  consumed by its doc line ("track the anchor entity's on-screen extent")
  without reading what it actually measures against THIS anchor's
  component shape. Every prior ApparentSize consumer anchors ships whose
  colliders ARE their visible hull, so the mode's assumption (colliders =
  visible extent) was invisible until the first sensor-only anchor. The
  plan even had a verify-first clause for the crate ("verify where the
  material handle lives") but aimed it at the wrong hazard. Sibling of
  advertised-but-unwired: the capability was wired and working; its DATA
  SOURCE just meant something different on this entity.
- R1.3 (MINOR) exposed a test-rig blind spot worth generalizing:
  run_system_once registers a fresh system every call, so Res::is_changed
  is ALWAYS true inside it - any change-detection-gated branch is
  untestable that way and silently looks covered. The gates were correct,
  but only an App-driven test can prove they stay so.
- Cross-checkout LSP noise: mid-implementation, diagnostics for a
  parallel session's uncommitted field (infinite_ammo) appeared against
  files of THIS worktree's snapshot, looking exactly like compile errors
  in just-edited code. Cost a verification detour (grep the worktree's
  actual struct); worth knowing the failure mode exists.

## What to improve next time

- Before consuming a generic widget/system mode on a NEW kind of entity,
  read what the mode measures and check it against that entity's actual
  components - "works for ships" does not transfer to a sensor-only prop.
  One query-line of reading (target_world_aabb) would have caught R1.1 at
  /plan time.
- Any system branch gated on Res::is_changed / Added gets an App-driven
  test across real frames from the start; run_system_once tests only the
  always-changed path.
- Treat IDE diagnostics that reference symbols absent from the worktree
  as parallel-session noise: verify with grep against the checked-out
  tree before acting.

## Action items

- [x] Ledger: new `generic-mode-vs-this-anchor` variant recorded under
      advertised-but-unwired; new `run-system-once-always-changed`;
      out-of-context-review-pass bumped to x8;
      landing-checkout-not-yours reinforced (guarded landing, third
      clean exercise).
- [x] Parent spike fix record updated
      (tasks/20260712-092926/SPIKE.md).
- [ ] Still open (inherited, not new): the human visual playtest owns the
      feel calls - gold vs cyan readability, breath/pulse rates, glow
      band, colorblind check, marker-vs-red-reticle on the pirate
      (spike 20260712-140842 Open questions).
