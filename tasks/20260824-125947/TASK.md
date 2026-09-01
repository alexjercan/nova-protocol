# Railgun: a spinal kinetic weapon family

- STATUS: CLOSED
- PRIORITY: 72
- TAGS: v0.13.0, content, ship, weapon

Promoted 2026-08-31 from ideation into v0.13.0: the release's new section.

A new weapon family: a spinal kinetic railgun - charge time, a
hitscan-fast slug, brutal recoil that pushes back through the attitude
model, and a distinct vacuum firing effect. A capital-ship reason to
exist.

## Shape

- A spinal section: it fires along the hull axis, so the SHIP aims it.
  That is a different combat verb from the turrets and the bays, and the
  reason the family earns its place.
- Charge before fire, authored charge time, a readable charge cue.
- Recoil is real: firing applies impulse the attitude model has to absorb.
- Model at the thruster's standard from day one (`20260831-083625` sets
  the bar and decides how a section declares an animation - the railgun
  wants a charge/fire track).
- Author it in the content builders, lint it, and give the AI a firing
  rule or explicitly deny it to raiders for now - record the decision.

## Owner notes (2026-08-31)

- Damage: "really really powerful pierce damage that basically goes
  through the entire ship" - one slug should cross the target's whole
  section stack, not stop at the first hull.
- Aiming is the ship: "you have to align the ship to be able to hit" - a
  custom shoot mechanic, not a turret cone. The gun fires where the hull
  points, and the skill is getting the hull pointed.
