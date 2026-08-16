# Let two sections share a keybind in the editor

- STATUS: IN_PROGRESS
- PRIORITY: 63
- TAGS: v0.11.0,editor,input,bug

## The bug

Owner: "in editor mode when I try to assign the same keybind to two sections it
doesn't work, but sometimes I want to have the same keybind on multiple
sections".

Firing two turrets on one key, or running two thrusters together, is a
legitimate loadout choice. The editor refuses it.

## Where

`binding_conflict` (`crates/nova_editor/src/keybind.rs:177`) refuses a rebind for
TWO different reasons, and only one of them is right:

1. the source is reserved by the always-on flight rig
   (`flight_rig_reserved_sources`) - **keep this.** A key that also drives the
   ship would fire both, which is a real conflict rather than a choice.
2. another SECTION already holds the source (`:189-195`) - **this is the bug.**

`apply_section_rebind` (`:250`) then keeps the chip armed and logs "already
driven by ... - pick another key".

## The editor is stricter than the content it produces

The authored-ship lint `scenario_input_overlaps`
(`crates/nova_authoring/src/lint_walk.rs:415`) checks player bindings against the
flight rig's reserved sources ONLY. It does not compare sections to each other.

So two authored sections sharing a key already pass `content lint` today. The
editor invented a restriction the rest of the pipeline does not have. Removing it
makes the editor agree with the format, and needs NO lint change.

That matters more now ships are a content kind: an editor export must be
authorable content, and today the editor refuses to build something a scenario
may legally carry.

## Work

- drop the section-vs-section arm of `binding_conflict`, keep the flight-rig arm
- `rebind_refuses_a_key_another_section_already_holds`
  (`keybind.rs:451`) pins the behaviour being removed - invert it into a test
  that two sections CAN hold one source, and keep a test that the flight rig's
  keys are still refused
- check the keybind UI does not assume one section per source anywhere else -
  the chip list and any glyph lookup

Check the git history or task records for why the section rule was added before
deleting it. If a real reason turns up, report it rather than working around it.

## Definition of done

- two sections can be given the same key in the editor, and both respond in Play
- a flight-rig key is still refused, with the same message
- the inverted test passes and the flight-rig test still passes
