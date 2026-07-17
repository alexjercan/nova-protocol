# Tune GraphicsBudget::for_quality fractions from the frame-time baseline (replace provisional values)

- STATUS: OPEN
- PRIORITY: 38
- TAGS: v0.7.0,performance,settings


## Goal

`GraphicsBudget::for_quality` (`crates/nova_gameplay/src/settings.rs:118`)
carries provisional Low/Medium/High fractions with a comment saying they are
"provisional pending this baseline". The baseline now exists
(tasks/20260716-123551) and gives real numbers to set them from, instead of
hand-tuning blind. Replace the provisional values with data-backed ones and
remove the "provisional" comment.

## Steps

- Read the baseline's "Graphics-preset fractions - HOLD" section and the combat
  numbers: `particles` is validated as a combat lever (~11% of the combat frame,
  idle at rest); `scatter_density` currently never fires for shipped scenes (see
  the separate scatter_density task).
- Set the per-tier `particles` fraction from the measured combat delta; do not
  soften it needlessly on Low (it earns its keep in combat).
- Fold in the render-scale lever fractions once that task lands (coordinate).
- Re-run the native + web sweeps at each tier to confirm the tiers are actually
  distinct and monotonic, and record the numbers.
- Delete the "provisional pending baseline" comment once values are grounded.

## Notes

- Baseline report: tasks/20260716-123551/frametime-baseline-report.md
- Depends on / coordinates with the web render-scale lever and the
  scatter_density follow-ups from the same baseline.
