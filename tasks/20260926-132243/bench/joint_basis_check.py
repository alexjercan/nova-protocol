"""Compare the settled relative rotation of a docked pair with the capture
pose (intended) and with avian 0.7's FromGlobal basis (basis * rot^-1)."""
import json, math, sys
def mul(a, b):
    ax, ay, az, aw = a; bx, by, bz, bw = b
    return (aw*bx+ax*bw+ay*bz-az*by, aw*by-ax*bz+ay*bw+az*bx, aw*bz+ax*by-ay*bx+az*bw, aw*bw-ax*bx-ay*by-az*bz)
def norm(q):
    n = math.sqrt(sum(c*c for c in q)); return tuple(c/n for c in q)
def inv(q): return (-q[0], -q[1], -q[2], q[3])
def ang(a, b):
    d = abs(sum(x*y for x, y in zip(a, b)))
    return math.degrees(2*math.acos(min(1.0, d)))
path, cap = sys.argv[1], int(sys.argv[2])
late = [int(t) for t in sys.argv[3:]]
snaps = {}
for l in open(path):
    e = json.loads(l)
    if e.get("event") == "channel_in" and e.get("raw"):
        snaps[e["tick"]] = {s["id"]: norm(s["rotation"]) for s in e["raw"]["ships"]}
r1, r2 = snaps[cap]["player"], snaps[cap]["tender"]
intended = mul(inv(r2), r1)           # rot2^-1 rot1 held at capture
buggy = mul(r1, inv(r2))              # local basis2 = basis * rot2^-1, basis = rot1
print(f"capture tick {cap}: player rot {r1} tender rot {r2}")
print(f"  intended vs avian target differ by {ang(intended, buggy):.2f} deg")
for t in late:
    if t not in snaps: continue
    a, b = snaps[t]["player"], snaps[t]["tender"]
    rel = mul(inv(b), a)
    print(f"tick {t}: rel vs intended {ang(rel, intended):7.2f} deg, rel vs avian target {ang(rel, buggy):6.2f} deg")
