# Railgun: a spinal kinetic weapon family

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.13.0,content,ship,weapon

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
  Xvfb: all six plates read 380.0 (500 - 120) off a single shot.
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

## Done when

- The railgun is a section a player can place in the editor, charge and
  fire in flight, with model and firing effect (audio may defer to
  `20260824-125955` if that lands later - record it).
- Recoil visibly moves the firing ship.
- At least one scenario or campaign beat features it.
