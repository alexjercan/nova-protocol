# Fix record: CI clippy warning cleanup (20260719-001600)

Goal: zero warnings from CI's exact gate,
`cargo clippy --workspace --all-targets --features debug`. 45 warning sites
at baseline; the local run matched the user's CI paste exactly. The
warnings arrived from a nightly toolchain bump (the repo floats on
`channel = "nightly"`), not from code changes.

## How the fixes were applied

`cargo clippy --fix` first (machine-applicable suggestions: lifetimes,
`is_multiple_of`, `is_none_or` x5, `then_some`, `useless_conversion`, one
`useless_vec`, and the `items_after_test_module` move in audio.rs), then the
diff was re-read line by line before hand-fixing the rest. That re-read
caught one WRONG auto-fix (shakedown.rs below).

## Non-mechanical choices

- **Doc-comment warnings are prose bugs, not indentation bugs.** Every
  `doc_lazy_continuation` / doc-quote site (flight.rs, loader.rs,
  mod_cache.rs, nova_core lib.rs, portal_install.rs, 12_menu_newgame.rs,
  shakedown.rs) was a wrapped line that happened to start with `-`, `+`, or
  `>=`, which markdown reads as a list/quote marker. Clippy's suggested fix
  (indent, or add `>` markers) would bake the misparse into the rendered
  docs. Fixed by rewrapping the prose so no line starts with a marker
  character; shakedown.rs's `>=53u` became "at least 53u".
- **shakedown.rs auto-fix reverted:** `--fix` turned three lines of normal
  prose into a markdown blockquote (`/// >`), because the preceding line
  began with `>=53u`. Reverted and rewrapped instead.
- **`assertions_on_constants` (nova_assets sections.rs):** the three balance
  pins moved into `const { assert!(...) }` blocks inside their existing
  `#[test]` fns - a STRONGER guard (a flattening regression now fails at
  compile time of the test target, not at test runtime). Cost: const panics
  cannot format values, so the messages now name the constants instead of
  printing them; the values are one click away at the definition site. The
  `assert_eq!(CONTROLLER_BASE_HEALTH, TORPEDO_BASE_HEALTH)` stayed runtime
  (the lint did not flag it, and assert_eq formats its operands).
- **`large_enum_variant`, two different fixes:**
  - `nova_modding::Content::Section` is now `Box<SectionConfig>` (the real
    fix). `Box<T>` serializes exactly like `T`, so the RON wire shape is
    unchanged - the content parity tests pin that. Call sites: the one
    production constructor (`build_section_content`), the four clone-out
    sites (`merge_content_item` x2, the lint walker x2) which became
    `cfg.as_ref().clone()`, and the test constructors wrapped in
    `Box::new`. `rewrite_refs` round-trips through serde_json, so it needed
    no change.
  - `nova_scenario::SectionSource::Inline` got a justified
    `#[allow(clippy::large_enum_variant)]` instead: the enum derives
    `Reflect`, and bevy_reflect 0.19 has NO `Reflect` impl for `Box<T>`
    (verified in the registry source - `src/impls/alloc/` has no `boxed.rs`),
    so boxing cannot compile without stripping reflection from the whole
    scenario-config tree. It is spawn-time config data, not per-frame state.
- **`field_reassign_with_default` (turret tests x3):** the
  `{ let mut c = default(); c.muzzle_speed = ...; c }` blocks became struct
  literals with `..Default::default()`, keeping the inline comment.
- **`while_let_loop` (turret joint walk):** the first let-else break became
  the `while let` condition; the second let-else (joint check) stays inside
  the body, and `chain = parent` still advances at the bottom - behavior
  identical.
- **CI gate deliberately NOT tightened.** No `-D warnings` added to the
  clippy step: `rust-toolchain.toml` floats on unpinned nightly, so
  enforcement would let any future nightly's new lints redden CI with zero
  code changes (exactly how this batch appeared). If the user wants
  enforcement, pin the nightly date and add `-- -D warnings` together -
  surfaced as a follow-up fork in the flow report.

## Out of scope

Dependency `future-incompat` notices (naga, wgpu, winit, nix,
proc-macro-error2) are upstream and untouchable from this repo.
