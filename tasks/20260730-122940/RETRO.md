# Retro: Wide keycaps render at their art aspect, height-constrained

- TASK: 20260730-122940
- BRANCH: fix/keycap-aspect
- REVIEW ROUNDS: 1 (APPROVE, two NITs)

## What went well

- Measuring the art BEFORE planning is what made this a small task. `magick
  -alpha extract -threshold 0 -format '%@'` over the glyph set turned "the caps
  look squeezed" into six exact bounding boxes, and those same numbers became
  the tests' independent expectations. The plan gate presented measurements, not
  a theory, so there was nothing to discover mid-build.
- The previous task's `hud::chip_layout_rig` (20260730-122909) paid for itself
  one day later: a live taffy+text app with `measure()` already existed, so the
  fail-first rig cost one helper (`load_png`) instead of a rig. Extending the
  neighbour's rig beat writing a bespoke one, exactly as
  `reuse-known-good-stack` says.
- Fail-first was done honestly even though the production change was already
  written: the new `KeyCap::apply` was temporarily reduced to the old square box
  and the rig watched to fail with real numbers (`Vec2(22.0, 22.0)` vs the art's
  1.514), then restored. That is a weaker proof than writing the test against
  untouched production code, but it is a real one - and the numbers are in
  NOTES.md rather than only in a session that will evaporate.
- Running `cargo test -p nova_assets` (not just the touched module) surfaced a
  red that predated the branch. Checking it against the base commit BEFORE
  blaming the branch is what turned a scary failure into a two-minute triage and
  a filed task (20260730-161545).

## What went wrong

- The two NITs are both "conventions I did not re-read": `len() > 0` where the
  type has `is_empty` (clippy's own lint), and a new `pub fn` that never reached
  the module's `prelude` though AGENTS.md requires it. Root cause: new public
  API was added incrementally while chasing the sizing behaviour, and the
  convention pass over the new SURFACE (what is public, what is exported, what
  clippy says about it) never happened as its own step. Both are one-line fixes,
  which is exactly why nothing forced them.
- `KeyGlyphs::len()` counts LABELS while the scan dedups to IMAGES, so the
  warn's `{measured}/{len}` compared two different denominators. Harmless as a
  guard, but it is the kind of imprecision that becomes a wrong bug report
  later; the reviewer caught it as prose, not as a finding, which is the right
  severity.
- Two capture runs (stash, run, unstash, run) were needed for the before/after
  crop, because the "before" was only reachable by reverting. Building the
  after-shot first and then discovering the before was still wanted cost a full
  extra compile+run.

## What to improve next time

- When a change adds public API, make "surface pass" an explicit step before
  review: everything new that is `pub` either goes in the prelude or gets
  narrowed, and the new code is read once as API rather than as behaviour. Both
  round-1 findings die there.
- When a task's DoD includes a rendered eyeball, capture the BEFORE shot first,
  while the tree is still unmodified - it costs nothing then and a full rebuild
  later.

## Action items

- [x] tatr 20260730-161545: fix the red `an_early_derelict_kill_skips_to_the_fight`
      shakedown test inherited from master (filed, not fixed here).
- [x] Interim note on backlog 20260728-214929: it inherits `KeyCap` as the
      keycap sizing path, so only the TINT half of its "Sizing/tint" bullet is
      still open.
- [x] Ledger: bumped `reuse-known-good-stack`, added
      `measure-the-asset-before-theorising-about-it` and
      `public-surface-pass-before-review`.
