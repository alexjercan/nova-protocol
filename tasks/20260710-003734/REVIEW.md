# Review: Shot-down torpedo dies without its blast

- TASK: 20260710-003734
- BRANCH: fix/torpedo-shootdown

## Round 1

- VERDICT: APPROVE

Reviewed `git diff master...fix/torpedo-shootdown` against TASK.md; full
nova_gameplay suite on the branch: 221/221 green. The diagnosis matches
the code (the root carries the fuze and no Health; children die through
the normal pipeline and nothing propagated the kill), and the fix is the
minimal correct seam: an observer at the HealthZeroMarker stage, scoped
by the torpedo marker on the PARENT, with try_despawn for the
both-sections-die-same-burst race, and deliberately no blast_damage - the
design rationale is documented at the observer. The test trio is exactly
right: the unit kill, the real-pipeline quiet-death (asserting zero
BlastDamageMarker entities through HealthPlugin damage propagation), and
the non-torpedo guard pinning that ship sections dying do NOT despawn
ships. The suppressed-debris polish gap is honestly recorded in the
Resolution rather than papered over. Existing armed-detonation regression
tests stay green, so a healthy torpedo still explodes on its target.
No findings.
