# G3+G4 contracts lane

agent: agent-aba9b605f8637b620
last_ts: 2026-09-09T20:47:41.028Z
stop_reason: end_turn
records: 320

## Dispatch prompt

```
You are the Contracts lane of a Nova Review panel.

Repository: /home/alex/personal/nova-protocol (branch master).

Read first, in order:
1. /home/alex/personal/nova-protocol/AGENTS.md
2. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/reviewer.md
3. /home/alex/personal/nova-protocol/.agents/skills/nova-review/lanes/contracts.md

Range under review: `45372be15^..ba0cc418f` plus `62043f3b5` and `9e69ac196`. Four commits:
- 45372be15 "Keep the status bar out of every capture that keeps the HUD"
- ba0cc418f "Tell a driver the truth about the radar dwell and the aim cap"
- 62043f3b5 "Let the scenario sweep tolerate a root that already died"
- 9e69ac196 "Compile the news loop producers without the debug feature"

Bundle, already built. Do NOT re-derive the range:
- /tmp/nightly-20260909/g34/log.txt
- /tmp/nightly-20260909/g34/stat.txt
- /tmp/nightly-20260909/g34/diff.txt   (the last two commits appended under `=== <sha> ===` headers)

Touched: crates/nova_bench/{gesture,manual,observation}.rs and its pages/*.md, crates/nova_channel/{apply,lib}.rs, crates/nova_debug/harness.rs, crates/nova_probe/capabilities/snapshot.rs, crates/nova_scenario/loader/lifecycle.rs, docs/environment-variables.md, and several examples.

Pay particular attention to: whether every new or changed `NOVA_*` environment variable is registered in docs/environment-variables.md and passes tests/env_contract.rs; whether the nova_bench page copy (pages/targeting.md, manual.md) now matches the code it describes after "Tell a driver the truth about the radar dwell and the aim cap"; whether any player- or creator-facing figure is printed in meters per AGENTS.md; whether the changed capture behaviour leaves any documented screenshot or wiki image stale; and whether anything here is a format break owed a **(breaking)** changelog note. These four commits shipped in v0.13.0 (release commit e8228c7da the same day).

Hard constraints:
- Read-only. Never edit, stage, commit, or fix anything.
- You do NOT hold the measurement slot. Do not run a rendered example, a probe, or any benchmark.
- Never run the workspace test suite or Clippy.

Report findings only, strongest first, in the reviewer contract's format, and close with `Checked:` and `Not checked:`.
```

## Final report

MAJOR - crates/nova_bench/src/pages/targeting.md:22 - the dwell figures the page keeps contradict the shipped curve and the player wiki, in the commit's own subject.

The commit separates the 15 search ticks from the dwell one line below, but leaves the sentence above attributing hold-to-lock TOTALS to "the dwell". `lock_dwell_secs` (crates/nova_ship/src/input/targeting/radar.rs:253-267) over the shipped defaults (crates/nova_ship/src/input/targeting/state.rs:84-88 - base 0.6, factor 1.5, reference 2000 world units = 20 km) gives `dwell_needed` = 0.645 s (39 ticks) at 1 km and 0.7125 s (43 ticks) at 2500 m. `web/src/wiki/targeting-radar.md:33` states the same curve correctly ("0.6 s point-blank to 1.5 s at 20 km"). Failure: a driver holds `targeting.radar_hold` on a contact at 2500 m, reads `dwell_needed: 0.71` in `me.radar`, and cannot reconcile it with the page's "about 60 ticks" - the exact confusion the commit set out to remove; the 60 and the "about a second" only come out right if the 15 search ticks are counted in, which the next sentence says they are not.

Change: say what the two numbers are. "A lock takes about a second from the press inside a kilometre and about 60 ticks at 2500 m - fifteen of those the search, the rest the dwell that `dwell_needed` names (0.6 s point-blank, 1.5 s at 20 km)."

Not BLOCKER: the page's operative advice - read `dwell_fill`, hold to 1 - is correct and is what a driver acts on.

MAJOR - docs/automation-harness.md:492 - the routed book chapter still scopes the frame-clocked deadline to "inside a loop" after the range established it is process-wide on any armed run.

