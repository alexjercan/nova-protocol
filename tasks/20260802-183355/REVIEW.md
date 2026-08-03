# Review: Document nova_autopilot: rustdoc, prelude, and the dev wiki page

- DATE: 20260803-000000
- TASK: 20260802-183355
- BRANCH: docs/autopilot-docs
- WORKTREE: /home/alex/.cache/sprouts/nova-protocol/docs/autopilot-docs
- BASE: master (8f9d7fa7)

## Round 1

- REVIEWER: out-of-context `general-purpose` subagent (agent
  `a3e32f0dfd146e797`), prompted with task ID, worktree, dimensions and record
  format only. Primary re-derived R1.1, R1.2 and R1.3 independently from the
  source before accepting them.
- VERDICT: REQUEST_CHANGES

Checked the diff (`master...HEAD`) against Story, Steps, DoD, the root
`AGENTS.md` and the crate source it documents.

### R1.1 MAJOR - the wiki opt-in snippet teaches the pattern nova_debug forbids

`web/src/wiki/dev/automation-harness.md:79-81` shows

```
AutopilotPlugin::new().hold(GameStates::MainMenu, 0.5).hold(GameStates::Playing, 6.0)
```

as how a Nova example opts in. `hold` force-sets `NextState`, and
`crates/nova_debug/src/harness.rs:9-23` documents at length why Nova must not
do that: `Loading -> Playing` is asset-gated by the loader, so forcing
`Playing` either fires before `GameAssets` exists (panicking scene setup) or
re-enters `Playing` after the loader already did (double-running
`OnEnter(Playing)`). The real preset is
`AutopilotPlugin::new().hold(GameStates::Loading, NOVA_AUTOPILOT_SECS)`
(`harness.rs:89-91`). A contributor who copies the wiki page writes a broken
harness, and the page is the one artifact whose job is to stop that.

Change: make the snippet crate-generic (the `driven_app` three-state shape, or
an unnamed `GameState`), or use the real `Loading`-hold preset and say why.

Response: fixed in 8cb01f55. Did both. The snippet is now the `driven_app`
`DemoState` shape (no Nova type at all), and a paragraph under it states that
`hold` force-sets `NextState`, spells out the two failure modes for an
asset-gated `Loading -> Playing`, names the real preset
`hold(GameStates::Loading, NOVA_AUTOPILOT_SECS)` with its file, and generalizes
the rule: hold a state something else is responsible for entering and you get
the same bug.

### R1.2 MAJOR - the page states adoption that has not happened

The page is present tense about a crate nothing depends on yet.
`rg 'nova_autopilot' --glob '**/Cargo.toml'` hits only the workspace member
list and the crate's own manifest. Concretely:

- `automation-harness.md:93-95`, the shell block:
  `NOVA_AUTOPILOT=1 cargo run --example scenario` is inert today.
  `examples/gameplay/scenario.rs:55` uses `nova_debug::harness::nova_autopilot()`,
  which re-exports the `bevy_common_systems` plugin
  (`nova_debug/src/harness.rs:63-66`) gated on `BCS_AUTOPILOT`. The documented
  command runs the example with no autopilot at all.
- `automation-harness.md:18-23`, the "Nova uses it for" column, and
  `:25-27`, "`nova_probe` is the layer above: it arms these variables" -
  `nova_probe` arms `BCS_AUTOPILOT` / `BCS_HARNESS_DEADLINE`
  (`crates/nova_probe/src/bin/probe/native/env.rs:100,120`) and never names
  `nova_autopilot`.
- The one caveat (`:51-53`) reads as an env-naming footnote. The callers are
  not on a differently-spelled variable; they are on a different crate.

TASK.md's Notes anticipated the naming half of this and correctly scoped the
rename to `20260802-183403`. The defect is the tense, not the scope: a
contributor-facing page must not describe a follow-up task's end state as
current fact.

Change: state once near the top that the crate is the extracted home and that
Nova's own callers migrate in `20260802-183403`; put the driver table's third
column and the shell block in that same tense (or mark them post-migration).

Response: fixed in 8cb01f55. A bolded paragraph after the intro now says to read
the page as the crate's contract rather than today's wiring, names
`nova_debug::harness` and the legacy `BCS_*` variables as what Nova's examples
and `nova_probe` actually run, and names `20260802-183403` as the migration. The
driver table's third column is "What Nova will use it for"; the `nova_probe`
sentence says it arms the legacy names today; the shell block leads with the
`driven_app` command that runs today and labels the `scenario` forms as
post-migration. The `wiki-pages.ts` summary lost its "how the game drives
itself" framing for the same reason.

### R1.3 MINOR - both env tables omit the only cross-variable interaction

