# Ledger chapter 2 encounter rework - design record

Task 20260717-112630, spike tasks/20260717-111808/SPIKE.md
(F1/F2/F3/F4/F6/F7). Companion engine change: the AI line-of-fire gate
(task 20260717-112622) is what makes the cover here mechanically real.

## What shipped

Chapter two split into two scenario files, both hidden, both in
`webmods/the-ledger/`:

- `ledger_ch2.content.ron` (id `ledger_ch2_claim_jumpers`, unchanged id so
  ch1's chain-in is untouched): wave one only. Two LIGHT-turret magpies
  from a single eastern lane ~600/690u out, patrols converging on the
  pickup, leash 550 centered on the patrol midpoint. Victory (kills > 1)
  is the chapter's checkpoint beat: a Victory overlay + lingering
  NextScenario into part two. Defeats retry part one.
- `ledger_ch2b.content.ron` (id `ledger_ch2b_the_heavies`, new): the
  reinforced pair from the OPPOSITE (south-west) lane ~920/1030u out,
  exactly one better turret between them (Crowbar guns, Kettle Black
  tanks). Victory chains to ch3; defeats retry part two only.
- Both parts share the arena: player and Dray Mule stations, five
  invulnerable boulders (nominal r3.5-5) forming corridor cover on BOTH
  lanes, three destructible chaff rocks. The Mule holds 85u ABOVE the
  fight plane.
- Bundle lists the new file, version 1.0.0 -> 1.1.0; README describes the
  two-act chapter; CHANGELOG entry under Scenarios & Objectives.

## The numbers (why these positions)

- Wave one 600u / wave two 950u vs effective weapon ranges (light 270u,
  better 450u): the approach IS the breathing room the old file lacked
  (old spawns: 175u at OnStart, 130u mid-fight).
- Single bearing per wave (spread ~8 degrees wave one, ~5 wave two,
  pinned <= 35): one rock can block ALL incoming fire; the old +/-X
  bracket made dodging one stream align the other.
- Asteroid bodies run 3.5x-6x nominal radius
  (ASTEROID_GEOMETRIC_FACTOR_MIN/MAX): nominal 4-5 boulders are 15-30u
  bodies, and the test pins worst-case 6x bodies against overlap with
  each other and both stations.
- Leash 550/650 centered on the patrol midpoint: full aggression in the
  arena, but a player who runs far resets the fight (the shakedown
  scavenger pattern) - an authored pressure-release valve.

## The Mule and the stray-fire model (a real catch)

First layout kept the Mule at the old station (y = -5, west of the
pickup). The new geometry pin failed it: 52u off the heavies' fire lane.
Re-deriving the model showed the pin itself was too weak - a round that
misses the player keeps flying PAST them, so the danger corridor is the
hostile->player line extended by a weapon range (500u), not the segment.
Fix: the Mule stations 85u above the fight plane (attack lanes and their
overshoot cones all run near y in [-40, 30]), and the test now models the
overshoot. That failing run is this task's fail-first evidence: the pin
caught the exact flaw (spike F6) in a layout its own author thought was
fine.

## Verification

- `cargo test -p nova_assets --test ledger_ch2_encounter`: 12 passed
  (geometry pins + behavior walks of both act machines + structural
  OnStart pins + bundle pin).
- `cargo run -p nova_assets --bin content_lint`: clean (one pre-existing
  ch4 warning, untouched by this change).
- `cargo test -p nova_assets --test webmods_validation`: green.
- Full suite intentionally left to CI per standing instruction.

## Decisions and alternatives

- **Checkpoint = act-split via NextScenario**, not an engine checkpoint:
  shipped mechanics only (hidden scenarios + lingering retry chains are
  proven patterns). The Victory overlay mid-chapter doubles as the
  breather and reads as "checkpoint reached". Fresh ship per act is
  accepted easing (spike open question, resolved as designed).
- **infinite_ammo stays true** in both parts: the base campaign moved to
  finite auto-reloading ammo, but sweeping the ledger's four chapters to
  finite ammo is a mod-wide consistency question, not a ch2 balance one -
  routed to the balance-audit task (20260717-112656) to decide with data.
- **speed_cap 25 stays**: the chapter's flight feel is not this task's
  lever (controls/feel are out of the spike's scope).
- Rock texture rides `dep://base/textures/asteroid.png` like every other
  ledger asset; no new art.
