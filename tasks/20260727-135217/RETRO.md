# Retro: NOVA CRT star mark icon on the Computer/TAB status item

- TASK: 20260727-135217
- BRANCH: feature/nova-os-tab-star-icon
- REVIEW ROUNDS: 1 (APPROVE, one doc NIT)

## What went well

- Smallest, lowest-risk task of the batch: reused the EXISTING asset
  (`icons/nova_crt_mark.png`) and the drawer plate's exact `ImageNode` +
  `Option<Res<AssetServer>>`-guard pattern, so there was nothing novel to get
  wrong. The reviewer confirmed consistency and found no correctness issues.
- The AssetServer guard kept the headless test rigs working (count + TAB still
  spawn without an AssetServer), and a dedicated rig with `AssetPlugin` proves
  the icon actually spawns + leads.

## What went wrong

- Two doc surfaces described the item as text-only ("no pill/glyph" module doc,
  "count + TAB, plain text" struct doc). I caught the module doc via the
  doc-surface sweep but missed the per-struct doc one line above the code I was
  editing; the reviewer caught it (NIT R1.1). A reminder that the doc sweep is
  not just README/wiki - it includes the doc comments right next to the change.
- A borrow-checker slip in the test (held `&ChildOf` across a `world_mut()`),
  fixed by mapping to an owned `Entity`.

## What to improve next time

- When a change alters what a widget renders, grep the widget's OWN doc comments
  (module + per-struct + per-field) for the old description, not only the
  external doc surfaces - they are the closest and easiest to miss.

## Action items

- [x] Fixed both the module and per-struct docs to mention the star icon.
- No new ledger entry: this is covered by the promoted `keep-docs-in-sync`
  lesson (the nuance here is "docs adjacent to the code count too", already
  implied).
