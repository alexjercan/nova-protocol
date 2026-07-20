# Retro: fix the 108 broken intra-doc links

- TASK: 20260720-134629
- BRANCH: docs/rustdoc-links
- REVIEW ROUNDS: 1 (APPROVE, one cosmetic NIT - left as-is deliberately)

## What went well

- **Measure-first, then re-measure in the right tree.** The umbrella's crate-
  level headers had shifted line numbers, so the fix started by re-running the
  strict `cargo doc` in THIS worktree to get authoritative file:line:name -
  fixing against the stale count would have edited the wrong lines.
- **The strict re-run IS the verification.** `RUSTDOCFLAGS="-D warnings" cargo
  doc ... -> 0` is a machine check that the sweep is both complete (no warning
  left) and correct (no new broken link introduced). A scripted 108-site edit is
  only safe because that check exists and was run.
- **Kept the edit precise, not blanket.** The un-link script keyed each change on
  the exact (file, line, name) rustdoc reported and verified 0 "needle not found"
  misses, so it never touched an unrelated `[`X`]` on the same line. The
  "stray brackets on other lines" I noticed turned out to be private-context
  links rustdoc neither renders nor warns on - correctly left alone (the strict
  run confirmed they are not warnings).
- **Honest fixes over convenient ones.** Un-linked (name kept in prose) rather
  than widening internal systems to `pub` or inventing a target for an
  unresolved ref - the two things the DoD forbids. Confirmed the diff was
  doc-comments-only (0 non-`///`/`//!` lines).

## What went wrong

- Nothing material. Briefly worried the script had missed occurrences (audio.rs
  had `[`X`]` on un-edited lines), but that was correct behavior - those are
  private-item docs, which do not warn. The strict re-run resolved the doubt.

## What to improve next time

- When a scripted doc sweep "misses" occurrences, check whether they are even IN
  the warning set before assuming a bug - rustdoc only flags PUBLIC-doc links to
  private items, so private-context links legitimately stay untouched.

## Action items

- No new ledger slug: this reinforces the existing measure-first /
  verify-scripted-edits-applied lessons (re-measure in the right tree; the tool's
  own strict check is the proof). Recorded here.
- The rustdoc strand is now warning-free-clean; 133030/133032 (per-item doc adds)
  start from that surface, and `RUSTDOCFLAGS="-D warnings"` is available to gate
  it in CI when desired.
