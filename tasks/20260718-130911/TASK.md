# CHANGELOG still claims Low/Medium thin scatter fields, but scatter is no longer a preset lever (task 20260718-004834 removed it)

- STATUS: CLOSED
- PRIORITY: 15
- TAGS: v0.7.0, docs, settings

## Goal

Task 20260718-004834 removed `scatter_density`/`scaled_count` from
`GraphicsBudget` (scatter/object counts are gameplay content, not a quality-tier
lever), but the `CHANGELOG.md` `[Unreleased]` entries still tell players the
preset thins scatter. Two stale claims:

- The "Settings menu is real content" entry (Interface & HUD): "Low is
  spawn-less (no particle bursts) and thins dense asteroid/debris fields, Medium
  thins the fields a little while keeping particles."
- The "Low/Medium graphics-quality presets now skip expensive visuals" entry
  (Performance): "Low is spawn-less ... and thins dense scatter fields to half,
  and Medium keeps particles but still thins the fields a quarter."

## Steps

- Edit both CHANGELOG entries so they no longer claim any tier thins scatter;
  keep the particle spawn-less (Low) and render-scale (Low) facts, which are
  true. Medium's only remaining gate is "keeps particles" (it no longer differs
  from High on the budget).
- Sweep the other player-facing surfaces for the same stale claim (the dev wiki
  `keeping-docs-in-sync` map): player wiki, `/news/`, tutorial - the scatter
  removal task may have missed them too.

## Notes

- Found while landing the render-scale task (20260718-004723); pre-existing on
  master, not introduced there. Sibling of the scatter-removal task
  20260718-004834.
