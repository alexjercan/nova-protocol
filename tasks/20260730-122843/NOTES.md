# NOTES: dock hide - the re-centring verdict and the rendered evidence

## The centring question (Steps item 5)

The dock row is `justify_content: Center`, so hiding a chip re-centres the ones
that remain. The plan asked whether that reads as intended or as the row sliding
under the eye.

Verdict: it reads fine, and the evidence is rendered, not reasoned.
`screenshot_combat` under Xvfb produced three shots of the same range at
different moments, cropped identically (`-gravity south -crop 1400x110+0+0`, all
from 1920x1080 sources - one zoom, per the `compare-crops-at-one-zoom` lesson):

- `feature-hud.png` - five chips: STOP, GOTO, RADAR (hot/inverted), COMPONENT,
  RCS. No well in reach, so ORBIT is gone; nothing engaged, so CANCEL is gone.
- `feature-autopilot.png` - the same five.
- `tutorial-combat-lock.png` - FOUR chips: COMPONENT has dropped out, and the
  row re-centres (STOP shifts right by roughly half a chip).

So a real set change was observed, not just argued. The shift is about half a
chip width on a five-chip row and the chips stay legible through it; nothing
about it reads as the row swimming. Two things make it mild in practice: the
dock is short (never the full seven at once in this range), and a chip that
leaves is by definition one you were not about to press.

This is the honest limit of the evidence: three still frames prove the row
re-centres and stays readable, they do not prove how the MOTION feels in the
hand. That is DoD 5, the owner's playtest, and it stays open until then.

## Probe

`cargo run -p nova_probe -- run playable`: OK, 5/6 checks, 1401 frames, 0
invariant violations, 0 panic/ERROR lines, clean exit. `fps_within_baseline` is
SKIPPED - no baseline captured, so it is NOT MEASURED, not "held".

## What the DoD 3 grep turned into

DoD 3 shipped a proof command that could not prove anything: `dim chip|greyed|
all seven verbs` over `crates` hits `nova_editor`'s greyed coming-soon rows and
the corrected CHANGELOG line itself (which now says "rather than shown greyed
out" - a correct sentence containing the banned word). Flagged at the plan gate
and tightened to the stale CLAIMS instead of the words they were made of:

    rg -n -i 'dim ?/ ?available|dimmed when the verb|dim chip STAYS|dimmed icon dock' \
      web/src README.md CHANGELOG.md crates

Zero hits. Each alternative is a phrase that was actually in the tree before
this task: the CHANGELOG's `(dim / available / hot)`, the wiki's "dimmed when
the verb cannot do anything right now", `update_dock`'s "a dim chip STAYS on
screen", and the wiki's "the dimmed icon dock".
