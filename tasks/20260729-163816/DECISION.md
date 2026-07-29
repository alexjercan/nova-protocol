# Decision: what artifact carries the objective on the flight HUD

- TASK: 20260729-163816
- DATE: 2026-07-29
- STATUS: ACCEPTED

## The fork

Demo 2 shows ONE artifact - a fixed top-centre amber bordered chip carrying
the objective's own text (`hud_rework_poc.html:246`: diamond + `SALVAGE WRECK`
+ dim `// 2.1 km`), popping on posting and then breathing. The game had three
things and none of them was that chip:

- the top-RIGHT hint - star + active COUNT + TAB keycap, flat (it renders flat
  because it is parented into the bcs status-bar row), also the reveal card's
  tuck anchor;
- the diegetic reveal card - the text, but transient (~3.2s, then it tucks);
- `objective_markers` chips - diamond + label + live range + breath, but
  world-anchored to an `ObjectiveMarkerTarget`, and absent for objectives that
  declare none.

Task 20260728-175747 added the pop and the breath to the HINT, which is why the
owner's playtest read as "objectives still look the same": the motion changed,
the artifact did not.

## Why the candidates were mutually exclusive

- The hint CANNOT become the bordered pill while it is a status-bar item -
  that is the exact conflict 20260724-161545 resolved by parenting it into the
  bar (a floating top-right node collided with the version string). "Keep the
  hint where it is" and "make it look like demo 2's chip" could not both hold.
- Demo 2's chip carries no count and no TAB affordance, so adopting it verbatim
  partly reverses 20260724-134312's owner choice (count + TAB on the flight
  HUD, per-objective detail in the reveal and NOVA OS).
- Top-centre is already occupied by the scenario readout strip (`readout.rs`,
  `top: 16px`), so a permanent chip there competes with a time-trial's timer.

## The call (owner, 2026-07-29)

A top-centre STACK of demo-2 objective chips that ABSORBS the hint:

1. The chip is the demo's: amber, bordered, diamond + objective LABEL + dim
   range suffix. Not a count.
2. It STACKS - several objectives can be active, so the artifact is a column of
   chips, not one chip.
3. The hint is PROMOTED OUT of the status bar and folded into the stack; the
   status bar goes back to fps + version. The TAB affordance rides the stack.
4. Visibility is a READ NOTIFICATION, not a permanent readout: a chip shows on
   posting (pop, then breathe) and leaves when it is read - EITHER a dwell
   elapses OR the player opens NOVA OS. A change/completion re-posts it unread.

Point 4 is the owner's resolution of a conflict raised at the gate: with the
hint folded into a FADING stack, the standing "there is work, press TAB" cue
disappears entirely (the status bar no longer carries one and the stack is gone
in idle cruise). Three alternatives were offered - a dim always-on TAB stub, a
dimmed persistent stack, or nothing - and the owner chose NOTHING, explicitly:
"after some time or after open the TAB thing also goes away, it's like a read
notification". Idle cruise having no objective cue at all is therefore
intended, not an oversight; the standing answer to "what am I doing" is the
world-anchored marker chip and the NOVA OS `objectives` command.

## Supersedes

- 20260724-134312 (flight objective HUD: minimalist top-right status-bar
  notification) - its PRESENTATION decision only. The reasons it removed the
  always-on compact objectives panel still hold; this replaces what it put in
  that panel's place.
- 20260724-161545 (objective hint becomes a plain status-bar item) - fully:
  the hint leaves the bar. The version-overlap this fixed must not regress -
  the stack is top-CENTRE, so it cannot collide with the top-right version.

Both stay CLOSED as history; this record is the forward-looking one.
