# AI evasion under fire: threat model + jink maneuvers

- STATUS: OPEN
- PRIORITY: 68
- TAGS: v0.4.0,ai,spike,handling


Spike: docs/spikes/20260709-225508-ai-combat-behaviors.md (wave 3)

Goal: the AI reacts to being shot. A cheap threat model (received
HealthApplyDamage recently; a hostile within range is aiming at me) drives
Engage -> Evade: timed jink maneuvers (lateral thrust bursts, heading changes
off the pursuit vector) that decay back to Engage. Start with the cheap
signals; true incoming-projectile proximity detection is a follow-up if
evasion feels blind.

Depends on: 20260709-225726 (skeleton), 20260709-225729 (engagement flight,
for sane re-entry into Engage).

Note (20260710, from 225727): this task also owns wiring damage
attribution (populating HealthApplyDamage.source from projectile owners -
nothing sets it today) and, once attributed, adding recently-damaged-me
threat scoring to pick_ai_target (deferred from 225727).

Note (20260710, planning): bcs (rev 4c58835) already populates
HealthApplyDamage.source with the HITTING COLLIDER for both impact and
blast damage, so "attribution" here means resolving that collider to the
firing ship root through ProjectileOwner: direct for turret bullets (root
== collider), an ancestor walk for torpedo warhead sections, and copying
ProjectileOwner onto the detonation blast entity (which carries none
today).

## Steps

- [x] torpedo_section: torpedo_detonate_system copies the torpedo root's
      ProjectileOwner onto the spawned blast entity, so blast damage
      resolves to the shooter like contact damage does.
- [x] input/ai.rs: `AIThreat` component (required by AISpaceshipMarker):
      recent-hostile-damage memory (timer) + the attacker's ship root.
      Written by a HealthApplyDamage observer: when the propagating event
      reaches an AI root, resolve source -> ProjectileOwner (on the
      collider or an ancestor), record only when the relation is Hostile
      (a self-blast must not spook the ship).
- [x] input/ai.rs: `AIEvade` component (required): evade duration timer,
      re-entry cooldown timer (starts elapsed), jink-leg timer + leg
      counter.
- [x] Threat model in update_behavior_state: threatened = recently
      damaged OR (current target inside AI_THREAT_AIM_RANGE with its nose
      within AI_THREAT_AIM_COS of the bearing to me). Tick the
      threat/evade clocks here.
- [x] Transitions (next_behavior_state, kept pure): Engage + threatened +
      cooldown elapsed -> Evade; Evade -> Engage when the duration
      expires (re-arming the cooldown); passive states + recently damaged
      + target acquired -> Engage (getting shot interrupts a patrol even
      beyond detection range). Everything else as today.
- [x] Jink maneuvers: pure `ai_evade_direction(to_target, leg)` - box
      weave off the pursuit vector (alternating lateral quadrants with a
      small alternating along-LOS bias); leg advances on the jink timer.
      Rotation + thrust systems use it in Evade instead of the standoff
      envelope, with a looser AI_EVADE_THRUST_ALIGNMENT gate so lateral
      bursts fire mid-swing.
- [x] pick_ai_target: recently-damaged-me scoring - the remembered
      attacker's distance is discounted (AI_THREAT_ATTACKER_DISCOUNT) so
      whoever is shooting me steals the pick from comparably distant
      hostiles (deferred from 225727).
- [x] Tests: attribution (bullet owner direct, torpedo warhead ancestor
      walk, blast owner copy, self-blast not recorded); pure transition
      table incl. cooldown/expiry; jink direction pattern (off-LOS,
      alternating, unit length, degenerate LOS); picker attacker bias;
      pipeline test Engage -> damage -> Evade -> decay -> Engage.
- [x] Verify: cargo fmt + check + the new/touched test modules
      (per-module cargo test), full suite left to CI.