- Cadence: "takes some time to load, has only one shot every X seconds
  (so it's really slow reload time)" - a charge-up before the shot AND a
  long reload after it. One shell in the air per gun, ever.
- Model: 1x1x3 is fair. The WFC segment chain (`0c64e2cf`) already
  places multi-cell parts, so the shape costs the generator nothing.

## Design round (2026-08-31)

Four mockups fielded on the `railgun-mockups` sprout: twin (1x1x3 open
rails), quad (1x1x2 vented muzzle brake), coil (1x1x3 ring stack),
blade (1x1x2 vertical blade rails). Owner verdict: blend - the twin's
open-rail read with the quad's vented muzzle brake, SINGLE bore, 1x1x3.
The blend (`railgun_lance`) is SETTLED as the main design idea. A
thickness round fielded brake diameters 0.50 / 0.60 / 0.75 of the cell
(0.90 skipped - its clamp yokes would leave the cell box); owner picked
0.60.

Landed as squash `020e3306`: the lance recipe
(`scripts/section-part-recipes/railgun_lance.json`), the staged mockup
(since promoted to `assets/base/gltf/railgun_lance.glb`), and a railgun row
in the default `screenshot_section_gallery` grid - no env switch. The
dropped mockups and thickness variants are off disk; this section is
their record. Design side quest CLOSED - next work here is gameplay.

## Mechanics round (2026-09-01)

Four decisions taken with the owner, in the order they were asked:

1. FIRE VERB - tap to commit, auto-fires at full charge. The trigger
   starts the charge and nothing stops it: releasing, or being told to
   safe the ship, are two different answers (see 3). The commit IS the
   decision, so the alignment that matters is the one at the END of the
   charge, not at the tap.
2. PIERCE DEPTH - true through-and-through: POWER only, no layer cap.
   The authored round carries `layers: u32::MAX` and its `slug_power` is
   the whole bound on depth.
3. AI - allowed, on a deliberately crude gate (engage-like state, a ship
   target, bore within ~8 degrees, target inside 0.6 of reach, line of
   fire clear, 14 s cooldown). `20260901-104359` is the follow-up that
   teaches it to FLY a lance run.
4. BEAT - the systems range first (`system_railgun_lance`), then the
   editor sandbox only. Shipped scenarios are untouched; improving them
   is already someone else's task.

Two consequences worth recording:

- `nova_gameplay::rounds` used ONE constant for both the across-step bite
  ring and the within-step resolution cap. A slug at lance speed crosses
  a whole hull inside one 15.6 ms step, so that cap WAS the pierce layer
  cap decision 2 rejects - silently, whatever the round authored. Split
  into `BITE_MEMORY` (8, the ring) and `MAX_BITES_PER_STEP` (32, a
  runaway backstop), pinned by
  `a_lance_speed_pierce_round_rakes_a_whole_hull_in_one_step`.
- Safing the ship mid-charge DUMPS the charge and KEEPS the shell. The
  commit is un-abortable by the trigger, not by the safety - a lowered
  ship stays cold, which is the rule every other weapon already follows.

Recoil is `apply_linear_impulse_at_point` at the MUZZLE, not at the
centre of mass, so a lance bolted off the ship's axis yaws it as well as
pushing it. That is what makes "put it on the spine" a real decision.

Audio ships now rather than deferring to `20260824-125955`: the lance
answers its own `RailgunFired` report and plays the authored sound
(currently the launch thump, per-target so playtest can diverge it).

Where it landed, in the order the beat decision set:

- `examples/systems/system_railgun_lance.rs` - the range. Six 500 hp plates
  on 6 u centres, one commit, and five markers: the commit outliving the
  trigger, the bolt tracking the charge, one slug raking every layer, the
  recoil moving the ship, and the one-shell magazine. Verified live under
  Xvfb: every plate takes the slug's full damage off a single shot, and the
  range still passes at the raised damage below.
- The editor sandbox's third picket - already named `picket_lance` before any
  of this - mounts the gun. Its PDC moves from the nose face to the nose roof
  so the bore is not staring at its own turret. One picket, not three: a
  telegraphed charge is fair, three at once is a range you stop exploring.
- The ammo readout grew a `Railgun` bar in the slug's pierce blue. One pip is
  one pip, so the HUE is what separates a spent bay from a spent lance.

Two notes for whoever tunes this next:

- `probe run system_railgun_lance` scores 7/8. The eighth,
  `fps_within_baseline`, reads "armed and silent" for EVERY systems range -
  `system_blast_penetration` at HEAD fails it identically - because no range
  emits a frametime capture and the repo has no baseline for one. Not a
  railgun defect.
- The range holds `MouseButton::Right` in `PreUpdate` rather than writing
  `WeaponsHot`. The flag is derived from the held combat stance every frame,
  so a range that pokes it charges for two ticks and dumps. Any future
  weapon range wants the same shape.

## Second pass: aiming it, and seeing it

Three follow-up decisions, all the owner's:

- **The gun aims by hull, so the hull needs a sight.** A pierce-blue thread
  runs down the bore and RINGS EVERY SECTION THE SHOT WOULD DESTROY - not
  the first thing hit, the whole depth read. It shares `pierce_remainder` with
  the round it predicts, so the sight and the shot cannot drift apart.
  `crates/nova_hud/src/bore_sight.rs`.
- **The sight is gated on `WeaponsHot`, and that was the whole answer to the
  aiming problem.** The stance that raises weapons also takes mouse steering
  away, so a sight that only appeared in Turret mode would be a sight you
  cannot use. But `WeaponsHot` is `raised OR combat-locked`
  (`input/targeting/safety.rs`), so a LOCK keeps the gun hot with the stance
  released: lock the target, line the ship up in Normal flight with the sight
  live, then commit. No new mechanic, no exception carved for this weapon,
  and no clutter on a ship that is not fighting.
- **Damage 300, up from 120.** The toughest thing in the catalog is a
  reinforced hull block at 200, so an aligned shot does not damage a column,
  it removes one. Past roughly 260 the extra buys nothing against content
  that exists; the margin is for what a mod authors. Depth stays priced in
  `slug_power`, never here.

The charge got its VFX to match. A point light rides the bore from breech to
brake on `muzzle_offset * progress` - config-derived, so a modded lance ends
at its own brake - and brightens on a CUBED curve, so the bore is still nearly
dark at half charge and the light arrives in the last third of a second. It
lands exactly where `on_railgun_fired_flash` is about to fire from: the charge
does not cut to the flash, it becomes it. On the shot, sparks off the brake for
everyone, and camera trauma for the player whose own hull just fired.

Not done, deliberately: **no align key.** An "align while in combat mode"
modifier was considered and dropped - the lock already gives back the steering
the stance takes away, and a second way to do it would be a mechanic to teach
for no new capability. `rcs_modifier` is the shape to copy if playtest
disagrees.

## Third pass: what the first playtest said

Flown in the sandbox, and three things came back.

- **The sight went away during the reload, and that read as broken.** It was
  written to draw only a LOADED lance - "no shell, no shot to aim". Wrong: the
  reload is twelve seconds and it is exactly when a pilot is lining the next
  shot up, so taking the only aiming instrument away for it was taking it away
  when it was needed most. It now stays up DIMMED, with the rings still on what
  the next shot would take off. The dim is the "not yet"; the ammo gauge is the
  countdown.
- **The arena wants one.** `wfc_arena` now stamps a spinal lance on every
  generated bow - the nose's answer to `stamp_large_drives` - standing on the
  bow keel cell `seed_keel` guarantees is solid, with everything the collapse
  hung past the bow face carved off first. The arena's own lint reports every
  contact mating with the gun on. Whether the AI actually SPENDS it is
  `20260901-104359`'s problem, not this one's: the envelope is deliberately
  crude and only commits when an orbit happens to sweep the bore across a
  target. A `:player` slot gets the gun on `R` - its own key and not the
  turrets' held mouse button, because one shell on a long reload cannot share
  a trigger with guns a pilot leans on.
- **Mouse sensitivity makes it hard to aim**, and the owner's read was that
  this is fair for a one-shot weapon. Left alone.

Open: the frame rate is down since this pass. Unresolved, and the cause is not
yet established - see the note in the session, not guessed at here.

Verified: the range passes all five invariants under autopilot at damage 300;
the lance's cycle, the charge glow and the fire kick run clean in a live Xvfb
session; the sight has four tests over the real system, including the two that
matter - it rings only what dies, and it stops where the power runs out. NOT
verified: the sight's on-screen appearance. That needs the sandbox flown with
a player-built ship carrying a lance, which is the beat this task chose.

## Done when

- The railgun is a section a player can place in the editor, charge and
  fire in flight, with model and firing effect (audio may defer to
  `20260824-125955` if that lands later - record it).
- Recoil visibly moves the firing ship.
- At least one scenario or campaign beat features it.

## Closed

All three met.

- **Placeable, chargeable, firable, with model and effect.** The lance is a
  `SectionKind::Railgun` arm in the catalog, `hide_in_editor: false`, wearing
  `railgun_lance.glb` with the `charge_bolt` track walking its bore. It did NOT
  defer audio to `20260824-125955`: it authors `railgun_fire_sound` and reports
  on the shot. Charge glow, muzzle flash, brake sparks and a camera kick came
  in the second pass.
- **Recoil visibly moves the firing ship.** Applied at the muzzle, not the
  centre of mass, so an off-axis bore yaws as well as pushes. Held as invariant
  4 of `system_railgun_lance` and confirmed in the seat.
- **At least one beat features it.** The sandbox's `picket_lance`, and - after
  the playtest asked for it - a bow stamp on every `wfc_arena` hull, bound to
  `R` for a player slot.

Flown and accepted by the owner. What the playtest raised and where it went:
the sight vanishing during the reload was a real defect and is fixed; the
mouse sensitivity was judged fair for a one-shot weapon and left alone; the
frame-rate drop did not reproduce (it was ~40 FPS in one session and ~140 in
the next, with the sight drawing MORE in the second) and is not treated as a
lance defect.

Deliberately not done here:

- **No align key.** A modifier that rotates the ship while the combat stance is
  held was considered and dropped. The combat lock already keeps weapons hot
  with the stance released, so a pilot can lock, align in normal flight with
  the sight live, and then commit - a second way to do that is a mechanic to
  teach for no new capability. `rcs_modifier` is the shape to copy if playtest
  ever disagrees.
- **The AI does not FLY the shot.** It commits when an orbit happens to sweep
  the bore across a target it is already fighting. That is recorded in
  `input/ai/railgun.rs` itself, and the lance run - break the orbit, roll onto
  the line, commit, peel off - is `20260901-104359`.
- **The sight's on-screen look is owner-verified, not test-verified.** Its
  geometry and depth read have five tests; how it reads at speed does not.
