# Review: Make the CI clippy job warning-clean

- TASK: 20260719-001600
- BRANCH: chore/ci-clippy-clean

## Round 1

- VERDICT: APPROVE
- Reviewer: out-of-context pass (fresh-context agent over
  `git diff 66c56e2b...chore/ci-clippy-clean`, spec = TASK.md + NOTES.md),
  per the `out-of-context-review-pass` lesson; the driving session
  independently re-verified the content parity pin (2/2 tests green on the
  boxed enum, zero `assets/` changes in the diff).

Verified-claims record (all re-derived by the reviewer, not trusted):

1. `while let` turret joint walk: head/inner-break/`chain = parent` advance
   equivalent to the old `loop` + let-else form; termination unchanged.
2. `HudVisibility::shows` matches! rewrite: all 6 truth-table cells
   identical.
3. shakedown pair loop: `enumerate()` before `.skip(i + 1)` preserves the
   original (i, j) pairs and assert-message indices.
4. const-block assertions: the three predicates unchanged vs
   `git show 66c56e2b:crates/nova_assets/src/sections.rs`; all operands
   const-legal; guard strictly stronger (compile-time); only loss is value
   interpolation in messages, disclosed in NOTES.md.
5. `Content::Section(Box<SectionConfig>)`: all 27 workspace sites correct -
   constructors wrap in `Box::new`, the four value-out sites clone the
   inner `SectionConfig` via `as_ref().clone()` (same value the old
   `cfg.clone()` produced), read-only sites auto-deref.
6. Serde wire shape: `Box<T>` delegates to `T`; pinned by
   `content_ron_parity` (byte-compares committed RON vs fresh
   serialization; green, and the diff touches no `assets/` files) and by
   `mod_refs.rs:534` deserializing an old-shape `Section((...))` literal.
7. bevy_reflect 0.19 has no `Reflect` impl for `Box<T>` (registry source
   checked); the `SectionSource` allow justification is accurate.
8. Turret test struct literals equivalent to mutate-after-default blocks.
9. Doc rewraps: word-level diff shows pure rewrapping; the three
   meaning-neutral textual edits are disclosed; no line starts with a
   markdown marker.
10. audio.rs `pause_loops`/`resume_loops` byte-identical, only relocated.
11. Remaining mechanical fixes (`is_none_or` x5, `is_multiple_of`,
    `then_some` on side-effect-free arms, lifetime elisions, vec!->array
    only where no `&Vec` is needed) all behavior-preserving.
12. Honesty: TASK.md ticks and NOTES.md claims match the diff; CI gate
    untouched as stated; no stray files.

Findings:

- [x] R1.1 (NIT) tasks/20260719-001600/NOTES.md:43-45 - "two merge
  clone-outs" undercounts; there are four `as_ref().clone()` sites
  (merge_content_item x2, lint walker x2). Suggested: correct the count.
  - Response: fixed in the round-1 follow-up commit (count corrected to
    x2 + x2).
- [x] R1.2 (NIT) crates/nova_scenario/src/objects/spaceship.rs:149 - the
  "~530-byte" figure in the allow comment can silently go stale; drop the
  precise number.
  - Response: fixed in the round-1 follow-up commit (now "hundreds of
    bytes (528 at the time of clippy's report)" - keeps the evidence,
    dates it).

No BLOCKER/MAJOR/MINOR findings. APPROVE stands with the two NITs
addressed; comment/doc-only edits, no compile impact, suites not re-run
for them.
