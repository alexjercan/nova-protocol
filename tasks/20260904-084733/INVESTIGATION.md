# Investigation: arrival standoff and target radius

- TASK: 20260904-084733
- BRANCH: master
- BASE: `8d558cd4`
- METHOD: full trace of every read, write, authored field, test and doc claim
  for the four standoff dials and the two radius sources. Every load-bearing
  claim below was read in the source; the three defects in section 3 were
  re-read directly before the recommendation was written.

## 1. What exists today

Four dials that all mean "how far short do I stop", and two that answer "how
big is the thing I am stopping short of".

| # | control | scope | where |
|-|-|-|-|
| 1 | `FlightSettings::arrival_standoff` | global default, `50.0` engine = 500 m | `crates/nova_ship/src/flight/state.rs:274`, default `:367` |
| 2 | `FlightArrivalStandoff` | per-ship component | `crates/nova_ship/src/flight/state.rs:49` |
| 3 | `AIControllerConfig::arrival_standoff` | authored, AI ships only | `crates/nova_scenario/src/objects/spaceship.rs:201` |
| 4 | `MoveShipToActionConfig::arrival_standoff` | authored, per order | `crates/nova_scenario/src/actions/ship.rs:213` |
| 5 | `BodyRadius` | derived target size, asteroids only | `crates/nova_ship/src/flight/state.rs:14` |
| 6 | `GravityWell::body_radius` | well size, asteroids and anchors | `crates/nova_gameplay/src/gravity.rs:96` |

Dials 3 and 4 are not independent controls. Both are writers of dial 2, with
different guards, different clearing rules and different lint coverage. That is
the duplication the task suspected.

`arrival_standoff` is never written at runtime. It is not surfaced in any
settings menu or inspector panel, despite the doc at `state.rs:258` promising
"a future settings menu".

## 2. The precedence chain as implemented

```
effective margin   = FlightArrivalStandoff ?? FlightSettings::arrival_standoff
  FlightArrivalStandoff, last writer wins:
    MoveShipTo Some(x)              order.rs:591   ungated, x == 0 installs
    suspended restore on retire     order.rs:713   cancel/interrupt only
    AIControllerConfig Some(x > 0)  spaceship.rs:670   x <= 0 silently dropped
    absent                          -> global 500 m

target_radius (entity GOTO) = max(BodyRadius, well.body_radius)   autopilot.rs:471
target_radius (GotoPos)     = 0                                    autopilot.rs:492

final CENTRE distance       = effective margin + target_radius     autopilot.rs:301
published distance          = centre - target_radius               autopilot.rs:321
"arrived"                   = published <= effective margin        autopilot.rs:485

ORBIT park after arrival    = clamp(max(body + GLOBAL margin, r),
                                    1.5 * (body + 1.0),
                                    0.9 * 0.85 * soi)              autopilot.rs:735
```

The mover's own size appears nowhere. `arrival_desired` treats the ship as a
point at its avian `Position` throughout.

## 3. Defects, each re-read in the source

**D1 - the ORBIT park ignores the per-ship margin.** `autopilot.rs:735` reads
`settings.arrival_standoff`, not the resolved value. A ship authored with
`arrival_standoff: Some(Meters(100.0))` arrives at `body + 10 u`, and the
handoff then plans a ring at `max(body + 50, body + 10) = body + 50` and burns
400 m OUTWARD to a ring nobody asked for. `state.rs:53` documents this split as
deliberate; the effect is that the arrival rule and the park rule disagree
about the same ship's stated intent.

**D2 - a completed order never gives the override back.** `complete()`
(`order.rs:660`) does not call `retire_ship_order_execution`, which only
cancel and interrupt reach. `web/src/create/actions.md:653` promises the
override is "taken back off when the order retires". It is not: after First
Shift's `warship_approach` completes, the warship keeps
`FlightArrivalStandoff(20.0)` and a stale `SuspendedArrivalStandoff` until some
later order happens to clear it.

