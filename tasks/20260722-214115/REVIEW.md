# Review - 20260722-214115 Ledger per-chapter look

## Round 1

Adversarial out-of-context review. Verified the diff, the accent placement, the
rig fix, and ran lint/fmt/tests. All findings below are LOW (nits); nothing
blocks.

### Engine untouched (verified)

`git diff master...HEAD -- crates/nova_scenario/` is EMPTY. The shipped
`SetSkybox -> PendingSkyboxSwap` mechanism (actions.rs:314-324) is reused
as-is; no engine change. actions.rs:324 does `world.resource::<AssetServer>()`
(a hard unwrap that panics if the resource is absent), which is exactly what the
rig fix accommodates. GOOD.

### Landed logic intact (verified)

Full RON diff shows only: ch1 +3 lines (comment + SetSkybox), ch3 +5/-1 (the
`cubemap:` value flip alt->cubemap.png plus comment + SetSkybox), ch4 +3
(comment + SetSkybox). No spawn / objective / filter / gate / Outcome / kills
counter touched. ch2 and ch2b are unchanged (grep: 0 SetSkybox each; ch2b still
opens on cubemap_alt). The optional ch2/ch2b victory-breather swaps and the ch4
burn-path swap are correctly SKIPPED and the skip is motivated in NOTES. GOOD.

### Accents sit in the motivating handler, one-shot, never OnStart (verified)

- ch1 (ledger_ch1.content.ron:1748): inside the OnUpdate handler guarded
  `act==2 && ping_said==0 && elapsed>ping_gate`; the same actions list sets
  `ping_said=1` (line 1734) before the swap. Fires exactly once.
- ch3 (ledger_ch3.content.ron:738): inside the OnUpdate handler guarded
  `pinch_warn_said==0 && pinch_gate>0 && elapsed>pinch_gate`; same list sets
  `pinch_warn_said=1` (line 728). Fires once.
- ch4 (ledger_ch4.content.ron:442): inside the OnEnter(handoff_berth) sell
  handler guarded `act==1`; same list sets `choice=1` then `act=2` (lines
  428-434). Fires once, sell path only. The burn branch stays on the calm
  start by design.

None of the three sits in an OnStart handler; the starting look is carried by
`cubemap:`. Each reuses an EXISTING guard, so re-entry cannot thrash. GOOD.

### Rig fix is faithful, not a mask (verified)

ledger_ch3_channel.rs and ledger_ch4_ending.rs each add exactly
`AssetPlugin::default()` + `init_asset::<Image>()` to `slice_app()`. Diffing the
two test files, the ONLY non-comment changes are those two plugin lines - no
handler assertion, no filter, no expected-state logic changed. The reason is
sound: production runs with a real AssetServer, the ch3/ch4 rigs now drive
handlers that carry a live SetSkybox whose command reads that resource, so
giving the MinimalPlugins rig the same plumbing makes it production-faithful
rather than papering over a bug. With no scenario camera present the swap
no-ops after kicking the load, which is all these behavior rigs need. The
ch2_encounter rig got NO change (its swaps were skipped) - consistent. This is
the correct fix, not a masked panic. GOOD.

### New test ledger_skybox.rs is meaningful, not a false-green (verified)

`starting_cubemaps_are_the_deliberate_palette` pins all five starts by exact
path; a reshuffle fails. The three accent tests use `handler_with_line` (which
also asserts exactly-one handler carries that StoryMessage text) to locate the
beat, then `handler_swaps_to` asserts the SetSkybox target IN THAT handler.
Dropping an accent, moving it to OnStart, moving it to a different handler, or
retargeting it would fail. `every_swap_targets_a_base_cubemap` forbids new-art
paths; `no_chapter_swaps_at_on_start` forbids frame-0 swaps (EventConfig::OnStart
confirmed a real variant, events.rs:19, so the filter is live). Not a shallow
presence check. GOOD.

### Checks run

- `content lint --target the-ledger`: 0 errors, 0 warnings, 5 balance-audited,
  1 acked (the ch4 auditor close-spawn, pre-existing). Matches expectation.
- `cargo fmt --check`: clean (exit 0).
- `cargo test -p nova_assets --test ledger_skybox --test ledger_ch2_encounter
  --test ledger_ch3_channel --test ledger_ch4_ending`: 6 / 12 / 9 / 10, all
  passed, 0 failed. Matches expectation.

### LOW-1 - two-cubemap ceiling is real but accepted (no change)

`ledger_ch1.content.ron`, ch3, ch4. With only two base cubemaps, "distinct per
chapter" is inherently coarse: ch2/ch2b/ch4-start all share cubemap.png- or
alt-level looks and ch3->ch4 both start calm. The owner explicitly accepted this
minimal sourcing and deferred a richer self:// art pass; the deliberate
assignment + three mid-run accents are a reasonable delivery of the narrowed
DoD. Visual "does it render at the beat" confirmation is correctly deferred to
the owner's Finish replay - the tests here are data-level wired-proofs, not
renderer checks, and NOTES says so plainly. No action.

### LOW-2 - accent test coverage is asymmetric (optional)

`ledger_skybox.rs` pins the THREE non-optional accents but does not assert that
ch2/ch2b carry NO accent (the skip). A future re-add of a ch2 swap would pass
all tests silently. Given the swap was skipped precisely to protect the
ch2_encounter MinimalPlugins rig, a `ch2/ch2b carry no SetSkybox` assertion
would guard that decision. Optional hardening, not required for approval.

## Verdict

- VERDICT: APPROVE