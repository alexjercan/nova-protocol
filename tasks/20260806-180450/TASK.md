# Vendor bevy-common-systems

- PRIORITY: 0
- TAGS: v0.10.0
- ACTIVITY: UNDERSTANDING
- GATES: -
- RESOLUTION: -

## Problem

I don't want to depdend on `bevy-common-systems`.

## Context

Basically I feel like we sometimes make bad code decisions because BCS is
another crate and the thing is that __some__ features are not really generic
(see what happened with health for instance). Now sure, there are common things
like the math module or the camera module, but I still feel like we should
first of all vendor everything ourselves and once the game is "DONE" see what
parts of the game are "copy-pastable" to other games. That way we can create
a better `bevy-common-systems` from this crate; I think my idea was right to
split bcs from nova, but I did it too early and with too many dependencies that
are game specific; What we will do is what NOTES.md says: just copy paste bcs
in here, and you know just migrate it nicely using compiler assited refactoring.
