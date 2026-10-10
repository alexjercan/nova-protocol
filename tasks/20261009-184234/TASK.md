# Keep the cargo intake door shut for a canister that cannot fit

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.16.0,gameplay,ui

## User facts
- A full hold must not make the intake door open as if a canister can be picked up. This task does not change physical pickup eligibility.
- Implementation was approved and committed locally as `78fb1a192`; no push was authorized.

## Agent findings
- Before `78fb1a192`, door target was set by any untaken canister in detection range, independent of cargo room (`crates/nova_ship/src/sections/cargo_intake_section.rs:555-569`). Actual pickup requires trigger contact AND `free_g >= canister.total_mass_g()` (`:574-595`). `CargoIntakeDoorMoved` may play an opening cue even when room is insufficient. The world-space pickup sight selects a pair without inspecting `ready` (`crates/nova_hud/src/pickup_sight.rs:175-227`). This was source-backed at the pre-fix revision; the later rendered proof is below.
- An ejection queue can also open the door (`cargo_intake_section.rs:553-558,616-639`); preserve this independent purpose. Research: `tasks/20261007-090722/RESEARCH.md` candidate 2.

## Delivery gate
- Reproduce a full hold with an in-range canister and verify door pose, cue and unchanged inventory. Design door intent as ejection pending OR an eligible in-range canister with room; retain normal approach opening before contact and deterministic behavior with multiple different-mass canisters. Decide if full-hold status needs a separate explicit cue (not required by this task) before editing UI/audio. Do not conflate range detection with physical trigger contact.
- What dies: false-positive opening for an unfit canister alone. What may break: ejection, door animation/audio, pair choice and pickup sight. Invalid authoring still fails at its owning validation boundary.

## Verification
- Focused ECS assertions cover in-range fit, in-range too heavy, no-contact approach, mixed-mass canisters, ejection with full hold, transition after cargo is freed, and actual trigger pickup. Inspect rendered door/cue before and after. Do not claim pickup from a door-open event alone.

## Proof and limits
- An oversized-canister regression failed before the fix; 14 focused state tests pass, including mixed-mass eligibility and queued full-hold ejection. The change is limited to intake door intent; pickup still requires trigger contact.
- Two isolated, cheat-marked fixture recordings used a stable free-look camera and passed the door judge and pair comparison. Both showed one ejected canister and unchanged item counts. With zero room, the door-close cue followed ejection by one tick; with 10,000 g free, it followed after 713 ticks near the 40 m range edge. Both captures showed upright open leaves and later folded leaves. Recorded sidecar cues were checked, but no one listened to the audio. This covers ejection then a nearby canister, not a flown approach or a New Game pickup.
- An earlier combined free-look/aim fixture turned the hull and failed the 300-tick control threshold. Splitting the press from aim kept turn rate at zero in both final runs. Whether same-tick aim incorrectly turns the ship is a separate, uninvestigated input issue.
- Generated fixture scripts, bench logs, images, and recordings were removed from this task folder; the task record retains only the summarized results and limits.

## Done when
- Reproduction, approved door rule, focused state tests and rendered feedback proof are recorded. This gate is met with the above limits; no push was authorized.
