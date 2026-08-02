# Promote the self-ending completion guard into bevy-common-systems

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog
- KIND: TASK
- FLOW STEP: DROPPED
- PLAN STATUS: DRAFT

## Story

Four examples now carry a verbatim copy of the same completion guard -
`broadside`, `lifeline`, `menu_scenarios` and (as of task 20260729-222131)
`screenshot_nova_os`:

```rust
fn guard_script_completion(mut exits: MessageReader<AppExit>, script: Option<Res<...>>) {
    let Some(script) = script else { return };
    if exits.read().next().is_some() && !script.done {
        panic!("<example>: run ended with the script stalled in stage {}", script.stage);
    }
}
```

It is the other half of `AutopilotPlugin::self_completing()`: the plugin turns
a runway EXPIRY into an abort, and the guard turns a premature `AppExit` from
anywhere else into one. Only the plugin half lives in
`bevy-common-systems`; the guard half is copy-paste, so each new self-ending
example re-implements it (or, as 20260729-222131 showed, ships without it and
passes silently until someone notices).

`tasks/20260719-235305/SPIKE.md:57` already proposed the promotion.

Two wording problems the copies share, which a single owner would settle once
(review R1.5/R1.6 of 20260729-222131):

- the panic reports the stage index AFTER the increment, so it names a stage
  that never started ("stalled in stage 13" for a 12-stage script);
- `main()`'s comment says a runway expiry "is an error exit naming it", but the
  autopilot writes `AppExit::error()` in `PreUpdate` and the `Last` guard then
  panics, so the panic is the ending actually observed - the two comments
  describe mutually exclusive endings.

## Notes

- The guard runs in `Last`, unordered against bcs's private `completion_watch`
  (also `Last`). Bevy stops after the update in which `AppExit` is written, so
  an exit written by a later-scheduled `Last` system goes unobserved. A
  protocol-level version could order itself explicitly and close that gap.
- Source: review R1.7 of task 20260729-222131.


## Dropped

- REASON: old
