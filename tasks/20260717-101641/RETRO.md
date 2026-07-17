# Retro: Per-target impact and destroy sounds

- TASK: 20260717-101641
- BRANCH: task-20260717-101641-impact-destroy-sounds (squash-landed af0c631c)
- REVIEW ROUNDS: 2 (APPROVE at round 2)

## What went well

- The bounded ancestor walk solved the asteroid node-vs-parent shape cleanly
  and doubled as the torpedo-detonation path with zero extra code; the walk
  test pins the shape.
- The 19-literal BaseSectionConfig surface was counted BEFORE implementing, so
  the compile-error cascade was expected mechanical fallout, not a surprise.
- The reviewer + implementer split worked exactly as designed: the reviewer
  traced detonation to the projectile (confirming the snapshot host) and found
  the one real gap.

## What went wrong

- R1.1: the silent-regression sweep covered assets/, examples/ and the editor
  but NOT webmods/ - 15 webmod asteroids would have gone silent. This is the
  reel-miss lesson (sweep-content-repo-wide-not-just-assets, retro 002105)
  recurring in narrower form: the sweep enumerated the surfaces I remembered,
  not ALL content-shaped trees. Root cause: the sweep greps targeted .rs
  config literals + base content; webmod RON asteroids match neither pattern.
- A blind `AsteroidConfig {` string-split edited the struct DEFINITION too -
  caught by the immediate compile, cheap, but the fix-it-twice pattern shows
  blind splits need anchors that exclude definitions.

## What to improve next time

- Silent-regression sweeps for authored-or-silent flips must enumerate content
  trees mechanically: assets/**, webmods/**, examples/**, editor code paths -
  grep the OBJECT KIND (Asteroid((, Turret(() across ALL of them, not just the
  trees the previous cycle touched.

## Action items

- [x] LESSONS: bump sweep-content-repo-wide-not-just-assets with the webmods
  occurrence.
