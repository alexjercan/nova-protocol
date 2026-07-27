# Review: NOVA CRT star mark icon on the Computer/TAB status item

- TASK: 20260727-135217
- BRANCH: feature/nova-os-tab-star-icon

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

DoD proofs run by the reviewer: `objective_hint` tests PASS (3 passed, incl. the
new `objective_hint_shows_the_nova_crt_star_icon`); `cargo check -p nova_gameplay
--all-targets` clean.

Reviewer confirmed independently: the icon is a child of `ObjectiveHintMarker`
so it collapses with the parent's `Display::None` toggle; the
`Option<Res<AssetServer>>` guard mirrors the drawer plate and degrades safely
headless (count + TAB still spawn, no panic); `assets/icons/nova_crt_mark.png`
exists on disk; the reveal tuck anchor reads the hint's `GlobalTransform` + a
fixed `HINT_ANCHOR_SIZE`, not the icon rect, so the added icon cannot break it;
the new test would fail if the icon were missing, mis-parented, or not leading.

- [x] R1.1 (NIT) crates/nova_gameplay/src/hud/objective_hint.rs:38 - the
  `ObjectiveHintMarker` per-struct doc still read "(count + TAB, plain text)",
  stale now that the item leads with the star glyph (the module-level doc was
  already updated).
  - Response: fixed - the struct doc now reads "(star mark icon + count + TAB)".
    Confirmed mod.rs:244 already says "count + glyph + Tab" (consistent).

Pending user checks (manual DoD, cleared at flow Finish):
- Owner confirms the NOVA CRT star mark appears on the top-bar TAB item and it
  still collapses when there are no objectives.
