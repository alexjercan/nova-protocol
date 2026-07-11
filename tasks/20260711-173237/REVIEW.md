# Review: CTRL press alone fires the target cycle

- TASK: 20260711-173237
- BRANCH: fix/ctrl-alone-target-cycle

## Round 1

- VERDICT: APPROVE

Verified independently: the fail-first A/B was actually run (fix committed
first, then master's rig wiring checked out over it) and the new test
detects the landed bug with recorded numbers ("CTRL alone must not pin:
left Some(4.001216), right None") - notably it also exposed that the buggy
wiring fired BOTH cycle directions on a bare CTRL press. With the fix
restored: ctrl_routes test green, input:: 128/128 green, cargo check
--workspace green, fmt clean.

Design check: moving the modal routing from input conditions into the
observers is the right call, not a workaround - both condition-DSL
attempts fail for verified reasons (Chord ignores the binding value;
the combiner + Start-on-Ongoing semantics leak the unmodified gesture),
and the dispatch keeps the gesture semantics in one readable place. The
replacement test asserts on the lock/pin/component resources at EVERY
gesture step with delivery guards (component actually stepped; modifier
actually Fired), fixing the coincidental-pass weakness of the original
event-counting test. Rig behavior otherwise unchanged: hint labels,
DPadUp, plain-scroll component cycling all covered.

One accepted residue, not a finding: `TargetCyclePrevInput` keeps an
empty bindings list as a seam for a future dedicated key; harmless and
documented in the rig comment.

No findings.
