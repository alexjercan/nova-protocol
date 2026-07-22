# ch3 stealth rework: neutral-until-provoked channel Magpies (proximity + paint), clean-run comms payoff

- STATUS: OPEN
- PRIORITY: 58
- TAGS: v0.8.0, content, scenario, playtest

## Story

Playtest VERDICT (owner, 2026-07-23): ch3 "The Quiet Channel" says "go dark" in
the comms but the channel Magpies always spawn hostile and engage - it is not a
stealth mission. Make it one: the two channel Magpies patrol NEUTRAL and only
wake (become hostile) if the player PROVOKES them - by red-locking one (paint)
OR by entering their detection radius (fly too close / into their lane). Thread
the channel slow in the safe lane without painting them and you slip past - "go
dark" delivered. Reaching Vesh's yard undetected earns a Vesh comms payoff and
the Magpies stay asleep; waking either one drops into the current fight.

Owner design (2026-07-23): provocation = proximity + paint; payoff = comms line
+ they stay asleep (no ch4 coupling, no scoring). Depends on the SetAllegiance
action (20260723-000253).

## Steps

- [ ] Spawn `channel_magpie_1/2` with `allegiance: Some(Neutral)` and a patrol
      route down the channel (AI patrol waypoints), NOT engage_delay-hostile.
      Keep them on the lane the player must thread.
- [ ] Detection: create `OnEnter` proximity areas (CreateScenarioArea) around
      each Magpie's lane / a "hot zone"; entering one fires
      `SetAllegiance((id: "channel_magpie_N", allegiance: Enemy))` + a "they've
      seen us" Vesh line + sets a `spotted` variable. Tune the radius so the
      safe (pinch) lane stays outside it - careful slow threading avoids it.
- [ ] Paint provocation: `OnCombatLock` on each Magpie fires the same
      SetAllegiance->Enemy + spotted flip (painting = about to shoot = wake).
- [ ] Clean-run payoff: reaching YARD (gate==4) with `spotted==0` posts a Vesh
      "nothing on their scopes - nice and quiet" line before Victory; the
      Magpies never engage. `spotted>0` = the existing fight; Victory/retry and
      the ch4 chain unchanged either way.
- [ ] Keep the fighting-is-optional contract and the debris-pinch beat (the
      pinch IS the slow safe lane that keeps you outside detection - make them
      reinforce each other).
- [ ] Update `crates/nova_assets/tests/ledger_ch3_channel.rs`: pin the Neutral
      spawn + patrol, both wake paths (proximity OnEnter and OnCombatLock ->
      SetAllegiance Enemy), the `spotted==0` clean-run payoff line, and that
      YARD->Victory->ledger_ch4_the_buyer holds on BOTH paths. Clock-pump as
      needed.
- [ ] Bump the bundle 1.6.0 -> 1.7.0 (content rework, minor), update the mod
      CHANGELOG + README (ch3 is now a real stealth run), regenerate the portal
      catalog locally (do NOT publish), sync any wiki ledger-version line
      (keep-docs-in-sync, whole-tree grep).

## Definition of Done

- ch3 Magpies spawn Neutral and only turn hostile on proximity OR paint; a
  careful undetected run reaches YARD with no fight and a payoff line. (test:
  ledger_ch3_channel pins Neutral spawn, both wake paths, and the clean-run
  payoff; manual: owner replays and confirms stealth is real.)
- `content lint --target the-ledger` clean; the ledger suite green with
  deliberate test updates. (cmd.)
- Bundle 1.7.0, CHANGELOG/README/wiki synced, catalog regenerated locally.
  (cmd; manual: owner publishes.)

## Notes

Records the owner VERDICT on the shipped ch3 (20260722-214105): "go dark" did
not read as stealth in play. Depends on 20260723-000253 (SetAllegiance). Owner
Finish checks: replay the stealth run (slip past) AND a provoked run (fight),
then publish.