`LoopCapturePlugin::build` inserts `TimeUpdateStrategy::ManualDuration(profile.frame_duration())` for the whole app the moment `capture::capturing()` holds (crates/nova_autopilot/src/loops.rs:343-351) - which is what the new `nova_debug::harness` section says ("from plugin build on and not just while a loop is open") and what the three rewritten example comments cite. The chapter and `crates/nova_autopilot/src/loops.rs:57-62` both still say the pin applies to a deadline "inside a loop". `docs/keeping-docs-in-sync.md` routes `nova_debug/harness.rs` to this chapter, and the commit's whole contribution to harness.rs is documentation. Failure: a contributor sizes a beat that runs OUTSIDE `loop_start`/`loop_end` on a capturing producer - `examples/screenshots/loop_command_shell.rs:178` `.deadline(30.0)` on "settle at the start" is one - believing the book that it counts wall seconds, and sizes it as a backstop against the ~0.75 s/frame software floor. Armed, the clock is already pinned, so 30.0 is 900 frames, about eleven wall minutes on that host; the named-step backstop stops being one and `NOVA_AUTOPILOT_DEADLINE` is frame-clocked too, so a genuinely stuck pre-loop beat is named by nobody.

Change: rewrite the "One exception" paragraph and the `loops.rs` "Cadence" paragraph to say the pin covers the armed run end to end, and point both at "What a step deadline counts".

Not BLOCKER: no shipped code path is wrong, and the error makes a deadline too generous rather than too tight.

MAJOR - docs/environment-variables.md:176 - the page that indexes "every `NOVA_*` variable" omits seven names, three of them capture-pipeline flags of exactly the class the range just spent five lines documenting.

The range rewrote this bullet and gated one of the missing names' declarations (`examples/screenshots/loop_goto_standoff.rs:277`). Absent from the page: `NOVA_STANDOFF_TRACE`, `NOVA_COLLAPSE_LOOP` (examples/systems/stress_hull_collapse.rs:191), `NOVA_SIGHT_LOOP` (examples/systems/system_lock_line_of_sight.rs:94), `NOVA_WAKE_LOOP` (examples/playable/railgun_wake_bench.rs:986), `NOVA_MENU_PATH` (examples/systems/system_menu_boot.rs:14), and shell-only `NOVA_REUSE_STAGE` (scripts/capture-web-media.sh:184) and `NOVA_UNFREEZE` - which `docs/keeping-docs-in-sync.md` names as a real authoring lever. Failure: re-cutting the release loops, someone reads this bullet (now the fullest account of what the media pipeline needs) to learn which variables to set, sees only the railgun pair, and drives `stress_hull_collapse` / `system_lock_line_of_sight` / `railgun_wake_bench` without `*_LOOP=1`. `loop_requested()` is false, no webm is written, and `scripts/capture-web-media.sh:210` aborts naming the missing loop - the identical failure the bullet describes for `NOVA_RAILGUN_LIVE`, for names the page never lists.

Change: add the five example-local names to the "Example-local knobs" bullet and the two shell names to the "Shell-only" bullet.

Not BLOCKER: nothing ships wrong - `capture-web-media.sh` encodes the flags and fails loudly.

MINOR - docs/agent-bench.md:99 - the book's gesture table still shows `"ticks": K` unbounded after the manual gained the cap.

`parse_gesture` refuses anything outside `1..=MAX_AIM_TICKS` (crates/nova_bench/src/gesture.rs:198-204), `manual.md:101` now states it, and `manual.rs` pins the two together with a test. `docs/keeping-docs-in-sync.md` routes `nova_bench` to this chapter, which still presents `K` as free. Failure: a contributor writing a scripted agent off the book's table sends `{"aim": ..., "ticks": 60000}` - the millisecond-for-ticks slip the bound exists to catch - and meets the bound as a refusal the book never mentioned.

Change: write the cell as `"ticks": K` with `1 <= K <= 3600`, or point the row at `MAX_AIM_TICKS`.

Checked:

