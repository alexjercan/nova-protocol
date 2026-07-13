# Spike: Diegetic flight status - where does each piece of the bottom-left line go?

- DATE: 20260710-234019
- STATUS: RECOMMENDED
- TAGS: spike, hud, ux, v0.5.0

## Question

Task 20260710-231926: the bottom-left flight status line
(hud/flight_status.rs) packs too much information and reads as debug text
to anyone who does not know the project. Replace it with diegetic,
in-world presentation and delete the line. The uncertainty this spike
reduces: for each fact the line encodes, where does it live instead, in
which of the HUD's existing visual languages, and what (if anything) needs
new tech? A good answer assigns every consumer a home, so the line can be
deleted rather than duplicated. The user answered a four-question
questionnaire (2026-07-10) that settled the direction; this doc records
the choices and the reasoning.

## Context

The status line is one formatted string (flight::flight_status_line)
that encodes, depending on state:

| Fact                          | When shown            |
|-------------------------------|-----------------------|
| Mode (MAN vs AP)              | always                |
| Verb (STOP / GOTO / ORBIT)    | AP engaged            |
| Phase (ALIGN / BURN / HOLD)   | AP engaged            |
| Speed (u/s, numeric)          | always                |
| GOTO distance (m)             | GOTO engaged          |
| Orbit radius r (current)      | ORBIT engaged, in well|
| GRAV <well name>              | MAN inside a well     |

Since the flight-instruments cycle (spike 20260710-174523, shipped), the
HUD already speaks a two-language vocabulary this spike extends rather
than invents:

- **World-space holo geometry** (unlit NAV_CYAN meshes): ORBIT ring
  (maneuver_instruments.rs), trajectory ribbon and flip gate
  (holo_instruments.rs), plus the velocity and gravity spheres
  (velocity.rs, custom shader materials around the ship).
- **Screen-projected chips** (screen_indicator.rs substrate): destination
  readout with ETA + closing speed + distance, FLIP countdown chip,
  orbit-ring `r | v_circ` chip, anchored keybind cues, and the bottom-left
  keybind hint cluster (keybind_hints.rs).

Two facts of the line are therefore already rehomed: GOTO distance/ETA
(destination chip) and planned orbit numbers (ring chip). There is no
world-space text anywhere in the repo (no Text2d, no billboard text); all
text rides the UI pass, per the weapons-hud decision (no second Camera2d,
no gizmos in shipped UI).

## Options considered

Per fact, the candidates weighed (questionnaire options):

- **Speed.**
  - *Ship-anchored chip near the velocity sphere* (chosen): a
    screen-projected numeric chip anchored to the ship entity with a pixel
    offset parking it just outside the sphere. Proven substrate, matches
    the user's "close to the sphere widget, but still readable as UI".
  - *World-space 3D text*: most diegetic but a brand-new capability with
    legibility risk; rejected for now.
  - *No number, shader shading only*: cleanest screen but loses the exact
    value (docking, matching speeds); rejected.
- **Mode + phase.**
  - *Chip + shader tint* (chosen): a compact ship-anchored chip carries
    the verb and phase text (e.g. `AP ORBIT - BURN`), and a family-wide
    shader/material treatment (velocity sphere, ribbon, ring, gate)
    signals engaged-vs-manual at a glance. Manual mode shows no chip:
    quiet HUD is the manual-flight look.
  - *Highlight the engaged verb in the keybind cluster*: no new elements,
    but keeps the info in corner furniture, which is what the task kills.
  - *Shader treatment only*: fully diegetic but the specific verb becomes
    implicit; rejected as the sole channel.
- **Orbit radius (current).**
  - *Radius spoke line* (chosen, the user's original idea): a thin
    world-space holo line from the well center to the ship along the
    current radius, with the number riding it as a chip. New holo element
    but squarely in the ribbon/ring language (thin unlit cylinder).
  - *Extend the existing ring chip with current r*: cheapest, but the
    current radius is a spatial fact between ship and well, and the chip
    sits on the planned ring, not on the ship's actual radius.
  - *Both*: overlapping elements saying the same thing; rejected.
- **GRAV cue while coasting in a well.**
  - *Gravity sphere suffices* (chosen): the yellow gravity indicator in
    the velocity-sphere family already shows pull direction and strength
    in-world; the text cue is dropped entirely.
  - *Name chip on the well / near the ship*: rejected; the well's identity
    was never the point of the cue, the curving trajectory was.
- **Do nothing** (keep the line): rejected by the task itself; the
  playtest verdict is that the line is opaque and overloaded.

## Recommendation

Replace the status line with three pieces, all in the existing vocabulary,
then delete it:

1. **Ship-anchored status chips** (screen-projected, anchored to the
   player ship with offsets clearing the velocity sphere):
   - a speed chip (numeric u/s) parked beside the velocity sphere, always
     visible;
   - a mode chip showing verb + phase (`AP GOTO - BURN`) only while the
     autopilot is engaged; manual shows nothing.
2. **Engaged-state shader tint across the instrument family**: velocity
   sphere, ribbon, ring, and gate share a material treatment that reads
   engaged-vs-manual (hue/intensity shift), reinforcing the mode chip
   diegetically.
3. **Radius spoke holo** while ORBIT is engaged: a thin world-space line
   from the well center to the ship, with the current radius as a chip
   riding it. The planned ring and its `r | v_circ` chip stay as-is.

Dropped without replacement: the `GRAV <name>` coasting cue (the gravity
sphere carries it) and the standalone GOTO distance (the destination chip
already shows distance, ETA, and closing speed).

Deletion criterion: `flight_status_line` and the bottom-left text node go
away in the same change that lands the chips and spoke - the task is
replacement, not addition. The keybind hint cluster is unaffected (it is
input affordance, not flight state) but its docking position may need a
nudge once the line under it disappears.

## Open questions

- **Chip offsets vs camera distance**: the velocity sphere has a fixed
  world radius (5 u), so its apparent size varies with camera distance
  while a fixed pixel offset does not. Decide at /plan time whether the
  speed chip offset is fixed-px (v1, simplest) or tracks the sphere's
  projected radius.
- **Shader tint mechanics**: the holo meshes are plain unlit
  StandardMaterial while the spheres use custom material extensions.
  Whether the engaged tint is a shared material extension or a simple
  per-frame color swap is a /plan decision; color swap is the honest v1.
- **Spoke endpoint**: well center to ship, or ring to ship (the radial
  error segment)? Playtest question; start with well-to-ship since it is
  what the number measures.
- **Hint cluster reflow**: where the cluster docks once the status line is
  gone - keep bottom-left or migrate toward the ship cluster later.

## Next steps

Direction-level tasks (for /plan to break into steps when picked up):

- tatr 20260710-231926 (existing, re-scoped by this spike): diegetic
  flight status v1 - ship-anchored speed + mode/phase chips, ORBIT radius
  spoke holo, delete the status line and flight_status_line.
- tatr 20260710-234115 (seeded): engaged-state shader tint across the
  instrument family (velocity sphere, ribbon, ring, gate).
