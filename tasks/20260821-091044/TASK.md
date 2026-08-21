# Give the environment variables one shape, one home, and one page

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog

# Give the environment variables one shape, one home, and one page

Came out of a review of the fixed-step instrumentation, which read four env vars
from inside `NovaGameplayPlugin::build` - a SHIPPED plugin, with no
`#[cfg(feature = "debug")]` anywhere in the file, and one of them
(`NOVA_SUBSTEPS`) able to `panic!` on a malformed value. That is the symptom.
The defect is that this codebase has no rule about where an environment variable
is declared, what it is called, or which subsystem owns it.

## What is actually there

Env vars are read THREE different ways at once:

- **Bare literals at the use site.** `"NOVA_AUTOPILOT"` appears at 18 separate
  `env::var` sites.
- **Named constants.** `PERF_ENV`, `NORENDER_ENV`, `RENDER_DIAG_ENV`,
  `AUTOPILOT_ENV`, `CAPTURE_ENV`, `SCREENSHOT_ENV`, `SHOT_DIR_ENV`,
  `DEADLINE_ENV`, `PROBE_MODE_ENV`, and more.
- **Computed names.** `format!("NOVA_PERF_{}", ...)`,
  `format!("NOVA_STRESS_PD_{knob}")`.

**`NOVA_AUTOPILOT` is read as a bare literal 18 times AND has an
`AUTOPILOT_ENV` constant.** Both forms coexist for the same variable. The same
is true of `NOVA_PERF` and `NOVA_SHOT_DIR`.

Only THREE constants live in `nova_core`: `NOVA_NORENDER`, `NOVA_PERF`,
`NOVA_PERF_RENDER_DIAG`. Everything else is declared per-crate or not at all.

This is `CONVENTIONS.md` Nova rule 5 - a runtime string nothing type-checks,
renamed silently, failing at load - applied to a surface that rule never named.

**NOT in scope, and worth writing down because it was miscounted once:** the
~50 `NOVA_OS_*` names are `const Color` and layout values in
`nova_os_ui/src/terminal/style.rs`. They are NOT environment variables. A grep
for `NOVA_[A-Z_]*` returns about 130 names; only a fraction are read from the
environment. Count `env::var` call sites, not identifiers.

## 1. Collapse `NOVA_SHOT` into the capture path

Today there are two picture mechanisms:

- `NOVA_SHOT` arms `ScreenshotPlugin` - force-advance to `Playing` on frame one,
  settle, hide overlays, shoot, done. **20 `systems/` ranges** use it via
  `nova_screenshot()`.
- `NOVA_CAPTURE` puts an autopilot script on its capture path so it shoots at
  its own beats. **10 examples** branch on `capturing()`. Loops are on this same
  path.

Both are live; neither is dead code. But the second is strictly more general,
and `nova_screenshot()` is a degenerate script: one beat, wait for settled,
shoot.

The evidence that they occupy ONE role rather than two:

- They collide. Both write `NextState`, so `NOVA_SHOT` **stands down with a
  warning** whenever `NOVA_AUTOPILOT` is set. Two things doing genuinely
  different jobs do not need a stand-down rule.
- `nova_screenshot()` force-advances to `Playing` on the first frame, which is
  why its own doc warns it only suits examples that build their scene in
  `OnEnter(Loaded)` rather than `OnEnter(Playing)`. A one-beat script has no
  such caveat.
- `NOVA_SHOT_DIR` is declared in `capture.rs` and is the CAPTURE path's output
  directory. The one name shared between the two families already belongs to the
  other one.

**Do**: rebuild `nova_screenshot()` as a one-beat capture preset over the script
mechanism, keeping the same one-line ergonomics for the 20 ranges. Delete
`ScreenshotPlugin`, `SCREENSHOT_ENV`, the stand-down rule and the
`OnEnter(Loaded)` caveat. `NOVA_SHOT_DIR` becomes `NOVA_CAPTURE_DIR`.

"Capture" is the right noun for the whole thing, screenshots and loops alike.

## 2. `NOVA_PERF_*` becomes `NOVA_PROBE_*`

`nova_core/src/lib.rs` already documents that the `NOVA_PERF_*` prefix predates
the crate's rename to `nova_probe`, and `NOVA_PROBE_MODE` already exists
(`nova_probe/src/capabilities/mod.rs`). **This finishes a rename that is already
half-done**, it does not start one.