**D3 - `Some(0.0)` means two opposite things.** On a `MoveShipTo` it passes
lint (`lint/scenario.rs:744` allows non-negative) and parks the hull on the
mark. On an `AIControllerConfig` it is silently dropped by the
`> Meters::ZERO` gate (`spaceship.rs:671`) and the ship flies the 500 m global.
Neither path warns. `AIControllerConfig::arrival_standoff` has no lint at all.

**D4 - the setting does not cross the seam in the type.** `state.rs:367` is a
bare `50.0`, while two fields below it in the same `Default` impl do use
`MetersPerSecond(100.0).to_engine()` and `MetersPerSecondSquared(...)`. The
unit lives in a prose comment instead of the value. `web/src/widgets.ts:101`
and `nova_hud/src/holo_instruments.rs:279` each re-declare `50` by hand with a
citation comment, so both go stale silently.

**D5 - two safe-distance models that cross.** Arrival gives `body + margin`.
The ORBIT band floor gives `1.5 * (body + surface_margin)`
(`guidance.rs:229`). With the 50 u margin those cross at `body = 97 u`
(970 m): below it GOTO parks outside the ring ORBIT would fly, above it GOTO
parks inside a ring ORBIT calls unsafe and the handoff shoves the ship out.
The campaign's own inspection planetoid straddles that crossover across its
seed range (`BodyRadius` 70-120 u for an authored 200 m).

## 4. What each target category gets today

Default 500 m margin, no override:

| target | target_radius | final centre distance |
|-|-|-|
| `GotoPos` mark | 0 | 500 m |
| Beacon | 0 - `BeaconRadius` is never consulted | 500 m from centre |
| Ship | 0 - wells exclude `SpaceshipRootMarker` (`autopilot.rs:129`) | 500 m origin to origin, neither hull's ~28 m arm counted |
| Asteroid, no gravity | `nominal x [3.5, 6.0]` | a 30 m rock: 155-230 m |
| Asteroid with gravity | same | inspection planetoid: 1200-1700 m |
| Gravity well then ORBIT | | ring clamped into `[1.5(body+1), 0.765 soi]`, which may push outward - see D5 |
| Authored anchor | authored `body_radius` | authored + 500 m |

So the two categories the task names first - beacons and ships - are exactly
the two that get no geometry at all.

## 5. Units

The authored seams are correct: `spaceship.rs:674`, `actions/ship.rs:241`,
`anchor.rs:70` and `asteroid.rs:199` all call `to_engine()`. The engine-side
constants are the problem: `arrival_standoff = 50.0`, `AI_WAYPOINT_SLACK =
25.0` (`input/ai/passive.rs:22`), `AI_STANDOFF_RANGE = 100.0`
(`input/ai/maneuver.rs:39`) and the whole of `GravitySettings`
(`gravity.rs:207`) are bare world-unit floats whose meaning lives in prose.

## 6. "Standoff" names seven different things

Arrival rest distance; the per-ship override of it; a cinematic's per-order
override of that; the AI's preferred **engagement** range
(`AI_STANDOFF_RANGE`, `input/ai/maneuver.rs:39` - a completely unrelated
dial); the AI's "standoff orbit" combat behaviour; the HUD's word for the
trajectory ribbon terminus; and a generic distance example in the player
glossary. The navigation meaning and the gunnery meaning share a word and
nothing else.

## 7. Recommendation - one model, fewer dials

```
final centre distance = target geometric radius
                      + mover geometric radius
                      + navigation margin
```

with the margin resolved ONCE and read everywhere.

**R1. One resolved margin.** Extract `FlightArrivalStandoff ?? settings` into a
single helper and use it at all three sites - arrival (`autopilot.rs:149`),
ORBIT park (`autopilot.rs:735`) and AI patrol (`passive.rs:211`). Fixes D1.

**R2. Retire on completion.** `complete()` retires the order's overrides the
way cancel does, so an override cannot outlive its order. Fixes D2 and makes
`actions.md:653` true.

**R3. `None` inherits, `Some(x)` is used - in both authored paths.** Drop the
`> Meters::ZERO` gate at `spaceship.rs:671` and add the missing lint for
`AIControllerConfig::arrival_standoff` beside the one `MoveShipTo` already has.
Fixes D3. This is also the answer to "how is close arrival authored for a
navigation beacon": `Some(Meters(0.0))`, which then means the hull's outer face
on the mark rather than its origin on the mark.

