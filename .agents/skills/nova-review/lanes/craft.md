# Lane: craft

Judge whether the change is the simplest correct shape for this repository, and
whether it obeys the house rules.

Read `AGENTS.md` first. This brief only says where to look.

## Look for

- Machinery this repo bans: compatibility shims, adapters, and options that
  exist to avoid changing a caller. Prefer the correct, simple, maintainable
  change.
- The same logic in two places. A helper that abstracts exactly one caller.
- Code in the wrong crate, and a dependency edge added only for a constant. A
  cross-crate identifier belongs in the lowest crate its consumers share.
- An import that bypasses a prelude. Every exporting module has one and the
  crate root exports it, including for use inside the same crate.
- Names: `<Subsystem>Plugin`, `<Subsystem>Systems`. Cross-plugin ordering stated
  explicitly, not implied by insertion order.
- `#[allow(...)]` where `#[expect(<lint>, reason = "...")]` belongs. A new
  workspace-wide pedantic, nursery, wildcard-import, redundant-pub-crate,
  needless-pass-by-value, or private-missing-doc lint.
- Tests in the wrong place: unit tests inline or in `src/**/tests/`, and
  `crates/*/tests/` for integration tests only.
- Comments that narrate the code or its history instead of stating ownership and
  constraints. A comment an edit left stale.
- An app or example not built with `AppBuilder`. Gameplay randomness not seeded
  through `bevy_rand`.
- Two ways to say the same thing: a field the user can set that another field
  already decides.

## Ignore

- Frame cost, test coverage, formats, and documentation. Other lanes own them.
- Anything `rustfmt` already settles.
