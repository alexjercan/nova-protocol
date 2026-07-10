# AI torpedo usage from Engage: launch envelope + cooldown

- STATUS: IN_PROGRESS
- PRIORITY: 66
- TAGS: v0.4.0, ai, spike, torpedo

Spike: docs/spikes/20260709-225508-ai-combat-behaviors.md (wave 3)

Goal: AI ships fire their torpedo bays. From Engage, write
TorpedoSectionInput when inside a launch envelope: range band, rough
alignment, per-bay cooldown; reuse the commit-on-launch targeting the player
path already has (input/player.rs). Needs standoff flight to read well: a
point-blank launch self-hits (see 20260709-140559 on blast self-harm).

Depends on: 20260709-225727 (AITarget), 20260709-225729 (standoff flight).

Note (20260710, planning): the torpedo section already has a per-bay
FIRE-RATE cooldown (TorpedoSectionSpawnerFireState); the per-bay cooldown
here is the AI-side LAUNCH cadence on top of it, so the AI does not hold
the trigger and dump a torpedo every 1/fire_rate seconds. Launch detection
(for resetting that cadence) rides the projectile's TorpedoSectionPartOf
back to the bay, which goes pub for the purpose - truthful (only an actual
spawn resets it, an inactive section never burns the cooldown), unlike
predicting the launch from the spawner's fire state.

## Steps

- [x] torpedo_section: make TorpedoSectionPartOf and
      TorpedoSectionConfigHelper pub (exported via the section prelude) so
      the AI input module can read a bay's config for the envelope and
      attribute a fresh projectile to its bay. No behavior change.
- [x] input/ai.rs: `AITorpedoBay` component - per-bay launch cooldown
      (AI_TORPEDO_COOLDOWN_SECS, starts elapsed so the first launch in a
      fight is immediate). Lazily inserted on torpedo sections whose parent
      is an AI ship (spawn ordering makes an Add-observer racy; one frame
      of warmup is fine).
- [x] input/ai.rs: pure `ai_torpedo_envelope(to_target, forward,
      blast_radius) -> bool`: range band [blast_radius *
      AI_TORPEDO_MIN_RANGE_BLAST_FACTOR, AI_TORPEDO_MAX_RANGE] (the min
      keeps the detonation point clear of the shooter - blast self-harm,
      20260709-140559) + rough hull alignment (AI_TORPEDO_ALIGNMENT_COS,
      loose - PN guidance does the turning, the gate is for readability
      and not launching off an orbit tangent pointed away).
- [x] input/ai.rs: `update_torpedo_section_input` in the AI chain: for
      each AI ship, write TorpedoSectionInput on its bays = envelope
      && Engage-like state (Evade excluded: a jinking hull is no launch
      platform; Retreat inherits per its stub) && target is a SHIP (no
      torpedoes at torpedoes) && bay cooldown elapsed. Explicit false
      otherwise, so a firing bay stops. Tick bay cooldowns here.
- [x] input/ai.rs: AI-side `update_torpedo_target_input` (sibling of the
      player's commit-on-launch in input/player.rs): fresh torpedoes owned
      by an AI ship commit to that ship's current AITarget
      (TorpedoTargetChosen + TorpedoTargetEntity; dumb-fire when None),
      and the sourcing bay's cooldown resets. Runs before the input write
      in the chain so the post-launch frame sees the fresh cooldown.
- [x] Tests: pure envelope (in-band, below blast-derived min, beyond max,
      misaligned); integration through the real systems - lazy bay
      insert + input write with a hostile in envelope; explicit false on
      out-of-envelope / Evade / Idle / torpedo-target / cooldown; commit
      path (AI torpedo -> Chosen + TargetEntity + cooldown reset,
      dumb-fire without a target, player-owned torpedo untouched).
- [x] Verify: cargo fmt + check + the new/touched test modules
      (per-module cargo test), full suite left to CI.
