# Burn push ease traces

Headless traces from a disposable test harness (not kept): `MinimalPlugins`,
`ChaseCameraPlugin`, `update_burn_push` in `FixedUpdate`, `update_camera_rig`
in `Update`, Normal mode rig (20.6 u). `MainDriveCommanded` is true from
t = 0.5 s to t = 2.0 s. Columns: commanded, `BurnPush.engaged`,
`ChaseCamera.offset.z` (target) and the smoothed camera z.

- `before-*`: PR #84 at e21b46711, binary push.
- `after-*`: extension spooled at `FlightSettings` rates (6/s up, 10/s down).
- File names give the fixed rate and the render rate.

Fixed 64 Hz, render 60 Hz:

| | before | after |
|---|---|---|
| largest one-frame target step | 3.09 u | 0.45 u |
| camera move on the engage frame | 0.61 u | 0.05 u |
| largest one-frame camera move | 0.61 u | 0.21 u |

## Rendered comparison

A disposable example (not kept) loaded the player skiff or industrial carrier
fixture alone. It ran at a fixed 1/60 s virtual step under lavapipe at
960x540, held `main_drive` from frame 0 to frame 120, and captured every third
frame. Three builds: master camera (ab6fbcb33, heat-scaled push), PR head
(e21b46711, binary push) and after (extension eased).

- `render-skiff-hull-width.csv`: rendered skiff width in px.
- `render-carrier-tower-top.csv`: carrier tower top edge y in px (the hull
  fills the frame width).
- `cmp-*.jpg`: frames around engage and release, one row per build. The
  version label in each frame is the checkout at build time, not the variant.

| | master | PR head | after |
|---|---|---|---|
| skiff width, rest to full push | 197 -> 174 | 197 -> 171 | 197 -> 171 |
| skiff largest move per 3 frames | 5 px | 11 px | 5 px |
| carrier tower top, rest to full push | 122 -> 126 | 122 -> 138 | 122 -> 138 |
| carrier largest move per 3 frames | 1 px | 6 px | 4 px |

At full push the carrier camera sits over the stern: aft plating fills the
lower third of the frame. The PR head frames show it too.