`ScreenshotPlugin::build` stands down entirely when `NOVA_AUTOPILOT` is also
set (`crates/nova_autopilot/src/screenshot.rs:156-165`, warn-only, with a
dedicated `tests/screenshot_stand_down.rs`). Neither the crate table
(`lib.rs:20-31`) nor the wiki table lists it, so a reader who arms both from
the table gets no PNG and no error. Change: extend the `NOVA_SHOT` row in both
tables - "ignored when `NOVA_AUTOPILOT` is also set; the autopilot wins".

Response: fixed in 8cb01f55. Both `NOVA_SHOT` "Arms" cells now carry the
stand-down, with the reason (both drivers write `NextState`) and the observable
(a warning, no PNG).

### R1.4 MINOR - the prelude scan cannot see a new module

`crates/nova_autopilot/tests/prelude.rs:36-42` hardcodes the four module
files. Adding a fifth `pub mod` to `lib.rs` with any number of public items
leaves `prelude_names_every_public_item` green - the exact failure the DoD
("a new public item that skips the prelude fails the build") claims to close.
Change: also `include_str!("../src/lib.rs")`, scan its `pub mod ` lines, and
assert every module except `prelude` appears in `MODULES`.

Response: fixed in 8cb01f55. `tests/prelude.rs` gained `LIB_RS` and a second
test, `every_module_is_scanned`, which scans `lib.rs` for column-0 `pub mod`
lines, skips `prelude`, and asserts each remaining module has a `MODULES` entry
- plus a length equality, so a `MODULES` entry for a file `lib.rs` does not
declare fails too. Proven to bite: adding `pub mod probe_only;` with a public
const failed the new test with the "public module but MODULES does not scan it"
message, and was reverted.

### R1.5 MINOR - `NOVA_SHOT_DIR`'s "Read by" is incomplete

`lib.rs:29` and the wiki row both name only `ScreenshotReelPlugin`. The
variable is read by the shared path resolver, which also backs the public
`capture_window` (`reel.rs:247-253`), whose own rustdoc advertises it for
callers scripting their own beats without the reel plugin. Change:
"`ScreenshotReelPlugin` and `capture_window`".

Response: fixed in 8cb01f55, in both tables; the crate table links
`capture_window` intra-doc.

### R1.6 NIT - the `BCS_*` caveat implies a mechanical prefix swap

`automation-harness.md:51-53` sends a reader back to today's run scripts with
`BCS_` substituted. That is wrong for one variable: the legacy name is
`BCS_HARNESS_DEADLINE`, not `BCS_AUTOPILOT_DEADLINE`
(`nova_probe/.../env.rs:100`). Change: name the variable whose stem also
changed.

Response: fixed in 8cb01f55. The caveat now says it is not a mechanical prefix
swap and names `BCS_HARNESS_DEADLINE -> NOVA_AUTOPILOT_DEADLINE`.

### R1.7 NIT - `lib.rs:78` overstates the enforcement mechanism

"fails the build when a new `pub` item skips this list" - only a *deleted*
re-export fails compilation; a new unexported item fails the
`prelude_names_every_public_item` assertion, which `tests/prelude.rs:1-5`
describes accurately. Change: "fails `tests/prelude.rs`".

Response: fixed in 8cb01f55 - "fails when a new `pub` item skips this list",
with `tests/prelude.rs` already named as the subject of that sentence.

### R1.8 NIT - CHANGELOG entry breaks the file's own format

`CHANGELOG.md:28-30` is a three-clause sentence spanning wiki, prelude, table
and doc examples; the header and root `AGENTS.md` require one short
commit-title line per entry with no rationale. "doc examples throughout" also
overstates - three of the four modules already had one on base; this branch
added exactly one (`completion.rs`). Change: split into two short lines, drop
"throughout".

Response: fixed in 8cb01f55. Two lines now: the wiki page, and the crate's
prelude + env table + `completion` doc example.

### Verified claims

- All five Steps and all five DoD proofs reproduce green in the worktree:
  `cargo test -p nova_autopilot --lib --test prelude` (15 + 2 pass),
  `cargo test --doc -p nova_autopilot` (4 compile-only, was 3),
  `RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps` clean,
  `cd web && npm run ci` exit 0 with
  `dist/wiki/dev/automation-harness/index.html` emitted, `cargo fmt --check`
  clean. Declared `headings` in `web/src/wiki-pages.ts` match the page's four
  rendered `<h2>` ids exactly.
- Prelude covers all 19 column-0 `pub` items of the four modules; `rg '^pub '`
  agrees. The scan was independently shown to bite on an added public const.
- `20260802-183349` R1.1 is cleared correctly, and the close-out is right that
  the plan's wording was wrong: `reel_drive` (`reel.rs:293-302`) consults the
  predicate before every step for the whole run, not only until the first
  `true`. The doc says the true thing.
- `20260802-183349` R1.2 is cleared without the env race the original test
  avoided: `resolve_capture_path` is split out pure and all five branches are
  asserted deterministically.
- `20260802-183352`'s `driven_app` pointer is present in both `lib.rs:52-60`
  and the wiki page.

### Pending manual checks

None.

### Inspection commands

