# Notes

- `wfc_arena` now treats NOVA OS and its result overlay as continuous owners of
  virtual time, Avian physics time, and the pointer.
- The ownership guard runs in `PostUpdate`. It re-pauses both clocks if another
  transition or reconciler changes them while the screen remains active.
- Closing NOVA OS leaves the guard, so the existing `OnExit(NovaOs)` path owns
  the one unpause transition.
