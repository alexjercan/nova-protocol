# Review: Gauntlet Run 2.0 (task 20260716-124722)

## Round 1 - out-of-context adversarial review (fresh context over `master...HEAD`)

- VERDICT: APPROVE (round 1). The reviewer independently re-derived both course
invariants from the raw RON (Python script), read the production
loader/filter/action code, ran all four suites green, and proved the geometry
rig can fail (moved a rock onto the line -> RED with the exact clearance
diagnostic -> restored). Soft-lock chain confirmed airtight (out-of-order,
double-fire, wrong-ship, post-win-flip all impossible). No correctness blocker.
One MINOR and three NITs, all addressed below even though none blocked.

### MINOR-1 - `demo` dependency silently doubled the racer's hull; undocumented coupling
The bundle declared `dependencies: ["base", "demo"]`; the content only names
base prototypes/textures, but the demo mod overrides `reinforced_hull_section`
by id (health 200 -> 400), so the "reinforced hull buys crash tolerance"
premise silently rode on demo - which also forced players to enable the demo
arena, and demo is slated for removal (task 20260716-155816).
- [x] Response: fixed in the review commit. Dropped `demo` - the content needs
  only `base`. The racer's crash tolerance is now base's honest 200-health hull
  (tuned by playtest, not by an accidental override), and the mod no longer
  depends on a scenario slated for removal. Documented in the bundle comment,
  the content.ron header, and NOTES.md ("Dependency: base only"). Verified:
  `webmods_validation` (recursive dep resolution) stays green on `["base"]`, and
  the portal republishes 1.1.0 cleanly.

### NIT-1 - stale gate-spacing figure in the content header ("80-95u", actual 81-102u)
- [x] Response: fixed. Header now reads "81-102u apart ... (min pairwise area
  gap ~36u)".

### NIT-2 - "rocks crowd JUST off it" overstates tightness (tightest clears by ~17u)
- [x] Response: fixed. Reworded the content header and NOTES to "rocks flank the
  line (the tightest ~9u past the ship margin) ... a wide, sloppy turn clips a
  rock", and flagged pulling them tighter as an explicit deferred playtest feel
  pass (the rig enforces only the >=8u floor).

### NIT-3 - stale "run with a unifying sibling crate" note in the rig header (it runs standalone)
- [x] Response: fixed. Rig header now states it lives in nova_assets (which
  already unifies the serde feature) and runs standalone via
  `cargo test -p nova_assets --test gauntlet_course`.

### Categories the reviewer found clean
Invariants independently TRUE (min area gap 36.2u; every rock clears its 6x
worst-case body by >=16.9u; scatter box by 26u); rig math sound with no
sampling gap (distance-to-convex-set along a segment is convex); ordering chain
airtight; RON parses/loads; claims-vs-data all accurate (version, gate count,
radii, skybox swaps, Defeat gating).

### Post-fix verification
`cargo fmt --all -- --check` clean; `gauntlet_course` 9 passed;
`webmods_validation` 1 passed (deps resolve on `["base"]`); `nova_portal_gen`
publishes gauntlet 1.1.0. The MINOR and all NITs are content/docs edits with no
behavior change, so no re-review round was required over an already-APPROVE
verdict.