```sh
cd "$(sprout show docs/autopilot-docs)"
git diff master...HEAD
rg -n 'nova_autopilot' --glob '**/Cargo.toml'
rg -n 'BCS_AUTOPILOT|BCS_HARNESS_DEADLINE' crates/nova_probe/src/bin/probe/native/env.rs
sed -n '9,23p;89,91p' crates/nova_debug/src/harness.rs
sed -n '150,166p' crates/nova_autopilot/src/screenshot.rs
```

## Round 2

- REVIEWER: out-of-context `general-purpose` subagent (agent
  `af89b833d7ae914bb`), prompted with task ID, worktree, dimensions, record
  format and the round-1 findings only. Primary re-derived R2.1 and R1.4
  independently from the source, and re-ran all five proofs plus
  `cargo fmt --check`.
- VERDICT: APPROVE

All eight round-1 findings verified fixed at root cause, from the code rather
than the Responses: R1.1 (snippet is `driven_app`'s `DemoState`, no Nova type,
with the force-set caveat and the real `hold(GameStates::Loading, ...)` preset
named), R1.2 (bolded framing paragraph, "What Nova will use it for" column, the
`nova_probe`-arms-legacy-names sentence, shell block leading with the command
that runs today), R1.3 and R1.5 (both env tables), R1.4 (`every_module_is_scanned`
scans `lib.rs` for column-0 `pub mod`, skips `prelude`, asserts membership in
BOTH directions, and the test runs green), R1.6, R1.7 and R1.8. No regressions
in `8cb01f55`; `wiki-pages.ts:535-540` headings still match the page's four
rendered `<h2>` ids after the edits.

Two NITs remain; neither blocks.

- [x] R2.1 (NIT) web/src/wiki/dev/automation-harness.md:104 - "forcing
  `Playing` either beats `GameAssets` into existence (panicking scene setup)"
  inverts its source. `crates/nova_debug/src/harness.rs:12-14` says the
  force-set "would fire before the `GameAssets` resource exists (panicking
  scene setup that reads it)"; "beats it into existence" reads as forcing the
  resource to appear, the opposite failure. Change to "fires before the
  `GameAssets` resource exists (panicking the scene setup that reads it)".
  - Response: fixed. The sentence now reads "fires before the `GameAssets`
    resource exists (panicking the scene setup that reads it)".
- [x] R2.2 (NIT) web/src/wiki/dev/automation-harness.md:132 - R1.7's fix landed
  in `lib.rs` only; the wiki page still says "a test fails the build if a new
  one is not". A new unexported item fails the `prelude_names_every_public_item`
  assertion, not compilation. Change to "and `crates/nova_autopilot/tests/prelude.rs`
  fails if a new one is not".
  - Response: fixed. The page now names `crates/nova_autopilot/tests/prelude.rs`
    as what fails, matching `lib.rs`.

### Verified claims

- All five DoD proofs re-run green in the worktree by the primary:
  `cargo test -p nova_autopilot --lib --test prelude` (15 + 3 pass),
  `cargo test --doc -p nova_autopilot` (4 compile-only),
  `rg -n '^/// ```|^//! ```' crates/nova_autopilot/src/completion.rs` (fence at
  36/65), the three-registry `rg`, `cd web && npm run ci` exit 0 with
  `dist/wiki/dev/automation-harness/index.html` emitted, and
  `RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps` clean.
  `cargo fmt --check` clean, tree clean.
- R1.4 re-derived: `every_module_is_scanned` strips `pub mod ` at column 0,
  trims `;`/` `/`{` (so the inline `pub mod prelude {` is caught and filtered),
  and pairs the per-module membership assert with `declared.len() ==
  MODULES.len()`, so a fifth module and a stale `MODULES` entry both fail.
  `pub(crate) mod` correctly does not match.
- The close-out's numbers reproduce exactly: 15 lib + 3 prelude tests, 4 doc
  tests (3 on base). The round-1 fixes section corrects the close-out's earlier
  "15+2" to "15+3"; both readings are of the same run, the later one right.
- Not run locally, as the close-out declares: the display-dependent integration
  tests (`tests/reel.rs`, `tests/screenshot*.rs`, `tests/autopilot_example.rs`)
  and workspace clippy. CI owns them.

Process signal: round 1's eight findings were all real and all fixed at root
cause rather than caveated, and R1.4 changed enforcement rather than prose. The
one recurring shape worth mining is that a docs task for a freshly EXTRACTED
crate defaults to writing the post-migration world as present fact - R1.2 and
R2.2 are both that, and neither the plan nor the close-out had a tense rule.

### Pending manual checks

None.

### Inspection commands

```sh
cd "$(sprout show docs/autopilot-docs)"
git show 8cb01f55
sed -n '9,15p' crates/nova_debug/src/harness.rs
sed -n '100,132p' web/src/wiki/dev/automation-harness.md
nix develop --command cargo test -p nova_autopilot --lib --test prelude
```
