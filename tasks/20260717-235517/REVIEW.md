# Review: Thruster exhaust square shape

- TASK: 20260717-235517
- BRANCH: square-exhaust-shape

## Round 1

- VERDICT: APPROVE

Independent out-of-context review + a hand-verified winding check.

Confirmed correct:
- R1.1 Triangle winding matches `new_cone` (sides `(p00,p10,p11)+(p00,p11,p01)`
  with `c0->c1` CCW like `dir0->dir1`; base cap `(ZERO,c1,c0)` = cone's
  `(base_center,p1,p0)`), so StandardMaterial back-face culling does NOT hide the
  flame. This was the highest-risk claim - it holds.
- R1.2 Shader is shape-agnostic: `max_r = radius`, square corners at
  `radius*sqrt(2)` fade to 0 elongation (intended). No degenerate tris / NaN.
- R1.3 Mesh: base y=0, tip y=1, all 4 sides, `with_scale(r,h,r)` -> half-side r.
  Test pins the square corner a cone can't produce.
- R1.4 serde: `geometry` field + enum `Serialize/Deserialize/Default(Cone)`;
  RON `shape: (geometry: Square)` round-trips; the one struct literal
  (torpedo_section) updated.

Open NITs (left to discretion, not blocking):
- [ ] R1.5 (NIT) `ThrusterExhaustConfig.geometry` under `ThrusterExhaust.shape`
  reads `shape.geometry` - mildly redundant naming. Cosmetic.
- [ ] R1.6 (NIT) `exhaust_mesh` hardcodes subdivisions `new_cone(32,4)` /
  `square_exhaust_builder(4)` - pre-existing magic numbers, could be consts.
- [ ] R1.7 (NIT) [visual] under thrust the square flame elongates edge-centers but not
  corners, tapering to a pillow/`+` cross-section - eyeball in-game; fine for a
  glow. Not a code defect.

No BLOCKER/MAJOR. APPROVE.