Covers `NOVA_PERF`, `NOVA_PERF_WARMUP`, `NOVA_PERF_FRAMES`, `NOVA_PERF_OUT`,
`NOVA_PERF_LABEL`, `NOVA_PERF_PRESENT`, `NOVA_PERF_MAX_DELTA`,
`NOVA_PERF_RENDER_SCALE`, `NOVA_PERF_CENSUS_FRAME`, `NOVA_PERF_TIMELINE`,
`NOVA_PERF_INVARIANTS`, `NOVA_PERF_CONTRACT`, `NOVA_PERF_SHA`,
`NOVA_PERF_HOST`, `NOVA_PERF_RENDER_DIAG`, and the `perf_param()` helper's
computed form.

Breaking, no back-compat alias, per the standing rule. **Nothing type-checks
these**, so grep every surface: `crates/`, `examples/`, `web/`, `docs/`,
`.github/`, `flake.nix`, any script. Then RUN what changed.

## 3. Outputs-off becomes a real family

`NOVA_NORENDER` (no renderer) and `NOVA_MUTE` (`HarnessMute`,
`nova_gameplay/src/settings.rs`) are the same idea: turn one output device off
for a headless or harness run. They currently sit in different crates with
different naming shapes, and only the render one has a CLI flag.

Make them a family, and give `NOVA_MUTE` a `--mute` flag to match `--norender`.

## 4. Modding gets its own prefix

`NOVA_MOD_CACHE_ROOT` and `NOVA_PORTAL_URL` become `NOVA_MODDING_*`. Both belong
to the mod portal and cache, which is about to matter more once mods are
published to a server.

`NOVA_CONFIG_ROOT` stays exactly as it is - it is the settings store root, not
modding, and the name is already right.

## 5. Every env var is a named constant with ONE home

No bare literals at use sites. Each variable is declared once, in the crate that
OWNS the behaviour it gates, and imported from there.

The gate goes where the behaviour goes. **Measurement knobs belong in
`nova_probe`, never in a gameplay plugin.** The four fixed-step knobs are the
worked example of getting this wrong.

`crates/nova_autopilot/tests/env_contract.rs` already pins two names by
assertion. Extend that idea to the whole set: one test that names every
environment variable the game reads, so adding one without declaring it fails.

## 6. Promote the step-diagnostics instrument

Sprout `fixed-step-investigation` carries env-gated per-step instrumentation
(`crates/nova_gameplay/src/plugin.rs`): a per-step CSV of avian's own phase
diagnostics plus wall time, contact and constraint counts, and body and collider
counts - with **body-count regime selection**, which is what made the fixed-step
investigation resolvable at all.

Move it into `nova_probe` as a real capability under `NOVA_PROBE_*`, beside
`nova_census()` and `nova_frametime()`. Drop the `NOVA_NO_PREPHYS_PROPAGATE`
attribution arm; it was a one-off ablation, not an instrument.

**Why it matters beyond tidiness**: whole-run averages over the arena are not
comparable between arms, because a faster simulation ends the fight sooner and
quietly measures a lighter scene. Regime selection is the fix, and it is
currently parked on a sprout that would otherwise be deleted.

## 7. One page in the dev book

An "Environment variables" section: every variable, what it gates, which crate
owns it, and which are harness-only versus player-facing. Route per
`docs/keeping-docs-in-sync.md`; the dev book is the mechanism surface.

## Explicitly not in scope

- `NOVA_OS_*` - constants, not env vars.
- Example-local knobs (`NOVA_STRESS_PD_*`, `NOVA_MENU_BACKDROP`). They belong to
  one example each, and `examples/` deliberately keeps literals because a probe
  run goes red when one drifts (`CONVENTIONS.md` Nova rule 5).
- Foreign variables the code legitimately reads: `DISPLAY`, `WAYLAND_DISPLAY`,
  `RUST_LOG`, `CARGO_*`, `CI`, `LVP_ICD`, `RUSTFLAGS`.

## Definition of done

- No `env::var` call site in `crates/` or `src/` takes a bare `"NOVA_..."`
  literal; each reads a declared constant.
- `ScreenshotPlugin` and `SCREENSHOT_ENV` are gone; the 20 ranges that used
  `nova_screenshot()` still produce their pictures, verified by RUNNING them,
  not by compiling them.
- No `NOVA_PERF_*` string survives anywhere in the repo, including `web/`,
  `docs/`, `.github/` and scripts.
- `--mute` exists and matches `--norender`.
- The step-diagnostics capability lives in `nova_probe` and the
  `fixed-step-investigation` sprout can be deleted.
- One contract test names the whole set.
- The dev book page exists and `docs/keeping-docs-in-sync.md` routes to it.
- `CHANGELOG.md` carries the breaks, tagged **(breaking)**.
