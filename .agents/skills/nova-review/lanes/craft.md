# Lane: craft

Judge whether the change is the simplest correct shape for this repository, and
whether it obeys the house rules.

Read `AGENTS.md` first. This brief only says where to look.

## Look for

- Machinery this repo bans: compatibility shims, adapters, aliases, legacy
  branches, and options that exist to avoid changing an owned caller.
- A replaced path left beside the new one. Find its callers, then propose
  deletion of the path, stale comments, and tests that protect old behavior.
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
- Comments that narrate code or history instead of stating a concrete reason,
  constraint, owner, failure, or debt. Flag vague terms and stale comments.
- An app or example not built with `AppBuilder`. Gameplay randomness not seeded
  through `bevy_rand`.
- Two ways to say the same thing: a field the user can set that another field
  already decides.

## Ignore

- Frame cost, test coverage, formats, and documentation. Other lanes own them.
- Anything `rustfmt` already settles.
