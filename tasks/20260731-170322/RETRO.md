# Retro: KISS: nova_gameplay HUD - NOVA OS drawer surfaces

- TASK: 20260731-170322
- BRANCH: refactor/kiss-nova-os-hud
- REVIEW ROUNDS: 2

## What went well

Mechanical verification carried this task, not narrative. Splitting 14.5k
lines by line range is exactly the operation where "it compiles and the tests
pass" is weak evidence - a dropped statement inside a live function can do
both. Four independent checks against `master` closed that gap: item-name
multisets per file group, whitespace-stripped plugin `build` bodies, stripped
byte counts (which may only grow), and the test-name list diffed via `--list`.
All four were cheap, and together they are much stronger than reading the diff.

Writing the slicer as a script rather than by hand also meant the boundary bug,
once found, was fixed in one place and re-run, rather than patched at thirty
call sites.

## What went wrong

**The doc-link breakage was a known lesson I did not apply.**
`rustdoc-no-public-to-private-intra-doc-link` has sat in the ledger at x2 since
20260727-015156 and says, in as many words, that "moving documented code across
a module/crate boundary reliably breaks these, so run `cargo doc -p <crate>
--no-deps` as part of the move". This task moved 14.5k lines across nineteen
new module boundaries and I did not run `cargo doc` until the review round. It
found 30 new warnings, two of which were silently WRONG paths
(`super::HudNovaOsExempt` and `super::super::audio` now resolve one level off).

Why it seemed sound at the time: the DoD listed check, fmt, the HUID grep, the
line-count bound and the tests, and all five were green. I treated the DoD as
the complete verification surface. A DoD is the floor the plan could foresee,
not a ceiling - and this hazard was foreseeable, because it was already
written down.

**Range slicing separated doc blocks from their items.** The first ship/map
split cut several slices mid-doc-comment. Three surfaced as `expected item
after doc comment` compile errors, which made the failure feel
self-announcing - it is not. When an orphaned doc block lands on a following
item instead of at end-of-file, it compiles clean and silently mis-documents
that item. Two such cases had already happened in the earlier `nova_os/tests/`
split (`press_pad`'s doc on the pad test and vice versa, `chin_controls_app`'s
on an unrelated CRT test) and were caught only by a deliberate hand audit.

**`cargo fix` deleted re-exports that only `cfg(test)` code reads.** It
stripped the seven names `nova_os_pointer_rig` imports, breaking the test
build; re-adding and re-running deleted them again. This is not a `cargo fix`
bug: `nova_os_pointer_rig` is `#[cfg(test)]`, so in the non-test build those
re-exports genuinely are unused. I lost two cycles treating it as flaky tooling
before asking why the compiler was right.

## What to improve next time

- Run `cargo doc -p <crate> --no-deps --document-private-items` on any change
  that moves documented items across a module boundary, and diff the warning
  count against the base rather than reading it absolute - all three warnings
  left here are pre-existing, and only a baseline comparison shows that.
- When slicing a file programmatically, make the boundary rule "snap back over
  any leading `///` / `#[...]` block" from the first attempt, then audit for
  silent mis-attribution anyway, because the compiler catches only the
  end-of-file cases.
- Treat an autofix that "will not stick" as evidence the compiler is right
  about a cfg the fix cannot see, not as tooling flakiness.

## Action items

- Ledger: bumped `rustdoc-no-public-to-private-intra-doc-link` to x3, crossing
  the threshold, so it moves to Pending promotions.
- Ledger: bumped `anchor-edits-in-the-right-scope` to x4 and widened it to
  cover programmatic range slicing, not just Edit anchors.
- Ledger: bumped `dead-code-hides-under-cfg-test-reader` to x2, extended with
  the `cargo fix` direction.
- `20260731-174911` filed for the dead objective / flight-log row lists found
  during the pass. Not fixed here, per the moves-only scope.
