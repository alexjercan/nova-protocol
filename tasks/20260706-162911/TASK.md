# Refactor integrity plugin: graph via relations, split glue systems

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.4.0,refactor


From the TODO sweep (task 20260525-132954). The integrity plugin carries an
IntegrityGraph component that the author would rather express as Bevy relations, and
several systems are glue that should move to a glue.rs so integrity stays focused.

Source TODOs (crates/nova_gameplay/src/integrity/plugin.rs):
- IntegrityGraph component -> use relations instead
- move glue systems out to glue.rs (x2)

Note: the generic blast/impact-damage systems in the same file are tracked separately by
task 20260706-151804 (promote to bevy_common_systems).
