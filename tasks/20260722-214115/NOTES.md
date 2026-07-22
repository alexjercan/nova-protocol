# Ledger per-chapter look - implementation notes

Minimal-look pass (owner decision 2026-07-22): reuse base's TWO existing
cubemaps only - `dep://base/textures/cubemap.png` (calm) and
`dep://base/textures/cubemap_alt.png` (danger/close). NO new image files, NO
`self://` resources, NO bundle `resources:` block. Data-only RON edits: one
`cubemap:` value change (ch3) plus `SetSkybox` accents added into EXISTING
handlers. No engine changes; the shipped `SetSkybox` -> `PendingSkyboxSwap`
mechanism (proven in gauntlet.content.ron) is reused as-is.

## Final per-chapter look

| Chapter | Start cubemap | Mid-scenario accent (beat / handler)                                   | To          |
|---------|---------------|------------------------------------------------------------------------|-------------|
| ch1 Dead Weight    | cubemap.png (calm home belt)  | 4th-ping REVEAL announce ("fourth return" Okono line, `ping_said` 0->1 one-shot) - the belt turns wrong the moment the black box shows up | cubemap_alt |
| ch2 Claim Jumpers  | cubemap_alt.png (opens tense) | none (optional victory-breather swap SKIPPED - see below)                                | -           |
| ch2b The Heavies   | cubemap_alt.png (opens tense) | none (optional victory-breather swap SKIPPED - see below)                                | -           |
| ch3 The Quiet Channel | cubemap.png (running dark/quiet) - CHANGED from alt | debris-PINCH warning ("Channel narrows here" Vesh line, `pinch_warn_said` 0->1 one-shot) - channel closes in | cubemap_alt |
| ch4 The Buyer      | cubemap.png (calm)            | SELL path: Auditor ARRIVAL (handoff_berth OnEnter, act 1->2, "military burn painting you" line) - danger sky | cubemap_alt |

Consecutive chapters now read distinct:
- ch1 calm -> ch2 danger (start): distinct.
- ch2 danger -> ch2b danger: both open on the alt (danger) sky by design - they
  are the campaign's two-part firefight and read as one continuous danger arc;
  ch1's mid-run swap to alt (the 4th-ping reveal) already bridges INTO that
  danger sky, so the belt "turning wrong" carries through the fight pair.
- ch2b -> ch3: alt-start -> calm-start: distinct (the fight pair ends, ch3 runs
  dark/quiet on the calm sky).
- ch3 calm -> ch4 calm: both start calm, but ch3 swaps to alt at the pinch and
  ch4 swaps to alt at the Auditor, so neither stays flat; the two calm starts
  are different beats (quiet-run vs pre-handoff).

## SetSkybox placement rationale + single-fire guards

Every `SetSkybox` sits in the handler for the beat that MOTIVATES it (never at
OnStart - that is what `cubemap:` is for), and reuses that beat's EXISTING
one-shot guard so it fires exactly once and cannot thrash on re-entry:

- ch1: the reveal ANNOUNCE handler is guarded `act==2 && ping_said==0` and sets
  `ping_said=1` in the same actions list - one fire.
- ch3: the pinch WARNING handler is guarded `pinch_warn_said==0` (+ clock past
  `pinch_gate`) and sets `pinch_warn_said=1` - one fire.
- ch4: the handoff_berth OnEnter is guarded `act==1` and sets `act=2` - one fire.

## Optional swaps - decisions

- ch2 / ch2b victory-breather swaps: SKIPPED. First implemented (SetSkybox ->
  cubemap.png in each win comms handler), but this broke the landed
  `ledger_ch2_encounter` fairness rig: that production-faithful rig runs on
  `MinimalPlugins` with NO `AssetServer`, and the `SetSkybox` command reads
  `world.resource::<AssetServer>()` (actions.rs:324) when the win handler's
  command queue is flushed, so it panicked. Per the task ("If it complicates,
  skip and note it") and the hard constraint "do NOT disturb the landed logic",
  the two optional swaps were removed rather than force a harness change on a rig
  I am told is off-limits. Design-wise the loss is small: ch2/ch2b are the
  two-part firefight and reading them as one continuous danger sky is coherent.
  (The ch1/ch3/ch4 accents are NON-optional and were kept - their rigs got the
  minimal harness fix below, since dropping THOSE swaps was not an option.)
- ch4 BURN path swap: SKIPPED (left on the calm cubemap.png start). Rationale: the
  burn ending is the "SAFE BUT BROKE / you slipped the belt, nobody left to
  chase you" clean escape; keeping the calm sky reinforces "no gunship, no fight"
  and contrasts the sell path's danger-sky swap. Adding a swap here would muddy
  that contrast. The sell path carries the one motivated ch4 accent.

## Verification

- `content lint --target the-ledger`: see task report (0 errors; only the single
  intended ACK - the ch4 auditor close-spawn - as before; the SetSkybox additions
  introduce no new warning).
- Landed-logic regression: `cargo test -p nova_assets --test ledger_ch2_encounter
  --test ledger_ch3_channel --test ledger_ch4_ending` all green (additive edits
  did not move any spawn/objective/gate).
- Harness fix (rig faithfulness, NOT a logic change): the ch3 and ch4 rigs now
  drive handlers that carry a real SetSkybox, whose command reads the AssetServer
  exactly as production does. Those two rigs were `MinimalPlugins`-only, so they
  gained `AssetPlugin::default()` + `init_asset::<Image>()` (the established
  pattern from `nova_scenario` skybox tests) so the shipped handler runs to
  completion instead of panicking on a missing resource. No scenario camera is
  present, so the swap no-ops after starting the load - all these behavior rigs
  need. The ch2_encounter rig needed NO change (its ch2/ch2b swaps were skipped).
- New data-level "wired" proof: `crates/nova_assets/tests/ledger_skybox.rs` pins
  the starting palette table AND that the ch1/ch3/ch4 accents are present in the
  right handler (matched by the beat's StoryMessage text), that every swap targets
  a base cubemap (no new-art path), and that no chapter swaps at OnStart. A future
  edit that drops a swap or reshuffles the palette fails a test.

## Visual confirmation is the owner's Finish step

This pass is the DATA-level "advertised is wired" proof only. Whether each swap
actually RENDERS at its beat (the skybox visibly changing in-game) is the owner's
Finish/replay checkpoint per the task DoD (manual). The lint + tests here prove
the actions parse, target valid base cubemaps, and live in the intended handlers;
they do not drive the real renderer.
