# Decision: campaign metadata as one Option<ScenarioCampaign>

- STATUS: ACCEPTED
- DATE: 20260723
- TASK: 20260723-095849
- UMBRELLA: 20260723-093914

## Context

The campaign-grouped picker (umbrella 20260723-093914) needs a scenario to
declare which campaign it belongs to and at what position, so the picker can
group + order scenarios instead of alphabetising display names. This is a new
field (or fields) on `ScenarioConfig`, which is serde-serialized to the mod RON
format and reconstructed as literals in ~50 builders/tests/examples.

## Options

1. Two loose optional fields: `campaign: Option<String>` +
   `campaign_order: Option<u32>`.
2. One optional struct: `campaign: Option<ScenarioCampaign { name, order }>`.

## Decision

Option 2 - a single `campaign: Option<ScenarioCampaign>` where
`ScenarioCampaign { name: String, order: u32 }`.

## Why

- ATOMIC membership. With two loose Options the type permits the meaningless
  states "order set, campaign unset" and "campaign set, order unset". The
  consumer (picker sort/label in task C) would have to decide what those mean.
  The nested struct makes membership a single yes/no: either a scenario is in a
  campaign at a known position, or it is not.
- One serde-defaulted field, not two, to keep back-compat surface minimal. The
  field mirrors `thumbnail`'s exact treatment (`serde(default,
  skip_serializing_if = "Option::is_none")`), so pre-existing scenarios and
  mods parse unchanged and clean scenarios omit the key.
- Author-facing syntax is a single, teachable literal:
  `campaign: Some((name: "Nova Protocol", order: 1))`. Documented on the field
  and pinned by a hand-written-RON parse test (not just a self round-trip, per
  the `roundtrip-hides-shared-bug` lesson).

## Consequences

- Adding a non-Default field breaks exhaustive `ScenarioConfig { .. }` literals
  that do not use `..Default::default()`. `cargo check --workspace
  --all-targets` found 6 (1 in nova_scenario/world.rs, 5 in nova_assets); all
  fixed with `campaign: None` in this task. Everything else uses
  `..Default::default()` and is untouched.
- `ScenarioCampaign` derives `PartialEq, Eq` so tests and the picker can
  compare membership directly; it is exported through the loader prelude
  alongside `ScenarioConfig`.
- Future richer campaign UI (the deferred follow-up 20260723-095951) may
  supersede or wrap this with a first-class Campaign content entity; if so this
  DECISION gets a SUPERSEDED-by link then.
