# Docking: dock to spinning neutralized ships

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.19.0, docking, flight

## User facts
- Owner 2026-11-22: "we need a way to be able to dock 'spinning' things,
  because now if a ship with a docking station has spin on it it's hard to
  align I would personally make 'neutralized' things slow down until they
  are at zero velocity eventually but not sure, can you do some thinking
  here and come up with a plan?"
- The owner's idea (neutralized hulls settle to rest) is a proposal, not a
  decision.

## Agent findings
- Research is running: what a neutralized hull does with its spin today,
  how DOCK, MATCH and the capture check treat a rotating target, and a
  reproduction at spin rates measured from fights. Results and the design
  page go in this folder.
- Options under study: neutralized hulls settle (spin, maybe drift); DOCK
  flies the spinning port's frame; a stabilise action; relaxed capture for
  spin; a combination.

## Map (2026-11-22)
- Page: `DESIGN-spin-docking.html`. Logs: `evidence/`.
- Only hulls with no live flight computer keep spin: no `AngularDamping`
  on ships, and attitude torque comes only from a live controller
  (`controller_section.rs:481-503`). The neutralized brake is linear only
  (`neutralized_brake.rs:122-128`). A live computer stops the spin; with
  none, 10, 45 and 120 deg/s held for 30 s (`evidence/unit-neutral.log`).
- The capture gate compares root centre-of-mass speeds, not port faces
  (`docking_section/port.rs:227-238, :286-291`). A perfect co-rotation at
  5 deg/s on a 63 m lever reads 5.24 m/s, TOO FAST. Above about 5 deg/s no
  motion state docks (`evidence/unit-gates.log`).
- MATCH matches linear velocity only (`autopilot.rs:2974-3020`). AI DOCK
  takes only live stations (`order.rs:601-610`) and fails above 4 deg/s on
  the port axis and above 6 deg/s across it (`evidence/unit-sweep.log`).
- Stations hold attitude by PD, so they do not spin. One live
  `system_ai_combat` run: live ships turned at up to 9.1 deg/s. Nobody was
  neutralized, so the spin of a fresh wreck is not measured.

## Decisions (owner 2026-11-22)
- Owner: "I want ships to settle after 30 seconds, I think it's the
  simplest way we can do it, I don't want to force a new mechanic of
  'matching the spin'... I am fine with this approach of having the
  'neutralized' ships just stop after some time".
- E = A + D as recommended: a ship root with no live flight computer, not
  docked, settles its spin, under the 5 deg/s gate in about 30 s (8 s
  half-life from 60 deg/s). Spin only; drift unchanged. Rocks, wreck
  fragments and docked pairs are not settled.
- The capture gate measures the two port faces with the same 5 m/s and
  5 deg/s numbers.
- No spin-matching MATCH or DOCK assist (B): rejected by the owner.

## Delivery
1. Map the slice, reproduce the failed dock, and write the design page with
   options and a recommendation.
2. Owner picks the design, then build it with a playable example.

## Verification
- An asserted example: dock to a neutralized ship with a docking port that
  spins at a rate seen in fights.

## Done when
- The player can dock to a spinning neutralized ship with a docking port,
  and the owner has played it.
