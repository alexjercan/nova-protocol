# Keep interactive frozen screens under mouse control

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: v0.11.0,fix,ui,input

## Goal

Keep the system pointer visible and ungrabbed while NOVA OS or an arena result
screen owns mouse interaction. A later flight cursor reconciler must not reclaim
it after the one-time state transition hook.

## Done when

- NOVA OS continuously owns a visible, ungrabbed cursor while open.
- The WFC arena result screen continuously owns a visible, ungrabbed cursor.
- Resuming flight still restores the existing player cursor policy.
- Regression tests cover cursor reclamation after another system locks it.
