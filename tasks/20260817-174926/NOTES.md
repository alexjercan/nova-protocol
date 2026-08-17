# Notes

## Lobby slice

- Owner playtest found action buttons below the seed input frame. Row labels and
  action wrappers now use matching top offsets; `/tmp/wfc-lobby-aligned.png`
  is the accepted rendered alignment.
- Added a shared single-line `nova_ui` text field with per-edit value updates,
  caret navigation, commit, cancel/restore, caller-owned validation, and live
  skin/error paint.
- A normal `wfc_arena` run now resolves the CLI/default draft, then opens a
  two-side lobby. Driven runs still bypass it.
- Style is side-owned. `--style` initializes both sides and the last explicit
  ship style wins for its side.
- Every displayed seed is exact. Initial automatic seeds and REROLL results pass
  the armament floor; explicit CLI seeds remain exact even when weak.
- Lobby mutations retain invalid text until corrected. Start is disabled while
  any seed is not a `u64`.
- The 1920x1080 rendered proof is `/tmp/wfc-lobby-1920.png`. The example's debug
  startup forces that capture resolution; a 1280x720 Xvfb capture crops the
  1920px window and is not valid layout evidence.

## Match loop and rebinding

- Escape keeps NOVA OS app -> terminal -> arena pause precedence. The arena
  pause offers Resume, exact-roster Restart, Return to Lobby, and Quit.
- A result freezes the match, then reports dynamic ammunition labels, team
  damage and remaining structure, and one outcome row per configured ship.
- The selected bindable section in the NOVA OS ship app can replace its input
  with `B`. Sections can share inputs; reserved flight controls are refused. Arena overrides
  survive exact restart and return to lobby; changing a ship seed clears that
  slot's overrides.
- Deadlocks use two example-level breakers. Each ship outside a 20 km sphere
  gets a visible 30-second disqualification countdown; re-entry resets it and
  expiry neutralizes its live controller sections through health damage.
- Global activity is ammunition fired, damage, or non-zero fighter thrust.
  After all ships spawn, 180 seconds without global activity is a stalemate.
  Remaining team structure as a percentage of starting structure decides the
  advantage; exact equality is a draw.
- Rendered pause proof is `/tmp/wfc-pause.png`; rendered return-to-lobby proof
  is `/tmp/wfc-return.png`. The final operational-victory board proof is
  `/tmp/wfc-result-final.png`.
