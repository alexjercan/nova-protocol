# Retro: NOVA OS map app: clicks miss their targets where the ship app's land

- TASK: 20260730-123039
- BRANCH: fix/nova-os-map-click-targets
- REVIEW ROUNDS: 3

## What went well

- **The owner's own comparison was the whole diagnosis.** "The ship app works,
  the map app does not, same plumbing" pointed straight at a defect whose size
  varies with distance from screen centre, because that is the one property that
  distinguishes the two apps' blip distributions. A bug report framed as a
  CONTRAST between two surfaces is worth more than a description of the failure,
  and it survived intact through the task text into the fix.
- **The A/B pass earned its cost three times.** It caught that the new label pill
  had silently absorbed the corner-click test's ability to fail (that test passed
  against a restored pre-fix mapping), and in review it caught that the collapse
  remap, and then the collapse GATE, were both untested. Each was a green suite
  hiding an untested path.
- **Out-of-context review found what the implementing session could not.** All
  six findings across three rounds were real, none was noise, and two of them
  (R1.1, R2.1) were holes in tests I had written and believed.

## What went wrong

- **I measured the task's framing instead of checking its premise.** The task
  named "the barrel inverse is approximate" as hypothesis 1 and I spent the first
  pass building a rig to quantify that residual. The actual defect was visible in
  one screen of WGSL: the shader's chain already maps screen->image, so no inverse
  was wanted at all, and the overscan line sits two lines below the barrel call.
  Root cause: I read the Rust helper's DOC COMMENT ("Inverse of the shader's
  forward barrel warp") as a description of what the shader does, instead of
  reading the shader - the folklore trap
  [[verify-engine-guarantees-in-source]] names, applied to our own code rather
  than a dependency's.
- **R1.1: I added a code path and tested only the branch where it is a no-op.**
  The pointer gained a raster-collapse remap, and the grid test ran only at
  `power = 1.0`, where that remap is exactly the identity - so flipping its divide
  to a multiply passed all 785 tests. It looked fine because the test's stated
  subject is "the mapping agrees with the shader", and it did, at the one power
  sampled. Root cause: the sweep's axis came from the ORIGINAL bug (position
  across the screen) and was never extended when a second input entered the
  function.
- **R2.1: my first fix for R1.1 modelled one of the shader's two output gates.**
  The fragment multiplies by `in_bounds` AND `(1 - collapsed)`; I transcribed only
  the first into the test reference, while the production helper correctly applied
  both. It passed purely because a 17x17 grid stepped over the band where the two
  disagree - 0 hits at 17x17, 186 at 201x201 at power 0.15. Root cause: I
  transcribed the shader's sample-UV COMPUTATION from the source and then wrote
  its VISIBILITY RULE from memory.
- **I ran `git checkout` on an uncommitted fix during an A/B and lost it.**
  `bug.md` warns about exactly this and I walked into it anyway, mid-review-fix,
  because that sabotage felt like a quick throwaway rather than "an A/B". Cheap
  only because I noticed immediately.

## What to improve next time

- When our own helper claims to mirror an external artifact (a shader, a wire
  format, a spec), **diff it against that artifact's actual lines and count the
  operations on each side** before theorising about its accuracy. "Two operations
  in the shader, one in the helper" IS the finding.
- When a function gains a new INPUT, extend the test's axes to cover it in the
  same edit, and add a guard proving the new axis is non-degenerate (here
  `off_picture > 0 iff collapsed`, derived from the constants). A parameter that
  never varies across a test run is a parameter that is not tested.
- Transcribe the WHOLE of an external rule in one pass - the computation AND the
  conditions under which its output is used - quoting the source lines in the doc
  comment so the next reader can diff them.
- Treat "quick sabotage" as an A/B, with the commit-first discipline, every time.

## Action items

- No follow-up code work. The lessons below carry the generalizable parts; the
  mapping, its coverage and the rig's single transcription all landed here.
