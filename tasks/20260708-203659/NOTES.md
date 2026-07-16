# Broadside - design record (task 20260708-203659)

## Story shape (user direction: "go ham", multiple actors, neutrals)

Three acts under the outcome frame, not a bare arena duel: contact (fly to a
NEUTRAL hauler's distress call through asteroid cover), escalation (a
two-corvette ambush - guns only), twist/climax (the gang's gunship: two
turrets, two torpedo tubes, an armored spine - the capital fight the
engine's PDC/torpedo/integrity systems were built for). Win = gunship dead
-> Victory overlay, nothing queued (end of the base story; Main Menu). Lose
= player dead in any act -> Defeat + lingering retry. The hauler is flavor
with stakes: neutral (nobody targets it), killable by stray blast damage,
and the story reacts with an objective line rather than a fail state.

The act machine is variables: `act` 0..3 gates every beat; the two corvette
kills set independent FLAGS (no arithmetic counter - a double OnDestroyed
cannot overshoot; count-gate-use-gt-not-eq by construction) and a
`OnUpdate`-gated one-shot escalates when both are up.

## Numbers vs the measured AI constants (ai.rs, verified in source)

- Engage range 800u, torpedo envelope [3 x blast_radius = 90u, 1000u],
  per-bay cadence 10s (first launch immediate), standoff orbit ~250u,
  acquisition 2000u.
- The gunship spawns ~720u from the hauler fight: engaged on arrival, tubes
  open through the whole approach - the screening beat is real, not
  authored fiction. Corvettes spawn ~150u out with a 420u leash anchored on
  the hauler patrol, so that fight stays in the derelict field.
- The plan's survey claimed a ~4s torpedo cadence; the source says 10s
  (AI_TORPEDO_COOLDOWN_SECS). Distances were sized against the source.

## New modding surface: authored allegiance

`SpaceshipConfig.allegiance: Option<Allegiance>` (serde-defaulted; strict
RON `allegiance: Some(Neutral)`). None keeps the controller-marker defaults
(Player/Enemy requirement components); Some overrides via a plain insert at
the spawn action - which wins regardless of command ordering, because
observer-queued commands apply before the queue's remaining commands
(ledger: verify-engine-guarantees-in-source) and a plain insert overwrites
a requirement default. Pinned through the production spawn path with a
default-Enemy delivery guard (actions.rs test).

Honesty note (review R1.6): the broadside hauler's untargetability does not
strictly need the field - with `controller: None` the root carries no
Allegiance at all, and `relation(None, ..)` already resolves Neutral. The
authored `Some(Neutral)` makes the intent explicit in the data; the surface
earns its keep for neutral ships WITH a controller (an AI freighter flying
a route would otherwise require-default to Enemy), which is the path the
delivery-guard test actually pins.

## Found by dogfooding: eager skybox install panics on non-preloaded skies

Example 19's first run crashed in bcs `skybox.rs` `.unwrap()`: the loader
inserted `SkyboxConfig` eagerly, which only ever worked because every prior
scenario used the PRELOADED `textures/cubemap.png`. Broadside's
`cubemap_alt2.png` - and any mod shipping its own sky - hit the known bcs
panic-on-unloaded-asset hazard. Fix: the loader now hands the camera a
`PendingSkyboxSwap` (the SetSkybox action's deferred applier, which handles
fresh installs - it falls back to the pending brightness when no SkyboxConfig
exists). Boundary pin: `scenario_load_defers_the_skybox_install`. Cost: the
sky pops in a few frames after load for non-preloaded cubemaps; was a crash.

## Example 19 (CI smoke)

One run drives BOTH outcome paths through the real app: picker -> Play ->
die in act 0 -> Defeat overlay -> click Retry -> assert the reload came up
clean -> teleport to the hauler (writes avian `Position`, not just
Transform - the physics clock owns the body) -> the avian area bridge
springs the ambush for real -> `HealthApplyDamage` overkill on roots drives
the production kill chain (propagation, integrity explode, OnDestroyed
bridge; only the bullets are skipped, pinned elsewhere) -> Victory overlay +
nothing queued -> capture `broadside_victory.png`. A completion guard panics
if the autopilot lifetime ends with the script mid-stage, so a stalled walk
cannot log "cycle complete" and pass.

## Deliberately deferred

- Feel/balance tuning of the gunship fight (magazine sizes, hull counts,
  torpedo pressure) needs HANDS, not a harness: the distances are authored
  against measured constants and the systems are exercised, but "fair and
  first-try-losable" is a user-playtest verdict. Per the v0.7.0 plan's
  policy, playtest findings land as `bug`/`balance` tasks at release
  priority.
- Real thumbnail art: task 20260715-220011 (placeholder `banner.png` ships).
- Ship prototypes (20260714-134115): every broadside ship is inline; no two
  scenarios share a hull yet, so the recorded deferral stands.
