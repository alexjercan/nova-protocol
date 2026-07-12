# Turret: manual free-aim while holding CTRL

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.5.0,feature

## Goal

Allow moving the turret manually in manual mode by holding CTRL. While CTRL
is held the turret should ignore the current target lock and just follow the
mouse cursor (free aim), instead of sticking to / snapping back to the lock.

Currently pressing CTRL keeps the turret glued to the lock. The desired
behavior: hold CTRL -> turret detaches from lock and tracks the mouse; release
CTRL -> normal locked/manual behavior resumes.

## Notes

- Only applies in manual mode.
- Needs step breakdown via /plan before /work.
