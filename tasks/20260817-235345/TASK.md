# Freeze WFC arena while NOVA OS is open

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: v0.11.0,fix,example,nova-os

## Goal

Make NOVA OS own a real match freeze in `wfc_arena`, not only the cursor.

## Done when

- WFC arena continuously pauses virtual and physics clocks while NOVA OS is open.
- Closing NOVA OS restores normal clock behavior through the existing state exit.
- The result screen remains frozen.
- A WFC-specific regression test proves both clocks are paused.
