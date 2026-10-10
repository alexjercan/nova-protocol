# Make refused mining pulses audible without sound spam

- STATUS: CLOSED
- PRIORITY: 82
- TAGS: v0.16.0,gameplay,audio

## User facts
- A rejected mining attempt should play a reject/not-found sound when it cannot mine, including distance-related failure. Do not change successful mining or ore yield.
- This is a task specification, not approval to change code or ship audio.

## Agent findings
- A deployed Mining emitter produces `MiningPulse { outcome: Err(MiningRefusalType) }`; reasons are NoLock, NotAsteroid, Barren, OutOfReach (emitter face to rock collider), and OffTarget (beam ray misses within reach) (`crates/nova_scenario/src/mining.rs:161-176,504-532`). The refusal is logged, while success beam flash and pulse sound return early for errors (`mining.rs:488-494,799-801,838-840`). Module docs explicitly make rejection silent (`mining.rs:16-23`): feedback changes intentional policy.
- Research: `tasks/20261007-090722/RESEARCH.md` candidate 2. No rendered New Game failure or sound proof yet.

## Delivery gate
- Reproduce at least no lock, out of reach, off target, barren and in-range accepted pulses on the real Mining section. Decide whether all five reasons share one sound or need distinct cues, when cues trigger while a key is held across repeated clocks, volume/position and cooldown/dedup rule. Clarify whether temporary field seed/remesh `Ok(0)` must remain quiet or use successful pulse audio. Name actual SFX asset and ownership before adding content or code; ensure logs remain useful.
- Only the refusal feedback changes. No implicit aim assist, expanded reach, ore yield, or section-key change. Playback must not flood the mix or allocate unbounded events while held.

## Verification
- Focused `MiningPulse` refused/accepted examples inspect actual audio event(s), held-key rate cap and preserved successful sound/beam. A rendered/audio capture checks cue audibility and clarity; recorded mining success requires carved-rock/canister evidence, not input acceptance alone.

## Done when
- Reproduction, approved cue and spam policy, focused proof and audible inspection exist with limitations recorded. No implementation is authorized by this task record alone.