- Env contract. No new `NOVA_*` literal reaches `crates/`, `src/` or `tests/`; the roster in `tests/env_contract.rs` is unchanged and still covers the scanned tree, which excludes `examples/` on purpose. Enumerated every `NOVA_*` name under `examples/`, `scripts/` and `web/` and diffed it against `docs/environment-variables.md` (finding 3).
- The railgun env prose the range added: `aftermath_window` panics on a non-number and asserts `> 0.0`, `live_cut` panics on anything but `0`/`1` (examples/screenshots/screenshot_railgun.rs:174-226), and `scripts/capture-web-media.sh:188` `rm -f`s the target before the run with a hard `-s` check at 210. All accurate.
- The `stress_torpedoes` header claims: 45+15+90+90 = 240; `.github/workflows/ci.yaml:200` sets `NOVA_AUTOPILOT_DEADLINE: 280`; `crates/nova_autopilot/src/completion.rs:92` `DEFAULT_DEADLINE_SECS = 120`; `crates/nova_probe_cli/src/native/env.rs:94` pushes `DEADLINE_ENV` on the fps pass only, so a correctness-only run does get the 120 s default. All accurate.
- `SNAPSHOT_SCHEMA`'s "that field never shipped" argument: `git show v0.12.0:crates/nova_probe/src/capabilities/snapshot.rs` names neither `dwell_fill` nor `dwell_target` and pins `SNAPSHOT_SCHEMA = 1`. No `**(breaking)**` is owed for the gate; the schema-2 ack shape already has its entry in the v0.13.0 changelog.
- Stale imagery. `cc005e202` re-shot the nine stills the same night; `news-090-contextual-hud.png` keeps the bar deliberately and the commit says why; `news-0130-comms-channels.png` was captured after the fix. Every remaining shipped figure comes from a producer that reaches `hide_hud` or `hollow::hud_cinematic`, and the NOVA OS producers open NOVA OS, which hides the non-exempt flight status bar (`crates/nova_core/src/lib.rs:801-812`). No live wiki or news image is left stale by this change.
- `hide_status_bar` coverage. `HudVisibility::On` appears in `examples/` only in the three `hud_instrument` helpers, all three of which now drop the bar; `hide_status_bar` despawns the `HudTier::Status` root recursively and the two items are its children by the time any beat runs.
- The `nova_channel` module-doc rewrite. `docs/keeping-docs-in-sync.md`'s `nova_channel` row does route to `architecture.md` plus `commands.md` and does name no second wire reference; `docs/agent-bench.md` defers the wire to `nova_probe::capabilities::snapshot` and claims nothing.
- The `9e69ac196` gating, statically: every newly gated const's only use sites sit inside `#[cfg(feature = "debug")]` items, and `stress_hull_collapse`'s `LOOP_AFTERMATH_SECS` is read by the ungated `close_the_loop`, so ungating it is the right direction.
- Meters. Every figure in the range's prose is in meters, ticks or seconds; the newly gated consts keep their `Meters` / `Meters3` / `MetersPerSecond` types.
- Ran the newly written tests: `cargo test -p nova_bench --lib` filtered to `manual`, `gesture`, `observation` - 24 pass, including `the_manual_states_the_aim_tick_cap_the_parser_enforces` and the extended lost-hull ordnance case.
- Changelog. Nothing in the range changes a shipped format; the three user-visible items it touches (schema 2, the lowered-context stop, the null dwell) already have their v0.13.0 entries, and the rest are same-cycle fixes that AGENTS.md says get none.

Not checked:

- Did not build. No `cargo check --examples` under default features, so the `9e69ac196` gating is verified by reading cfg boundaries, not by the compiler; the default-features `-D warnings` job is the real proof.
- Did not run a rendered example, probe or capture - no measurement slot - so the frame-cost and readback claims in the rewritten `screenshot_railgun` and `system_torpedo_launch` comments are unverified, as is the actual pixel content of the re-shot PNGs.
- Did not run `tests/env_contract.rs` (root package, links the workspace), the workspace suite, or Clippy.
- Did not review `examples/playable/wfc_arena.rs`'s strike-counter behavior change or `62043f3b5`'s `try_despawn` for correctness. `62043f3b5` ships no regression test for the double-despawn it fixes; that is the correctness lane's call.
- Did not re-derive `web/src/create/` or the rest of `web/src/wiki/` against this range; I only re-derived `targeting-radar.md`, because the bench page put a figure in play.
