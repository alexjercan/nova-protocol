# Spaceship handling / Newtonian flight-feel overhaul

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.4.0,handling,juice

Spike: docs/spikes/20260708-203517-roadmap-reprioritization-and-juice.md

Backlog gap: the game has no dedicated spaceship-handling/flight-feel work, yet
handling is a top-ranked juice dimension and is core to the hard-sci-fi capital
combat fantasy - a heavy ship must feel heavy. Today piloting is whatever the
player controller and avian physics give with no dedicated feel layer.

Goal: make flying a capital ship feel weighty, precise, and readable.

Direction (for /plan to break into steps):
- A flight-assist toggle: assisted mode (auto-brake to zero relative velocity,
  snappier response) vs full Newtonian (momentum persists, you burn to stop).
  Lean assisted-default for approachability; confirm at plan time.
- Inertial damping / RCS translation so the ship holds and corrects heading with
  believable lag instead of instant snapping.
- Throttle / main-drive control distinct from RCS trim, so big burns read
  differently from fine maneuvering.
- RCS + main-thruster feedback: visual (exhaust/particles) and audio hooks
  (shared with audio task 20260708-162011) that fire off actual thrust input, so
  the ship's motion is legible from its plumes and sound.
- Camera that conveys weight (chase lag, subtle shake under thrust - the shake
  side overlaps hit-feedback task 20260708-162013).

Open design calls captured in the spike: assisted-vs-Newtonian default; how much
realism (true 6DOF) vs playability. Settle when planning.
