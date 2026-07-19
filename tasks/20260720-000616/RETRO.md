# Retro: split fps pass + scene looping (S2, bcs v0.19.4)

- TASK: 20260720-000616
- BRANCH: feature/fps-pass-loop (landed d889b539); bcs v0.19.4 (f4d504b)
- REVIEW ROUNDS: 1 (APPROVE; 2 NITs/records)

## What went well

- The user's original instinct ("repeat the scene") shipped as designed
  through spike -> adjudication -> two upstream releases -> a zero-knobs
  e2e: scenario loops three cycles into a full 900-frame capture with
  its reloads reported as their own line. The idle-tail concern raised
  in S1's own close-out was retired one task later.
- The iterative e2e earned its cost THREE times: param-validation
  timing, torn-down-state reads, and the seed-assert race were all
  REAL loop-cycle hazards - a class with a name now: loop cycles strip
  the protections first cycles get for free (Loading-state gates,
  load-before-Playing ordering). Each fix landed at its honest site
  (Option params, a reloading gate, seed-waiting), not as sleep()s.
- The second upstream cycle in one day ran friction-free on the fresh
  lesson: both worktrees up front, five manifests re-pointed in one
  edit, restore + retest against the PUBLISHED tag.
- The sweep-cells precedent (capture-only passes) generalized cleanly:
  the fps pass reuses the exact env-retain pattern rather than
  inventing a second mechanism.

## What went wrong

- One self-inflicted compile round: an edit script PRINTED the struct
  field instead of writing it (inspect-then-write in one script; the
  write call was simply missing). The dry-run habit cuts both ways -
  when a script mixes probing and mutating, each mutation needs its
  write verified, not assumed.
- A scaffolding artifact (`_EnrollmentDocAnchor`) briefly landed in an
  example before being caught on the next read - anchor-based inserts
  should never introduce their own scaffolding into the target.

## What to improve next time

- Edit scripts either PROBE or MUTATE, never both in one pass; end
  every mutating script with a verification grep of what it claims to
  have written.

## Action items

- [x] The exit-coordination strand is complete: spike -> S1 (protocol,
      v0.19.3) -> S2 (split pass + looping, v0.19.4).
- [ ] 20260719-233732 (p59) next in strand: partial-emit shrunk to the
      deadline net + skip diagnostics.
