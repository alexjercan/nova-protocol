# Retro: wire sccache into the nix devshell

- TASK: 20260721-000229
- BRANCH: tooling/sccache-devshell (landed c6e0974b)
- REVIEW ROUNDS: 1 (out-of-context APPROVE; 2 NITs, no change)

See TASK.md close-out for the numbers; process only here.

## What went well

- The spike's up-front root-cause (cargo fingerprints by name+version, not
  source path) made the design decision clean and gave the safety argument its
  teeth: sccache is content-hash keyed with per-worktree target dirs, so the
  fingerprint collision provably cannot occur. Wiring was a 3-line flake edit.
- The measurement methodology was self-contained and honest: cold (sccache
  zeroed) vs warm (`cargo clean`, cache kept) in ONE worktree - no nested
  sprouting - gave 405s -> 38s with 0% -> 100% hit rate.
- The out-of-context reviewer INDEPENDENTLY re-ran the warm build (39s, same
  100% hits) rather than trusting the number. For a "the build is now fast"
  claim that is the only review that counts - a reproduced number, not a
  reported one.

## What went wrong

- Nothing material. The one honest gap (neither run re-measured the COLD build,
  since evicting the shared cache is expensive) is a NIT: the load-bearing claim
  is the WARM build, reproduced twice to the second.

## What to improve next time

- For a TRANSPARENT tool (a compiler wrapper / cache), "the build was fast" is
  not proof it is working - a silently-inactive `RUSTC_WRAPPER` gives a normal
  build that looks fine. Always confirm the tool is ACTIVE via its own counter
  (`sccache --show-stats` non-zero requests), and have review re-derive it. Both
  the impl and the review did this here; keep it the standard for cache/wrapper
  work.

## Action items

- [x] LESSONS.md: added `verify-transparent-tool-is-active` (x1).
- Follow-up (noted in TASK.md, not filed): the sprout-scoped
  `CARGO_INCREMENTAL` variant (main checkout keeps incremental) lives in
  nix.dotfiles; raise there if main-checkout iteration speed becomes a
  complaint.
