# Audit and finalize nova_core crate as thin wiring layer

- STATUS: OPEN
- PRIORITY: 100
- TAGS: v0.3.1,refactor,crates


nova_core should only assemble plugins from the other crates into the runnable game. Verify it contains no gameplay logic; move anything substantive into nova_gameplay or a dedicated crate. [new]
