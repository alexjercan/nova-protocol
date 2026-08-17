# Notes

- Frozen screens already released the cursor once on state entry, but a later
  flight cursor reconciler could lock it again.
- NOVA OS now reasserts visible, ungrabbed cursor ownership in `PostUpdate` for
  every frozen frame.
- WFC arena directly reasserts cursor ownership in `PostUpdate` while NOVA OS
  is open. Its example-local result does not use `PauseStates`, so the same
  guard remains active while a result is finishing or shown.
- Existing pointer forwarding through the NOVA OS CRT remains the click path.