**R4. Type the constant.** `Meters(500.0).to_engine()`, matching its
neighbours. Fixes D4.

**R5. Give every target category its geometry.** Beacons publish their radius;
ships publish their hull radius. The resolve stays "the largest size the target
publishes", so the rule does not grow a per-kind branch.

**R6. Count the mover.** `structural_arm` (`physics/attitude.rs:144`) already
measures a hull's outer face from its centre of mass, shrinks as sections die
and is already used at an engine boundary by the attitude envelope. Today it is
recomputed into a `Local<EntityHashMap>` inside the controller system
(`controller_section.rs:430`) and is not queryable. Publish it as a component
and both the envelope and the arrival read one value.

**R7. One safe-distance model.** For a well-bearing target, floor the arrival
at the ORBIT band's own floor, so GOTO arrives on a ring ORBIT already accepts
and the handoff never pushes the ship outward. This is what removes D5 rather
than documenting it.

### Decisions the task asked for

- **Who owns the primary override**: the MOVER, as one component with a global
  default. The order overrides it for the order's lifetime only. The target
  owns geometry, never a margin.
- **Target-specific clearance as a general component**: no. Adding a per-target
  clearance would be a third independent dial with no defensible precedence
  against the mover's margin. The target contributes size, which it already
  publishes; the margin stays one number.
- **Zero / close arrival**: `Some(Meters(0.0))`, legal and identical in both
  authored paths after R3. It means face-on-the-mark, not origin-on-the-mark,
  once R6 lands.
- **`GotoPos` with no geometry**: the point keeps `target_radius = 0`, which is
  honest - a mark has no size. After R6 the ship still stops
  `mover radius + margin` from it, so `Some(Meters(0.0))` is well defined and
  safe rather than parking a hull centre on a mark.
- **Unsafe authored values**: warn, do not clamp. Lint already rejects negative
  and non-finite; extend it to the AI field. A margin is NOT an obstacle
  guarantee and must not be sold as one - a `MoveShipTo` still flies a straight
  line through whatever is in the way (`first_shift/marks.rs:255` records the
  warship that died proving it). Route avoidance stays out of scope per the
  task.

## 8. Migration impact

- **Authored RON**: no field is removed or renamed, so no mod migration. The
  meaning of `Some(0.0)` changes on `AIControllerConfig` (from "ignored" to
  "zero"), which no shipped content uses.
- **Base content**: `weave_runner` (`main_menu/weave.rs:85`, `Some(Meters(100.0))`)
  and First Shift's two warship legs (`first_shift/mod.rs:387`,
  `Some(Meters(200.0))`) each gain their hull's arm. Both have kilometres of
  clearance at their marks; both need a regen and a live re-check.
- **Regenerate**: `assets/base/scenarios/first_shift.content.ron`,
  `assets/base/scenarios/menu_weave.content.ron`.
- **Tests to move**: `flight/tests/goto.rs` (all six, including the hard-coded
  `70 + 50 = 120` at `:285`), `flight/tests/telemetry.rs:74`,
  `flight/guidance.rs` unit tests, `flight/order.rs:914`,
  `objects/spaceship.rs:744`, `actions/ship.rs:1320`, `lint/scenario.rs:1758`,
  `input/ai/passive.rs:455`, `nova_hud/src/holo_instruments.rs:277`.
- **Docs to correct**: `web/src/create/actions.md:653` (the retire claim, false
  today), `web/src/create/objects.md:215` (silent on the dropped zero),
  `web/src/wiki/flight-autopilot.md:34` (silent that `GotoPos` gets no radius),
  `CHANGELOG.md:756` (stale units - "default 25 / 50" are now meters fields).
- **Hand-copied constants to link or re-cite**: `web/src/widgets.ts:101`,
  `nova_hud/src/holo_instruments.rs:279`.

## 9. Out of scope

Route obstacle avoidance, per the task. The margin is a parking rule, not a
path guarantee, and the recommendation above does not change that.
