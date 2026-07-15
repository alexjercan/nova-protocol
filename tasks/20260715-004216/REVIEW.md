# Review: devlog5-radar-stance-slots composite (stdlib PNG codec)

- TASK: 20260715-004216
- BRANCH: task/devlog5-stance-composite (one commit 91f860cc on master)

## Round 1

- VERDICT: APPROVE

Checks run (in the worktree):
- `python3 scripts/gen-web-screenshots.py --self-test`: OK (round-trips synthetic
  9x7 images through decode for all five row filters x {RGB, RGBA}, resize_box,
  and both the no-letterbox and letterbox compose paths).
- `python3 -m py_compile`: clean.
- Full run: builds `devlog5-radar-stance-slots.png` at 1920x1080 RGBA, exits 0,
  21 shots pending (the un-captured figures + the 3 split-out thumbnails), 0
  failed.
- Determinism: rebuilding the composite yields a byte-identical file (same
  sha256, no git diff) - safe to commit as content.

Independent verification (implementer and reviewer share a session, so the
load-bearing logic was re-derived, not read):

1. Filter reconstruction re-derived against the PNG spec (9.2), not just trusted
   via the round-trip (a predictor bug shared by the test's forward filter and
   decode's reverse filter would cancel in a round-trip):
   - Sub: `Recon = Filt + Recon(a)` -> code `line[i] + row[i-ch]`. Correct.
   - Up: `Recon = Filt + Recon(b)` -> `line[i] + prev[i]`. Correct.
   - Average: `Recon = Filt + floor((a+b)/2)` -> `line[i] + ((a + prev[i]) >> 1)`
     (>> on non-negative ints is floor). Correct.
   - Paeth: `p=a+b-c; pa=|p-a|,pb=|p-b|,pc=|p-c|; Pr = a if pa<=pb and pa<=pc
     else b if pb<=pc else c`. Code matches the spec verbatim. Correct.
   The self-test's forward filters are written independently (subtract the
   predictor computed from original neighbours), so the round-trip is a real
   check on top of the spec re-derivation.

2. Compose/letterbox offset math re-derived for the real case (1920x1080 sources
   into a 1920x1080 frame): half_w=960, scale=min(960/1920, 1080/1080)=0.5,
   draw=960x540, x_offset=tile*960 (no horizontal margin, so the halves meet at
   the centre seam), y_offset=(1080-540)/2=270. Verified against the actual
   output by decoding it back: RGB is pure black for rows 0..269 and 810..1079,
   content in 270..809 - exactly y_offset=270, draw_h=540. Bounds (810<=1080,
   1920<=1920) hold; the two halves do not overlap.

Design: the stdlib-codec choice is right for this repo (no Pillow, matching
`gen-placeholder-sounds.py`), and the aspect-preserving contain-fit avoids the
2:1 distortion a naive scale-to-half-width would give; the black margins blend
into the space-black frames. The `COMPOSITES` table mirrors `ALIASES` (staged
capture wins), and removing the shot from `FIGURES` avoids a double pending
report. Multi-IDAT concatenation, ancillary-chunk skipping, and the
interlace/bit-depth/colour-type guards are all handled. The eyeballed result
reads as intended (white NAV lock left, red combat lock right), matching the
post's caption.

- [x] R1.1 (NIT) scripts/gen-web-screenshots.py:~200 (`decode_png`) - a corrupt
  PNG with a bogus IDAT length or truncated stream raises `zlib.error` (or an
  IndexError), which `build_composites` does not catch (it only wraps
  `compose_side_by_side` in `except ValueError`), so a malformed source figure
  would crash the run rather than being reported like a mis-sized shot. The
  sources here are our own committed valid PNGs, so this is theoretical; if
  hardened, wrap the inflate/parse in `decode_png` to raise `ValueError`, keeping
  the "report, don't crash" contract the earlier `png_dimensions` fix
  established. Left to implementer discretion.
  - Response: FIXED. `decode_png` now guards the IHDR unpack (`len(body) < 13`
    -> ValueError) and wraps `zlib.decompress` to re-raise `zlib.error` as
    `ValueError`, so `build_composites`'s existing `except ValueError` reports a
    corrupt source instead of crashing. Verified: a valid-signature PNG with a
    garbage IDAT now raises `ValueError`; self-test still passes and the
    composite hash is unchanged.
</content>
