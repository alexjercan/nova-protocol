# nova_channel drivers

The design page (`../nova-channel.html`) made executable. `mock_game.py` is
the wire contract's SCHEMA REFERENCE -- the five input lanes, the reserved
refusals, the registry with contexts and the live set, the step clock, the
freeze model, the pointer census -- over a toy world. The crate exists now
(`crates/nova_channel`); the four drivers are its acceptance tests against
the real binary.

## Run

Build with `cargo build --features debug`, then from this directory:

```
CMD="target/debug/nova-protocol --norender --scenario shakedown_run --channel step"
python3 drive_pointer.py --cmd "$CMD"   # click Resume by name, then by raw pixels
python3 drive_novaos.py --cmd "$CMD"    # Tab, type at the prompt, run the map app
```

The flight pair needs the acceptance world (`acceptance.content.ron`: one
armed corvette, one hostile raider on the burn line) instead of a shipped
scenario -- swap `--scenario shakedown_run` for
`--scenario-file <absolute path to acceptance.content.ron>`:

```
python3 drive_flight.py --cmd "$CMD"    # fly, lock, fire -- by name and by section id
python3 agent_loop.py --cmd "$CMD"      # observe -> decide -> act on the step clock
```

Isolate `NOVA_CONFIG_ROOT` to a scratch directory so a run never touches the
real config. Without `--cmd`, the pointer and NOVA OS drivers fall back to
the mock; the flight pair refuses (their checks read the acceptance world).

Each narrates a transcript and exits non-zero on the first failed check.
No dependencies beyond the standard library.

`channel.py` is the shared client. It owns the tick counter, stamps every
payload, and wraps the idioms a driver needs (a hold is start / clock /
stop; a click is move / press / release on separate ticks).

## What is deliberately honest, and what is a toy

Honest, because drivers must design around it: the radar hold has a latch
threshold and a tap window; a click activates on release-over inside the
same target; a named input whose context is not live is refused, not
pressed blind; `action` and `command` lines are refused with the follow-up
task id; frozen screens keep taking frames while `elapsed` holds still.

Toy: the world. One corvette, one raider, straight-line kinematics, a
three-button pause menu, a prompt that knows `map`, `ship` and `exit`.

## The mock, retired

All four drivers pass against the real binary, so the mock's acceptance
duty is done: it stays as the executable schema reference for the wire --
the quickest way to see a whole session's lines and answers without
building the game.
