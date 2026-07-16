# Review: Explicit content generator bin; make content_ron_parity assert-only

- TASK: 20260716-155823
- BRANCH: refactor/gen-content-bin

## Round 1

- VERDICT: APPROVE

Verified independently rather than trusting the summary: both consumers
walk the same `content_files()` map (bin writes, test asserts - no
second serialization path survives); the generator ran twice with zero
diff against the committed files; BOTH new guards were proven able to
fail - deleting menu_ambience.content.ron failed
`committed_content_matches_builders` with the regen message and did NOT
recreate the file (the old write-on-missing behavior is really gone),
and deleting the bundle entry failed
`base_bundle_ships_exactly_the_generated_files`. check --all-targets,
fmt, and the parity tests are green after restore. Docs sweep covered
the two stale surfaces (modding-ron.md, including its pre-existing
wrong `scenario_ron_parity` name; the LESSONS write-on-missing clause).

No findings. R1.2 from 20260716-155816 (the uniformity guard) is
delivered by the bundle-set test.
