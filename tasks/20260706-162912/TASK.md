# OnDestroyed event fires inconsistently

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.4.0, bug

From the TODO sweep (task 20260525-132954). A FIXME notes that an event (in the
integrity/destruction path) is not fired consistently. Investigate why and make it
reliable.

Source: crates/nova_gameplay/src/integrity/plugin.rs (FIXME near the destruction event).
