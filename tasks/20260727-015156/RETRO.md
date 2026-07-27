# Retro: NOVA OS rename + nova_os crate extraction

- TASK: 20260727-015156
- BRANCH: refactor/nova-os-crate
- REVIEW ROUNDS: 1 (APPROVE, out-of-context; one NIT adopted)

See TASK.md for what changed and the DoD evidence; this is process only.

## What went well

- Understand-first paid off: verifying the plan's symbol map against the live
  code before building caught two stale plan facts (the file was 8382 lines,
  not ~5900; DoD-1's grep exclusion was `nova_editor/src/ui/` but the editor's
  own "drawer" doc line lives in `nova_editor/src/lib.rs`) and let me widen the
  exclusion at the gate instead of hitting a false DoD failure at the end.
- The rename was one ordered, case-preserving substring pass (`NovaDrawer` ->
  `NovaOs` before the generic `Drawer` -> `NovaOs`) driven by a script over an
  explicit in-scope file list: 596 substitutions, editor untouched, compiled
  first try. Prose (comments + test strings) got a separate pass to the display
  name "NOVA OS" so the result reads right, not just `s/drawer/nova_os/`.
- The source was already well-decoupled for extraction - `submit` takes a
  `TerminalCommandSnapshot` rather than reading the world - so the logic moved
  without a redesign. Scripted line-range extraction (not hand-transcription)
  kept the ~1500-line move faithful; the reviewer diffed `submit` against master
  and found only the visibility change.
- Designing the accessor API from the boot/sync systems' actual usage preserved
  the deliberate immutable-read-then-conditional-`&mut` change-detection pattern
  (don't mark `NovaOsTerminal`/`NovaOsAppRegistry` changed when nothing changed).
  The out-of-context reviewer verified exactly this and confirmed it matches
  master.

## What went wrong

- The plan under-scoped the public-API surface. It named the *methods* to move
  but not that ~10 gameplay systems and ~30 test sites read `NovaOsTerminal`'s
  *private fields* directly. Extracting the type across a crate boundary
  therefore required designing a getter/mutator API and splitting the test
  module - discovered mid-build by grepping field accesses, not planned. Root
  cause: the plan reasoned about the type's methods, not about who drives its
  state from outside.
- Moving documented code across the new crate boundary broke intra-doc links -
  cross-module `[NovaOsTerminal]` links needed full paths, and `pub` docs
  linking private items (`Self::cycle_stem`, detent consts) warned. `cargo doc
  -D warnings` surfaced them; fixing was quick but it was rework the move
  created.
- Splitting the single giant test module needed body-by-body reading: some
  `log`/`objectives`/`ship` tests looked pure but build snapshots from
  gameplay-resident bridge types, so they stay; only the model/shell tests move.

## What to improve next time

- Before extracting a state-holding type into a new crate, grep every field and
  method access of it across the whole crate FIRST, and design the public
  accessor API (plus the read-vs-write / change-detection seams) as part of the
  plan - so the API surface and the test split are scoped, not discovered while
  coding.
- Treat "move documented code across a module/crate boundary" as implying an
  intra-doc-link fix pass: run `cargo doc -p <crate> --no-deps` (or `-D
  warnings`) as part of the move, expecting cross-module links to need full
  paths and private targets to need unlinking.

## Action items

- [x] Bumped ledger `rustdoc-no-public-to-private-intra-doc-link` (x2) and added
      the cross-boundary-move facet.
- [x] Added ledger `extract-type-grep-its-drive-sites-first`.
- No follow-up code tasks: the `nova_ui` opportunistic helper was deliberately
  skipped (D2 allows it; nothing cleanly model-independent), recorded in TASK.md.
