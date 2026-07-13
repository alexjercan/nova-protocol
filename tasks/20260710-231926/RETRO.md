# Retro: Diegetic flight status v1

- TASK: 20260710-231926
- BRANCH: feature/diegetic-flight-status (squashed to master as b7da9c3)
- REVIEW ROUNDS: 1 (APPROVE; 2 MINOR, fixed in-round)

## What went well

- Questionnaire-first spike: the four open design forks (speed home, mode
  presentation, radius geometry, GRAV cue fate) were put to the user as
  explicit options with tradeoffs BEFORE any planning. /plan and /work then
  ran with zero re-litigation - the whole implement-review cycle closed in
  one round. Worth repeating for any UX-flavored task where the forks are
  matters of taste, not fact.
- Applying prior retro lessons on purpose paid twice: the mode chip's
  spawn state was derived from the same predicate as its runtime toggle
  (gravity-indicator lesson - and the test asserts the spawn state), and
  the concept-grep sweep ("status line" in prose, not just symbols) caught
  a stale keybind_hints comment the identifier grep missed.
- Reuse over invention: both new visuals cost no new tech - the chips ride
  the screen_indicator substrate, the spoke reuses HoloAssets'
  segment mesh and segment_transform (promoted to pub(crate) instead of
  copied).

## What went wrong

- R1.1 (speed chip anchor asymmetry): the Err arm cleared the anchor but
  the Ok arm never restored it, so a transient query miss would blank the
  chip forever. Root cause: the spawn state ("anchored from birth, always
  on") and the failure path were designed at different moments, and nobody
  asked "how does it come back". A hide path in a drive system implies a
  matching show path.
- R1.2 (untested well-death exit): the spoke test mirrored the ring test's
  shape (engage, track, disengage) instead of enumerating the new
  system's OWN exit conditions - the `q_well.get` failure branch existed
  in code but not in tests, and wells are destructible in play.

## What to improve next time

- When a sync/drive system has N conditions that hide or despawn its
  element, list them and cover each exit in the lifecycle test; do not
  inherit a sibling test's coverage shape.
- Visual constants written headless (the 120px chip offsets, spoke
  thickness) are unverified claims until a playtest; log them as explicit
  follow-ups at close time so they are not mistaken for done.

## Action items

- [x] Both review findings fixed in-round.
- [x] By-eye pass on the chip offsets and spoke noted in task
  20260710-234115's Notes (same instrument family, playtests together).
