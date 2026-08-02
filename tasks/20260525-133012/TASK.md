# Verify marker component for post-processing camera is wired

- PRIORITY: 0
- TAGS: wontdo, chore
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

Already done historically, verify it's correctly wired. Legacy #132.

CLOSED (wontdo, 20260714): verified wired. `PostProcessingDefaultPlugin` is added
in `crates/nova_gameplay/src/plugin.rs:61`, and `PostProcessingCamera` is applied
in `nova_editor/src/lib.rs:418` and `hud/target_inset.rs:456`. Nothing to do.
